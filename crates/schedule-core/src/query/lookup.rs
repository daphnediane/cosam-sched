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
//! Cardinality constraints ([`FieldCardinality::Single`] / [`FieldCardinality::Optional`]
//! / [`FieldCardinality::List`]) are enforced throughout the loop and again
//! after any deferred creation.
//!
//! Both functions return `Result<Vec<EntityId<E>>, LookupError>`:
//!
//! - `Single`   → exactly one element in the returned vec
//! - `Optional` → zero or one element
//! - `List`     → zero or more elements
//!
//! Convenience helpers [`lookup_single`], [`lookup_list`],
//! [`lookup_or_create_single`], and [`lookup_or_create_list`] specialize the
//! return type for the common cardinalities.

use crate::entity::{EntityId, EntityType};
use crate::schedule::Schedule;
use crate::value::FieldCardinality;

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

/// Which portion of the query a scan consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchConsumed {
    /// The full remaining string (including separators) was used; the loop
    /// stops consuming further tokens.
    Full,
    /// Only the partial token (before the first separator) was used; the
    /// loop continues with whatever follows the separator.
    Partial,
}

/// Whether and how a string can be used to create a new entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanCreate {
    /// The string cannot be used to create this entity.
    No,
    /// The string can be used to create the entity; the [`MatchConsumed`]
    /// value says whether to consume the full remaining query or just the
    /// partial token (before the first separator).
    Yes(MatchConsumed),
}

/// Entity types implement this to define their own holistic match logic.
///
/// Unlike the old per-field `IndexableField` approach, the entity type
/// controls how all its fields are weighted and combined.  Use
/// [`string_match_priority`] as a building block for individual fields.
pub trait EntityMatcher: EntityType {
    /// Decide whether `full` (the complete remaining query) or `partial` (the
    /// token before the first separator) can be used to create a new entity.
    ///
    /// `partial == full` when there is no separator in the remaining query.
    /// The default refuses creation ([`CanCreate::No`]).
    fn can_create(_full: &str, _partial: &str) -> CanCreate {
        CanCreate::No
    }

    /// Return the match quality for `query` against `data`, or `None` if there
    /// is no match.  Values ≥ [`match_priority::MIN_MATCH`] are considered a
    /// match; `None` and `0` are equivalent non-matches.
    fn match_entity(query: &str, data: &Self::InternalData) -> Option<MatchPriority>;
}

// ── EntityScannable ───────────────────────────────────────────────────────────

/// Per-token verdict of [`EntityScannable::scan_entity`].
///
/// The scan is authoritative; the lookup loop never consults
/// [`EntityMatcher::can_create`] after `scan_entity` returns.  When the
/// scan decides creation is refused, it returns
/// `Err(LookupError::NotFound)` directly rather than producing a
/// variant here.
#[derive(Debug)]
pub enum ScanFound<E: EntityType> {
    /// Matched an existing entity.
    Entity(EntityId<E>),
    /// No match; creation is allowed.  For find-or-create lookups the loop
    /// queues the consumed portion of the query for
    /// [`EntityCreatable::create_from_string`].  Non-create lookups treat
    /// this as a terminal `NotFound`.
    CanCreate,
}

/// Outcome of [`EntityScannable::scan_entity`].
///
/// The first field records how much of the query the scan consumed and
/// controls how the loop advances for both variants of [`ScanFound`].
#[derive(Debug)]
pub struct ScanResult<E: EntityType>(pub MatchConsumed, pub ScanFound<E>);

/// Entity types implement this to plug into the lookup loop's entity scan
/// step.  The default implementation performs a linear scan using
/// [`EntityMatcher::match_entity`] and disambiguates between the full
/// remaining string and the partial token the same way the built-in loop
/// always has.  Override for custom matching (e.g., index-backed lookup,
/// `"Group: Alice"` tokens, or entities that produce a creation hint on
/// miss).
///
/// The built-in UUID fast-path still runs before `scan_entity`.
pub trait EntityScannable: EntityMatcher {
    /// Resolve `full` / `partial` against the schedule.
    ///
    /// `full` is the entire remaining query (may contain separators),
    /// `partial` is the substring up to the first `,` / `;`.  When the query
    /// has no separator `partial == full`.
    ///
    /// # Errors
    ///
    /// Implementations typically return [`LookupError::AmbiguousMatch`] when
    /// multiple entities tie at the same match priority, but any
    /// [`LookupError`] is allowed and propagates out of the lookup loop.
    fn scan_entity(
        full: &str,
        partial: &str,
        schedule: &Schedule,
    ) -> Result<ScanResult<Self>, LookupError> {
        let full_hits = scan_entities::<Self>(schedule, full);
        let partial_hits: Vec<(EntityId<Self>, MatchPriority)> = if partial != full {
            scan_entities::<Self>(schedule, partial)
        } else {
            vec![]
        };

        let full_best = best_priority(&full_hits);
        let partial_best = best_priority(&partial_hits);

        // Prefer full on ties (>=).
        let (best_set, consumed) = if full_best >= partial_best {
            (filter_to_best(full_hits), MatchConsumed::Full)
        } else {
            (filter_to_best(partial_hits), MatchConsumed::Partial)
        };

        match best_set.len() {
            0 => match Self::can_create(full, partial) {
                CanCreate::No => Err(LookupError::NotFound {
                    query: full.to_string(),
                }),
                CanCreate::Yes(c) => Ok(ScanResult(c, ScanFound::CanCreate)),
            },
            1 => Ok(ScanResult(consumed, ScanFound::Entity(best_set[0]))),
            _ => {
                let q = match consumed {
                    MatchConsumed::Full => full,
                    MatchConsumed::Partial => partial,
                };
                Err(LookupError::AmbiguousMatch {
                    query: q.to_string(),
                })
            }
        }
    }
}

