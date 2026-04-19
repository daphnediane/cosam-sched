/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity lookup system: [`EntityMatcher`], [`EntityCreatable`], and the
//! multi-token lookup algorithm.
//!
//! # Match priority
//!
//! [`MatchPriority`] is a `u8` score (0 = no match, 255 = exact).  Use the
//! named constants in [`match_priority`] and the helper [`string_match_priority`]
//! when implementing [`EntityMatcher::match_entity`].
//!
//! # Lookup algorithm
//!
//! [`lookup`] and [`lookup_or_create`] split the query at commas/semicolons,
//! try each token as a UUID fast-path or entity scan, and accumulate results.
//! Cardinality constraints (`Single` / `Optional` / `List`) are enforced
//! throughout the loop and again after any deferred creation.
//!
//! The caller expresses cardinality via [`FieldType`]: the variant
//! (`Single` / `Optional` / `List`) carries the cardinality, and the inner
//! [`FieldTypeItem::EntityIdentifier`] carries the expected entity type name.

use crate::entity::{EntityId, EntityType, RuntimeEntityId};
use crate::schedule::Schedule;
use crate::value::{FieldCardinality, FieldType, FieldTypeItem, FieldValue, FieldValueItem};

// ── MatchPriority ─────────────────────────────────────────────────────────────

/// Numeric match score returned by [`EntityMatcher::match_entity`].
///
/// Higher values indicate stronger matches.  Use the constants in
/// [`match_priority`] rather than raw integers.
pub type MatchPriority = u8;

/// Named constants for the standard match tiers.
pub mod match_priority {
    use super::MatchPriority;

    /// No match at all; any value ≥ [`MIN_MATCH`] is a match.
    pub const NO_MATCH: MatchPriority = 0;
    /// Minimum acceptable match level.
    pub const MIN_MATCH: MatchPriority = 1;
    /// Weak match: query appears as a substring within the value.
    pub const WEAK_MATCH: MatchPriority = 50;
    /// Average match: query appears at a word boundary in the value.
    pub const AVERAGE_MATCH: MatchPriority = 100;
    /// Strong match: value starts with the query (case-insensitive).
    pub const STRONG_MATCH: MatchPriority = 200;
    /// Exact match: value equals the query (case-insensitive).
    pub const EXACT_MATCH: MatchPriority = 255;
}

/// Standard tiered string matching using [`match_priority`] levels.
///
/// Returns `None` when `query` is empty or does not appear in `value` at all.
/// Implements the tiers in descending order:
/// - [`EXACT_MATCH`]   — case-insensitive equality
/// - [`STRONG_MATCH`]  — value starts with query
/// - [`AVERAGE_MATCH`] — query appears at a word boundary
/// - [`WEAK_MATCH`]    — query is a substring of value
///
/// [`EXACT_MATCH`]: match_priority::EXACT_MATCH
/// [`STRONG_MATCH`]: match_priority::STRONG_MATCH
/// [`AVERAGE_MATCH`]: match_priority::AVERAGE_MATCH
/// [`WEAK_MATCH`]: match_priority::WEAK_MATCH
#[must_use]
pub fn string_match_priority(query: &str, value: &str) -> Option<MatchPriority> {
    if query.is_empty() {
        return None;
    }
    let q = query.to_lowercase();
    let v = value.to_lowercase();
    if v == q {
        Some(match_priority::EXACT_MATCH)
    } else if v.starts_with(q.as_str()) {
        Some(match_priority::STRONG_MATCH)
    } else if word_boundary_match(&v, &q) {
        Some(match_priority::AVERAGE_MATCH)
    } else if v.contains(q.as_str()) {
        Some(match_priority::WEAK_MATCH)
    } else {
        None
    }
}

/// Returns true if `needle` appears in `haystack` starting at a word boundary.
///
/// A word boundary is the start of the string or any position immediately
/// following a non-alphanumeric character.  Both strings must already be
/// lowercase.
fn word_boundary_match(haystack: &str, needle: &str) -> bool {
    let mut from = 0usize;
    while let Some(rel) = haystack[from..].find(needle) {
        let abs = from + rel;
        let at_boundary = abs == 0 || !haystack.as_bytes()[abs - 1].is_ascii_alphanumeric();
        if at_boundary {
            return true;
        }
        from = abs + 1;
        if from >= haystack.len() {
            break;
        }
    }
    false
}

// ── EntityMatcher ─────────────────────────────────────────────────────────────

/// Entity types implement this to define their own holistic match logic.
///
/// Unlike the old per-field `IndexableField` approach, the entity type
/// controls how all its fields are weighted and combined.  Use
/// [`string_match_priority`] as a building block for individual fields.
pub trait EntityMatcher: EntityType {
    /// Return the match quality for `query` against `data`, or `None` if there
    /// is no match.  Values ≥ [`match_priority::MIN_MATCH`] are considered a
    /// match; `None` and `0` are equivalent non-matches.
    fn match_entity(query: &str, data: &Self::InternalData) -> Option<MatchPriority>;
}

// ── EntityCreatable ───────────────────────────────────────────────────────────

/// Whether and how a string can be used to create a new entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanCreate {
    /// The string cannot be used to create this entity.
    No,
    /// The full remaining query string (before any separator) can create the entity.
    FromFull,
    /// Only the partial token (before the first separator) can create the entity.
    FromPartial,
}

/// Entity types that support find-or-create implement this on top of
/// [`EntityMatcher`].
pub trait EntityCreatable: EntityMatcher {
    /// Decide whether `full` (the complete remaining query) or `partial` (the
    /// token before the first separator) can be used to create a new entity.
    ///
    /// `partial == full` when there is no separator in the remaining query.
    fn can_create(full: &str, partial: &str) -> CanCreate;

    /// Create a new entity from `s` and return its ID.
    ///
    /// `s` is whichever string [`can_create`] indicated was creatable.
    ///
    /// [`can_create`]: EntityCreatable::can_create
    fn create_from_string(schedule: &mut Schedule, s: &str) -> Result<EntityId<Self>, LookupError>;
}

// ── LookupError ───────────────────────────────────────────────────────────────

/// Errors returned by [`lookup`] and [`lookup_or_create`].
#[derive(Debug, thiserror::Error)]
pub enum LookupError {
    /// Multiple entities match the query with the same priority.
    #[error("ambiguous match for '{query}'")]
    AmbiguousMatch { query: String },

    /// No entity matches and creation is not enabled.
    #[error("not found: '{query}'")]
    NotFound { query: String },