// ── EntityCreatable ───────────────────────────────────────────────────────────

/// Entity types that support find-or-create implement this on top of
/// [`EntityScannable`].
pub trait EntityCreatable: EntityScannable {
    /// Create a new entity from `s` and return its ID.
    ///
    /// `s` is whichever string [`can_create`] indicated was creatable.
    ///
    /// [`can_create`]: EntityMatcher::can_create
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

    /// The UUID string is syntactically invalid or refers to a nil UUID.
    #[error("invalid UUID: '{s}'")]
    InvalidUuid { s: String },

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
    let non_nil_uuid =
        uuid::NonNilUuid::new(uuid).ok_or_else(|| LookupError::InvalidUuid { s: s.to_string() })?;
    let id = unsafe { EntityId::<E>::new_unchecked(non_nil_uuid) };

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

/// Returns `true` for `Single` and `Optional` (both limit result count to 1).
fn is_limited(cardinality: FieldCardinality) -> bool {
    matches!(
        cardinality,
        FieldCardinality::Single | FieldCardinality::Optional
    )
}

// ── Core loop ─────────────────────────────────────────────────────────────────

/// Run the multi-token lookup loop.
///
/// - `results` accumulates matched entity IDs.
/// - `create_queue` accumulates strings to be passed to
///   [`EntityCreatable::create_from_string`] after the loop.  Always empty
///   when `allow_create` is `false`.
fn run_lookup_loop<E: EntityScannable>(
    schedule: &Schedule,
    query: &str,
    cardinality: FieldCardinality,
    results: &mut Vec<EntityId<E>>,
    create_queue: &mut Vec<String>,
    allow_create: bool,
) -> Result<(), LookupError> {
    let mut match_string = query.trim();

    loop {
        // ── Early cardinality checks ──────────────────────────────────────────
        if results.len() > 1 && is_limited(cardinality) {
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
        if results.len() == 1 && is_limited(cardinality) {
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
        // Dispatch to [`EntityScannable::scan_entity`]; the default handles
        // the linear-scan + full/partial disambiguation, overrides may do
        // something smarter (e.g., index lookups, tagged tokens that return
        // MustCreate with their own `Consumed` hint).
        let ScanResult(consumed, found) = E::scan_entity(match_string, partial, schedule)?;

        match found {
            ScanFound::Entity(id) => {
                results.push(id);
                match_string = match consumed {
                    MatchConsumed::Full => "",
                    MatchConsumed::Partial => rest,
                };
            }
            ScanFound::CanCreate => {
                if !allow_create {
                    return Err(LookupError::NotFound {
                        query: match_string.to_string(),
                    });
                }
                match consumed {
                    MatchConsumed::Full => {
                        create_queue.push(match_string.to_string());
                        match_string = "";
                    }
                    MatchConsumed::Partial => {
                        create_queue.push(partial.to_string());
                        match_string = rest;
                    }
                }
            }
        }
    }

    Ok(())
}

// ── Result packing ────────────────────────────────────────────────────────────

/// Combine found and newly-created IDs and enforce final cardinality.
fn check_final_cardinality<E: EntityType>(
    mut results: Vec<EntityId<E>>,
    create_results: Vec<EntityId<E>>,
    cardinality: FieldCardinality,
    original_query: &str,
) -> Result<Vec<EntityId<E>>, LookupError> {
    results.extend(create_results);
    let total = results.len();

    match cardinality {
        FieldCardinality::Single => {
            if total == 0 {
                return Err(LookupError::NotFound {
                    query: original_query.to_string(),
                });
            }
            if total > 1 {
                return Err(LookupError::TooMany { found: total });
            }
        }
        FieldCardinality::Optional => {
            if total > 1 {
                return Err(LookupError::TooMany { found: total });
            }
        }
        FieldCardinality::List => {}
    }
    Ok(results)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Find one or more existing entities matching `query`.
///
/// `cardinality` controls the allowed result count:
/// - [`FieldCardinality::Single`]   — exactly one match required
/// - [`FieldCardinality::Optional`] — zero or one match
/// - [`FieldCardinality::List`]     — any number of matches
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
pub fn lookup<E: EntityScannable>(
    schedule: &Schedule,
    query: &str,
    cardinality: FieldCardinality,
) -> Result<Vec<EntityId<E>>, LookupError> {
    let mut results: Vec<EntityId<E>> = Vec::new();
    let mut create_queue: Vec<String> = Vec::new();

    run_lookup_loop::<E>(
        schedule,
        query,
        cardinality,
        &mut results,
        &mut create_queue,
        false,
    )?;

    debug_assert!(
        create_queue.is_empty(),
        "find-only should never enqueue creates"
    );

    check_final_cardinality(results, vec![], cardinality, query)
}

/// Find or create one or more entities matching `query`.
///
/// Same as [`lookup`] but when no match is found,
/// [`EntityMatcher::can_create`] is consulted and, if creation is possible,
/// the string is deferred until after the loop.  All deferred creates are
/// applied in order via [`EntityCreatable::create_from_string`] after the
/// final cardinality check.
///
/// # Errors
///
/// All [`lookup`] errors plus:
/// - [`LookupError::CreateFailed`] when creation itself fails
///
/// When an entity type refuses creation for a token the lookup produces
/// [`LookupError::NotFound`] (same as a true no-match); the two cases are
/// not distinguished at this layer.
pub fn lookup_or_create<E: EntityCreatable>(
    schedule: &mut Schedule,
    query: &str,
    cardinality: FieldCardinality,
) -> Result<Vec<EntityId<E>>, LookupError> {
    let mut results: Vec<EntityId<E>> = Vec::new();
    let mut create_queue: Vec<String> = Vec::new();

    run_lookup_loop::<E>(
        schedule,
        query,
        cardinality,
        &mut results,
        &mut create_queue,
        true,
    )?;

    // Post-loop cardinality check before committing to deferred creates.
    let pre_create_total = results.len() + create_queue.len();
    match cardinality {
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

    check_final_cardinality(results, create_results, cardinality, query)
}

// ── Convenience helpers ───────────────────────────────────────────────────────

/// Find exactly one entity matching `query`.
///
/// Shorthand for [`lookup`] with [`FieldCardinality::Single`] that returns the
/// single [`EntityId<E>`] directly instead of a one-element vec.
///
/// # Errors
///
/// Returns [`LookupError::NotFound`] when no match is found and
/// [`LookupError::TooMany`] when more than one entity matches.
pub fn lookup_single<E: EntityScannable>(
    schedule: &Schedule,
    query: &str,
) -> Result<EntityId<E>, LookupError> {
    let mut v = lookup::<E>(schedule, query, FieldCardinality::Single)?;
    debug_assert_eq!(
        v.len(),
        1,
        "Single cardinality must yield exactly one result"
    );
    Ok(v.remove(0))
}

/// Find zero or more entities matching `query`.
///
/// Shorthand for [`lookup`] with [`FieldCardinality::List`].
///
/// # Errors
///
/// See [`lookup`].  `TooMany` cannot occur for `List` cardinality.
pub fn lookup_list<E: EntityScannable>(
    schedule: &Schedule,
    query: &str,
) -> Result<Vec<EntityId<E>>, LookupError> {
    lookup::<E>(schedule, query, FieldCardinality::List)
}

/// Find or create exactly one entity matching `query`.
///
/// Shorthand for [`lookup_or_create`] with [`FieldCardinality::Single`] that
/// returns the single [`EntityId<E>`] directly instead of a one-element vec.
///
/// # Errors
///
/// See [`lookup_or_create`].
pub fn lookup_or_create_single<E: EntityCreatable>(
    schedule: &mut Schedule,
    query: &str,
) -> Result<EntityId<E>, LookupError> {
    let mut v = lookup_or_create::<E>(schedule, query, FieldCardinality::Single)?;
    debug_assert_eq!(
        v.len(),
        1,
        "Single cardinality must yield exactly one result"
    );
    Ok(v.remove(0))
}

/// Find or create zero or more entities matching `query`.
///
/// Shorthand for [`lookup_or_create`] with [`FieldCardinality::List`].
///
/// # Errors
///
/// See [`lookup_or_create`].  `TooMany` cannot occur for `List` cardinality.
pub fn lookup_or_create_list<E: EntityCreatable>(
    schedule: &mut Schedule,
    query: &str,
) -> Result<Vec<EntityId<E>>, LookupError> {
    lookup_or_create::<E>(schedule, query, FieldCardinality::List)
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