    /// The UUID prefix names a different entity type than expected.
    #[error("wrong entity type: expected '{expected}', got '{got}'")]
    WrongEntityType { expected: &'static str, got: String },

    /// More matches were found than the cardinality allows.
    #[error("too many results: found {found}")]
    TooMany { found: usize },

    /// Find-or-create was attempted but the entity type refuses creation.
    #[error("cannot create entity from '{query}'")]
    CannotCreate { query: String },

    /// The UUID string is syntactically invalid or refers to a nil UUID.
    #[error("invalid UUID: '{s}'")]
    InvalidUuid { s: String },

    /// The [`FieldType`] passed to [`lookup`] is not an `EntityIdentifier` variant.
    #[error("field type is not an EntityIdentifier variant")]
    WrongFieldType,

    /// Entity creation failed.
    #[error("create failed: {message}")]
    CreateFailed { message: String },
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Returns `true` if `s` has the character pattern of a standard UUID
/// (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, 36 characters).
fn is_uuid_format(s: &str) -> bool {
    s.len() == 36
        && s.as_bytes().get(8) == Some(&b'-')
        && s.as_bytes().get(13) == Some(&b'-')
        && s.as_bytes().get(18) == Some(&b'-')
        && s.as_bytes().get(23) == Some(&b'-')
}

/// Returns `true` if `s` looks like a bare UUID or a `"type_name:uuid"` tagged UUID.
fn looks_like_uuid_or_tagged(s: &str) -> bool {
    if is_uuid_format(s) {
        return true;
    }
    if let Some(colon) = s.find(':') {
        return is_uuid_format(&s[colon + 1..]);
    }
    false
}

/// Parse `s` as a bare UUID or `"type_name:uuid"` tagged UUID, verify it
/// exists in the schedule as an entity of type `E`, and return its ID.
fn parse_typed_uuid<E: EntityType>(
    s: &str,
    schedule: &Schedule,
) -> Result<EntityId<E>, LookupError> {
    let uuid_str: &str = if is_uuid_format(s) {
        s
    } else if let Some(colon) = s.find(':') {
        let type_name = &s[..colon];
        let rest = &s[colon + 1..];
        if type_name != E::TYPE_NAME {
            return Err(LookupError::WrongEntityType {
                expected: E::TYPE_NAME,
                got: type_name.to_string(),
            });
        }
        rest
    } else {
        return Err(LookupError::InvalidUuid { s: s.to_string() });
    };

    let uuid = uuid::Uuid::parse_str(uuid_str)
        .map_err(|_| LookupError::InvalidUuid { s: s.to_string() })?;
    let id =
        EntityId::<E>::new(uuid).ok_or_else(|| LookupError::InvalidUuid { s: s.to_string() })?;

    if schedule.get_internal::<E>(id).is_none() {
        return Err(LookupError::NotFound {
            query: s.to_string(),
        });
    }

    Ok(id)
}

/// Split `s` at the first `','` or `';'`, returning `(before, after)` both
/// trimmed.  When no separator is present returns `(s.trim(), "")`.
fn split_first_sep(s: &str) -> (&str, &str) {
    match s.find([',', ';']) {
        Some(i) => (s[..i].trim(), s[i + 1..].trim()),
        None => (s.trim(), ""),
    }
}

/// Scan all entities of type `E` and return matches scored by
/// [`EntityMatcher::match_entity`].
fn scan_entities<E: EntityMatcher>(
    schedule: &Schedule,
    query: &str,
) -> Vec<(EntityId<E>, MatchPriority)> {
    schedule
        .iter_entities::<E>()
        .filter_map(|(id, data)| E::match_entity(query, data).map(|p| (id, p)))
        .collect()
}

/// Return the highest [`MatchPriority`] in `hits`, or `None` if empty.
fn best_priority<E: EntityType>(hits: &[(EntityId<E>, MatchPriority)]) -> Option<MatchPriority> {
    hits.iter().map(|(_, p)| *p).max()
}

/// Keep only the entries in `hits` that share the highest priority.
fn filter_to_best<E: EntityType>(hits: Vec<(EntityId<E>, MatchPriority)>) -> Vec<EntityId<E>> {
    let Some(best) = best_priority(&hits) else {
        return vec![];
    };
    hits.into_iter()
        .filter_map(|(id, p)| if p == best { Some(id) } else { None })
        .collect()
}

/// Extract the `&'static str` entity type name from an `EntityIdentifier`
/// [`FieldType`].  Returns [`LookupError::WrongFieldType`] for any other variant.
fn extract_entity_type_name(field_type: FieldType) -> Result<&'static str, LookupError> {
    match field_type.1 {
        FieldTypeItem::EntityIdentifier(n) => Ok(n),
        _ => Err(LookupError::WrongFieldType),
    }
}

/// Returns `true` for `Single` and `Optional` (both limit result count).
fn is_limited(field_type: FieldType) -> bool {
    matches!(
        field_type.0,
        FieldCardinality::Single | FieldCardinality::Optional
    )
}

// ── Core loop ─────────────────────────────────────────────────────────────────

/// Run the multi-token lookup loop.
///
/// - `results` accumulates matched entity IDs.
/// - `create_queue` accumulates strings to be passed to `EntityCreatable::create_from_string`
///   after the loop.  Always empty when `allow_create` is `false`.
/// - `try_create` is called when no match is found and `allow_create` is `true`.
fn run_lookup_loop<E: EntityMatcher>(
    schedule: &Schedule,
    query: &str,
    field_type: FieldType,
    results: &mut Vec<EntityId<E>>,
    create_queue: &mut Vec<String>,
    allow_create: bool,
    try_create: &dyn Fn(&str, &str) -> CanCreate,
) -> Result<(), LookupError> {
    let mut match_string = query.trim();

    loop {
        // ── Early cardinality checks ──────────────────────────────────────────
        if results.len() > 1 && is_limited(field_type) {
            return Err(LookupError::TooMany {
                found: results.len(),
            });
        }

        // Empty → done.
        if match_string.is_empty() {
            break;
        }

        let (partial, rest) = split_first_sep(match_string);

        // After detecting there is more input: if we already have 1 result
        // and cardinality is limited, adding more would exceed the limit.
        if results.len() == 1 && is_limited(field_type) {
            return Err(LookupError::TooMany {
                found: results.len() + create_queue.len() + 1,
            });
        }

        // ── UUID fast-path ────────────────────────────────────────────────────
        if looks_like_uuid_or_tagged(partial) {
            let id = parse_typed_uuid::<E>(partial, schedule)?;
            results.push(id);
            match_string = rest;
            continue;
        }

        // ── Entity scan ───────────────────────────────────────────────────────
        // Try the full remaining string and the partial token (if different).
        // Prefer the full string for ties (>= comparison).
        let full_hits = scan_entities::<E>(schedule, match_string);
        let partial_hits: Vec<(EntityId<E>, MatchPriority)> = if partial != match_string {
            scan_entities::<E>(schedule, partial)
        } else {
            vec![]
        };

        let full_best = best_priority(&full_hits);
        let partial_best = best_priority(&partial_hits);

        let (best_set, from_full) = if full_best >= partial_best {
            (filter_to_best(full_hits), true)
        } else {
            (filter_to_best(partial_hits), false)
        };

        match best_set.len() {
            0 => {
                if !allow_create {
                    return Err(LookupError::NotFound {
                        query: match_string.to_string(),
                    });
                }
                match try_create(match_string, partial) {
                    CanCreate::No => {
                        return Err(LookupError::CannotCreate {
                            query: match_string.to_string(),
                        });
                    }
                    CanCreate::FromFull => {
                        create_queue.push(match_string.to_string());
                        match_string = "";
                    }
                    CanCreate::FromPartial => {
                        create_queue.push(partial.to_string());
                        match_string = rest;
                    }
                }
            }
            1 => {
                results.push(best_set[0]);
                match_string = if from_full { "" } else { rest };
            }
            _ => {
                let q = if from_full { match_string } else { partial };
                return Err(LookupError::AmbiguousMatch {
                    query: q.to_string(),
                });
            }
        }
    }

    Ok(())
}

// ── Result packing ────────────────────────────────────────────────────────────

/// Combine found and newly-created IDs, enforce final cardinality, and pack
/// into a [`FieldValue`].
fn check_and_pack<E: EntityType>(
    mut results: Vec<EntityId<E>>,
    create_results: Vec<EntityId<E>>,
    field_type: FieldType,
    original_query: &str,
) -> Result<FieldValue, LookupError> {
    results.extend(create_results);
    let total = results.len();

    match field_type.0 {
        FieldCardinality::Single => {
            if total == 0 {
                return Err(LookupError::NotFound {
                    query: original_query.to_string(),
                });
            }
            if total > 1 {
                return Err(LookupError::TooMany { found: total });
            }
            Ok(FieldValue::Single(FieldValueItem::EntityIdentifier(
                RuntimeEntityId::from_typed(results[0]),
            )))
        }
        FieldCardinality::Optional => {
            if total > 1 {
                return Err(LookupError::TooMany { found: total });
            }
            if total == 0 {
                Ok(FieldValue::List(vec![]))
            } else {
                Ok(FieldValue::Single(FieldValueItem::EntityIdentifier(
                    RuntimeEntityId::from_typed(results[0]),
                )))
            }
        }
        FieldCardinality::List => Ok(FieldValue::List(
            results
                .into_iter()
                .map(|id| FieldValueItem::EntityIdentifier(RuntimeEntityId::from_typed(id)))
                .collect(),
        )),
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Find one or more existing entities matching `query`.
///
/// `field_type` must be one of:
/// - `FieldType::Single(FieldTypeItem::EntityIdentifier(E::TYPE_NAME))`
/// - `FieldType::Optional(FieldTypeItem::EntityIdentifier(E::TYPE_NAME))`
/// - `FieldType::List(FieldTypeItem::EntityIdentifier(E::TYPE_NAME))`
///
/// The variant controls cardinality; `Single` and `Optional` error when more
/// than one match is found.
///
/// `query` may contain comma- or semicolon-separated tokens; each is resolved
/// independently.  Bare UUIDs and `"type_name:uuid"` tagged UUIDs bypass the
/// entity scan.
///
/// # Errors
///
/// Returns [`LookupError`] when:
/// - No entity matches (`NotFound`)
/// - Multiple entities match at the same priority (`AmbiguousMatch`)
/// - More results than the cardinality allows (`TooMany`)
/// - The `field_type` is not an `EntityIdentifier` (`WrongFieldType`)
pub fn lookup<E: EntityMatcher>(
    schedule: &Schedule,
    query: &str,
    field_type: FieldType,
) -> Result<FieldValue, LookupError> {
    let type_name = extract_entity_type_name(field_type)?;
    if type_name != E::TYPE_NAME {
        return Err(LookupError::WrongEntityType {
            expected: E::TYPE_NAME,
            got: type_name.to_string(),
        });
    }

    let mut results: Vec<EntityId<E>> = Vec::new();
    let mut create_queue: Vec<String> = Vec::new();

    run_lookup_loop::<E>(
        schedule,
        query,
        field_type,
        &mut results,
        &mut create_queue,
        false,
        &|_, _| CanCreate::No,
    )?;

    debug_assert!(
        create_queue.is_empty(),
        "find-only should never enqueue creates"
    );

    check_and_pack(results, vec![], field_type, query)
}

/// Find or create one or more entities matching `query`.
///
/// Same as [`lookup`] but when no match is found,
/// [`EntityCreatable::can_create`] is consulted and, if creation is possible,
/// the string is deferred until after the loop.  All deferred creates are
/// applied in order via [`EntityCreatable::create_from_string`] after the
/// final cardinality check.
///
/// `field_type` must be an `EntityIdentifier` variant matching `E::TYPE_NAME`.
///
/// # Errors
///
/// All [`lookup`] errors plus:
/// - [`LookupError::CannotCreate`] when the entity type refuses creation
/// - [`LookupError::CreateFailed`] when creation itself fails
pub fn lookup_or_create<E: EntityCreatable>(
    schedule: &mut Schedule,
    query: &str,
    field_type: FieldType,
) -> Result<FieldValue, LookupError> {
    let type_name = extract_entity_type_name(field_type)?;
    if type_name != E::TYPE_NAME {
        return Err(LookupError::WrongEntityType {
            expected: E::TYPE_NAME,
            got: type_name.to_string(),
        });
    }

    let mut results: Vec<EntityId<E>> = Vec::new();
    let mut create_queue: Vec<String> = Vec::new();

    // The loop only reads from the schedule; the &mut Schedule is temporarily
    // reborrowed as &Schedule for the duration of the call.
    run_lookup_loop::<E>(
        schedule,
        query,
        field_type,
        &mut results,
        &mut create_queue,
        true,
        &|full, partial| E::can_create(full, partial),
    )?;

    // Post-loop cardinality check before committing to deferred creates.
    let pre_create_total = results.len() + create_queue.len();
    match field_type.0 {
        FieldCardinality::Single if pre_create_total == 0 => {
            return Err(LookupError::NotFound {
                query: query.to_string(),
            });
        }
        FieldCardinality::Single if pre_create_total > 1 => {
            return Err(LookupError::TooMany {
                found: pre_create_total,
            });
        }
        FieldCardinality::Optional if pre_create_total > 1 => {
            return Err(LookupError::TooMany {
                found: pre_create_total,
            });
        }
        _ => {}
    }

    // Deferred creation.
    let mut create_results: Vec<EntityId<E>> = Vec::with_capacity(create_queue.len());
    for s in create_queue {
        let id = E::create_from_string(schedule, &s)?;
        create_results.push(id);
    }

    check_and_pack(results, create_results, field_type, query)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── string_match_priority ────────────────────────────────────────────────

    #[test]
    fn test_exact_match() {
        assert_eq!(
            string_match_priority("hello", "hello"),
            Some(match_priority::EXACT_MATCH)
        );
    }

    #[test]
    fn test_exact_match_case_insensitive() {
        assert_eq!(
            string_match_priority("Hello", "HELLO"),
            Some(match_priority::EXACT_MATCH)
        );
    }

    #[test]
    fn test_strong_match_prefix() {
        assert_eq!(
            string_match_priority("hel", "hello world"),
            Some(match_priority::STRONG_MATCH)
        );
    }

    #[test]
    fn test_average_match_word_boundary() {
        // "world" starts after a space → word boundary
        assert_eq!(
            string_match_priority("world", "hello world"),
            Some(match_priority::AVERAGE_MATCH)
        );
    }

    #[test]
    fn test_weak_match_substring() {
        // "ello" is in the middle of a word → not a word boundary
        assert_eq!(
            string_match_priority("ello", "hello world"),
            Some(match_priority::WEAK_MATCH)
        );
    }

    #[test]
    fn test_no_match() {
        assert_eq!(string_match_priority("zzz", "hello world"), None);
    }

    #[test]
    fn test_empty_query_returns_none() {
        assert_eq!(string_match_priority("", "hello"), None);
    }

    // ── split_first_sep ──────────────────────────────────────────────────────

    #[test]
    fn test_split_comma() {
        assert_eq!(split_first_sep("Alice, Bob"), ("Alice", "Bob"));
    }

    #[test]
    fn test_split_semicolon() {
        assert_eq!(split_first_sep("Alice; Bob"), ("Alice", "Bob"));
    }

    #[test]
    fn test_split_no_sep() {
        assert_eq!(split_first_sep("Alice"), ("Alice", ""));
    }

    #[test]
    fn test_split_trims_whitespace() {
        assert_eq!(split_first_sep("  Alice  ,  Bob  "), ("Alice", "Bob"));
    }

    // ── is_uuid_format ───────────────────────────────────────────────────────

    #[test]
    fn test_is_uuid_format_valid() {
        assert!(is_uuid_format("550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn test_is_uuid_format_too_short() {
        assert!(!is_uuid_format("550e8400-e29b-41d4-a716"));
    }

    #[test]
    fn test_is_uuid_format_no_hyphens() {
        assert!(!is_uuid_format("550e8400e29b41d4a716446655440000xx"));
    }

    // ── looks_like_uuid_or_tagged ────────────────────────────────────────────

    #[test]
    fn test_looks_like_bare_uuid() {
        assert!(looks_like_uuid_or_tagged(
            "550e8400-e29b-41d4-a716-446655440000"
        ));
    }

    #[test]
    fn test_looks_like_tagged_uuid() {
        assert!(looks_like_uuid_or_tagged(
            "presenter:550e8400-e29b-41d4-a716-446655440000"
        ));
    }

    #[test]
    fn test_looks_like_plain_name_is_false() {
        assert!(!looks_like_uuid_or_tagged("Alice Smith"));
    }

    // ── word_boundary_match ──────────────────────────────────────────────────

    #[test]
    fn test_word_boundary_at_start() {
        assert!(word_boundary_match("hello world", "hello"));
    }

    #[test]
    fn test_word_boundary_after_space() {
        assert!(word_boundary_match("hello world", "world"));
    }

    #[test]
    fn test_word_boundary_not_mid_word() {
        assert!(!word_boundary_match("hello world", "ello"));
    }
}
