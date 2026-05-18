/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity builders — [`EntityBuildable`] trait and the [`build_entity`] driver
//! that underpins the `define_entity_builder!` macro.
//!
//! The builder pattern layers on top of [`crate::field::set::FieldSet::write_multiple`]:
//! each builder collects a `Vec<FieldUpdate<E>>`
//! via typed `with_*` setters, then [`build_entity`] seeds a fresh entity
//! with [`EntityBuildable::default_data`], applies the batch, runs
//! [`EntityType::validate`], and rolls back via [`Schedule::remove_entity`]
//! on any failure.
//!
//! `EntityBuildable` is a subtrait of [`EntityType`] so that mock entities
//! in unit tests and any future read-only entity kinds are not forced to
//! implement it.

use std::collections::HashSet;

use thiserror::Error;
use uuid::NonNilUuid;

use crate::entity::{EntityId, EntityType, EntityUuid, UuidPreference};
use crate::field::set::{FieldOp, FieldSetError, FieldUpdate};
use crate::field::NamedField;
use crate::schedule::Schedule;
use crate::value::ValidationError;

/// Entity types that support building via the `define_entity_builder!` macro.
///
/// Implementers provide a [`default_data`](Self::default_data) hook that
/// produces an empty [`EntityType::InternalData`] stamped with the caller-
/// supplied [`EntityId`].  The builder seeds the schedule with this
/// placeholder, then applies the user's field writes through
/// [`FieldSet::write_multiple`].  Required fields will intentionally fail
/// [`EntityType::validate`] until the writes run — that is the mechanism by
/// which the builder's "you must set required fields" contract is enforced.
///
/// [`FieldSet::write_multiple`]: crate::field::set::FieldSet::write_multiple
pub trait EntityBuildable: EntityType {
    /// Produce an empty `InternalData` stamped with the given ID.
    ///
    /// All fields are initialized to sensible defaults (typically via
    /// `Default::default()` on the inner `FooCommonData`).  The returned
    /// value is expected to fail [`EntityType::validate`] for any required
    /// field; the builder re-runs validation after batch writes have been
    /// applied.
    fn default_data(id: EntityId<Self>) -> Self::InternalData;

    /// Find all live entities matching the given natural key.
    ///
    /// Used by [`find_or_create_entity`] before falling back to creation.
    /// The `key` format is entity-type-specific (e.g. uppercase prefix for
    /// `PanelTypeEntityType`, lowercase name for room types, uppercase code
    /// for `PanelEntityType` and `TimelineEntityType`).
    ///
    /// Must not include tombstoned entities.
    ///
    /// Returns all matching candidates so that the caller can filter by a
    /// `seen` set when duplicate natural keys exist in the source data (a
    /// common human-error in XLSX spreadsheets).
    fn find_by_natural_key(schedule: &Schedule, key: &str) -> Vec<EntityId<Self>>;
}

/// Errors returned by [`build_entity`] (and therefore by every generated
/// builder's `build` method).
#[derive(Debug, Error)]
pub enum BuildError {
    /// The batch field write (or its verification phase) failed.
    #[error("field batch failed: {0}")]
    FieldSet(#[from] FieldSetError),

    /// The entity failed [`EntityType::validate`] after all writes were
    /// applied.  The rollback has already removed the placeholder from the
    /// schedule.
    #[error("validation failed: {0:?}")]
    Validation(Vec<ValidationError>),

    /// The requested UUID already exists for this entity type.
    ///
    /// This occurs with `UuidPreference::Exact` or `UuidPreference::ExactFromV5`
    /// when the UUID is already in use.  The caller should retry with a different
    /// UUID or use a "prefer" variant that allows fallback.
    #[error("UUID {0} already exists for entity type {1}")]
    UuidConflict(uuid::NonNilUuid, &'static str),
}

/// Find or create an entity by its natural key.
///
/// # `seen` and `allow_reuse`
///
/// `seen` is the set of UUIDs already consumed during the current import pass.
/// How `seen` is used depends on `allow_reuse`:
///
/// - **`allow_reuse = false`** (panels, timelines): duplicate natural keys in
///   the source spreadsheet must each bind to a *distinct* entity.  Candidates
///   already in `seen` are excluded so successive rows with the same code are
///   assigned to different entities.  Among remaining candidates the one whose
///   stored field values have the smallest edit distance to the incoming updates
///   is chosen.  If none remain, a new entity is created.
///
/// - **`allow_reuse = true`** (rooms, hotel rooms, panel types): the same name
///   is expected to appear on many rows and must always resolve to the *same*
///   entity.  `seen` is ignored entirely — the first (and normally only)
///   matching entity is reused even if it was already consumed by a previous
///   row.
///
/// Pass `&HashSet::new()` for non-import callers that do not need dedup.
pub fn find_or_create_entity<E: EntityBuildable>(
    schedule: &mut Schedule,
    key: &str,
    seen: &HashSet<NonNilUuid>,
    allow_reuse: bool,
    updates: Vec<FieldUpdate<E>>,
) -> Result<EntityId<E>, BuildError> {
    let candidates = E::find_by_natural_key(schedule, key);

    let mut pool: Vec<EntityId<E>> = if allow_reuse {
        // Reuse mode: prefer candidates already in `seen` (stable binding —
        // once this import has picked an entity for a name, stick with it).
        // Fast-path: single candidate is the overwhelmingly common case.
        if candidates.len() <= 1 {
            candidates
        } else {
            let seen_candidates: Vec<EntityId<E>> = candidates
                .iter()
                .copied()
                .filter(|id| seen.contains(&id.entity_uuid()))
                .collect();
            if seen_candidates.is_empty() {
                candidates
            } else {
                seen_candidates
            }
        }
    } else {
        // Dedup mode: exclude already-seen entities so each duplicate-code row
        // binds to a distinct entity rather than collapsing onto the first one.
        candidates
            .into_iter()
            .filter(|id| !seen.contains(&id.entity_uuid()))
            .collect()
    };

    let chosen = if pool.len() <= 1 {
        pool.pop()
    } else {
        // Multiple candidates remain.  Pick the one whose stored field values
        // are most similar to the incoming updates using Levenshtein distance
        // over a concatenated field-value fingerprint.
        let incoming = updates_fingerprint(&updates);
        pool.into_iter()
            .min_by_key(|id| edit_distance(&entity_fingerprint::<E>(schedule, *id), &incoming))
    };

    if let Some(id) = chosen {
        E::field_set().write_multiple(id, schedule, &updates)?;
        Ok(id)
    } else {
        build_entity(
            schedule,
            UuidPreference::PreferFromV5 {
                name: key.to_owned(),
            },
            updates,
        )
    }
}

/// Build a stable fingerprint string from a slice of [`FieldUpdate`]s.
///
/// Only `Set` operations are included (Add/Remove are order-sensitive and
/// uncommon in import paths).  Updates are sorted by field-ref display so
/// that the fingerprint is stable regardless of the order the caller supplies
/// them.
fn updates_fingerprint<E: EntityType>(updates: &[FieldUpdate<E>]) -> String {
    let mut values: Vec<String> = updates
        .iter()
        .filter(|u| u.op == FieldOp::Set)
        .map(|u| u.value.to_string())
        .collect();
    values.sort_unstable();
    values.join("|")
}

/// Build a fingerprint from the readable fields of an existing entity.
fn entity_fingerprint<E: EntityBuildable>(schedule: &Schedule, id: EntityId<E>) -> String {
    let mut pairs: Vec<String> = E::field_set()
        .fields()
        .filter_map(|desc| {
            let v = desc.read(id, schedule).ok().flatten()?;
            Some(format!("{}={}", desc.name(), v))
        })
        .collect();
    pairs.sort_unstable();
    pairs.join("|")
}

/// Compute the Levenshtein edit distance between two strings.
///
/// Used to pick the best-matching candidate when multiple entities share a
/// natural key.
pub(crate) fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = b.len();
    // Rolling single-row DP — O(n) space.
    let mut row: Vec<usize> = (0..=n).collect();
    for ca in &a {
        let mut prev = row[0];
        row[0] += 1;
        for (j, cb) in b.iter().enumerate() {
            let next = if ca == cb {
                prev
            } else {
                1 + prev.min(row[j]).min(row[j + 1])
            };
            prev = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[n]
}

/// Seed, populate, and validate a new entity of type `E`.
///
/// # Steps
///
/// 1. Resolve `uuid_pref` to a typed [`EntityId<E>`].
/// 2. Check for UUID conflicts:
///    - For `Exact` and `ExactFromV5`: return [`BuildError::UuidConflict`] if exists
///    - For `Prefer` and `PreferFromV5`: if conflict, fall back to `GenerateNew`
/// 3. Insert [`EntityBuildable::default_data`] into `schedule`.
/// 4. Call [`FieldSet::write_multiple`] with `updates`.  On any
///    [`FieldSetError`] the seed is removed via
///    [`Schedule::remove_entity`] (which also clears edges), and the error
///    is wrapped in [`BuildError::FieldSet`].
/// 5. Run [`EntityType::validate`] on the final internal data.  Any
///    [`ValidationError`]s trigger the same rollback and produce
///    [`BuildError::Validation`].
///
/// On success the caller-reachable `EntityId<E>` is returned; the entity is
/// fully inserted into the schedule, indistinguishable from one created by
/// any other path.
///
/// [`FieldSet::write_multiple`]: crate::field::set::FieldSet::write_multiple
pub fn build_entity<E: EntityBuildable>(
    schedule: &mut Schedule,
    uuid_pref: UuidPreference,
    updates: Vec<FieldUpdate<E>>,
) -> Result<EntityId<E>, BuildError> {
    // Resolve the UUID preference with conflict checking
    let id = match &uuid_pref {
        UuidPreference::Exact(_) | UuidPreference::ExactFromV5 { .. } => {
            schedule
                .try_resolve_entity_id::<E>(uuid_pref.clone())
                .ok_or_else(|| {
                    BuildError::UuidConflict(
                        // SAFETY: We know the UUID from the preference
                        unsafe { EntityId::<E>::from_preference_unchecked(uuid_pref.clone()) }
                            .entity_uuid(),
                        E::TYPE_NAME,
                    )
                })?
        }
        UuidPreference::Prefer(_)
        | UuidPreference::PreferFromV5 { .. }
        | UuidPreference::GenerateNew => schedule
            .try_resolve_entity_id::<E>(uuid_pref.clone())
            .expect("prefer variants and GenerateNew always return Some"),
    };

    schedule.insert(id, E::default_data(id));

    if let Err(e) = E::field_set().write_multiple(id, schedule, &updates) {
        schedule.remove_entity::<E>(id);
        return Err(BuildError::FieldSet(e));
    }

    // Safety: we just inserted this id; write_multiple does not remove it.
    let data = schedule
        .get_internal::<E>(id)
        .expect("entity was inserted at seed step");
    let errs = E::validate(data);
    if !errs.is_empty() {
        schedule.remove_entity::<E>(id);
        return Err(BuildError::Validation(errs));
    }

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityUuid;
    use crate::tables::panel_type::PanelTypeEntityType;

    fn valid_panel_type_updates() -> Vec<FieldUpdate<PanelTypeEntityType>> {
        vec![
            FieldUpdate::set("prefix", "GP"),
            FieldUpdate::set("panel_kind", "Guest Panel"),
        ]
    }

    #[test]
    fn build_entity_generate_new_succeeds() {
        let mut sched = Schedule::default();
        let id = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::GenerateNew,
            valid_panel_type_updates(),
        )
        .unwrap();

        let data = sched.get_internal::<PanelTypeEntityType>(id).unwrap();
        assert_eq!(data.data.prefix, "GP");
        assert_eq!(data.data.panel_kind, "Guest Panel");
    }

    #[test]
    fn build_entity_from_v5_is_deterministic() {
        let mut sched1 = Schedule::default();
        let id1 = build_entity::<PanelTypeEntityType>(
            &mut sched1,
            UuidPreference::ExactFromV5 { name: "GP".into() },
            valid_panel_type_updates(),
        )
        .unwrap();

        let mut sched2 = Schedule::default();
        let id2 = build_entity::<PanelTypeEntityType>(
            &mut sched2,
            UuidPreference::ExactFromV5 { name: "GP".into() },
            valid_panel_type_updates(),
        )
        .unwrap();

        assert_eq!(id1.entity_uuid(), id2.entity_uuid());
    }

    #[test]
    fn build_entity_validation_failure_rolls_back() {
        // Omit required `prefix` field — PanelType::validate should complain
        // and the placeholder must be removed from the schedule.
        let mut sched = Schedule::default();
        let err = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::GenerateNew,
            vec![FieldUpdate::set("panel_kind", "Guest Panel")],
        )
        .unwrap_err();

        assert!(matches!(err, BuildError::Validation(_)));
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 0);
    }

    #[test]
    fn build_entity_write_failure_rolls_back() {
        // Unknown field name → FieldSetError::UnknownField → BuildError::FieldSet.
        let mut sched = Schedule::default();
        let err = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::GenerateNew,
            vec![FieldUpdate::set("definitely_not_a_field", "x")],
        )
        .unwrap_err();

        assert!(matches!(
            err,
            BuildError::FieldSet(FieldSetError::UnknownField(_))
        ));
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 0);
    }

    #[test]
    fn build_entity_exact_conflicts_error() {
        let mut sched = Schedule::default();
        let id1 = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::GenerateNew,
            valid_panel_type_updates(),
        )
        .unwrap();

        // Try to create another entity with the same UUID using Exact
        let err = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::Exact(id1.entity_uuid()),
            valid_panel_type_updates(),
        )
        .unwrap_err();

        assert!(matches!(err, BuildError::UuidConflict(_, "panel_type")));
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 1);
    }

    #[test]
    fn build_entity_exact_from_v5_conflicts_error() {
        let mut sched = Schedule::default();
        let _id1 = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::ExactFromV5 { name: "GP".into() },
            valid_panel_type_updates(),
        )
        .unwrap();

        // Try to create another entity with the same name using ExactFromV5
        let err = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::ExactFromV5 { name: "GP".into() },
            valid_panel_type_updates(),
        )
        .unwrap_err();

        assert!(matches!(err, BuildError::UuidConflict(_, "panel_type")));
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 1);
    }

    #[test]
    fn build_entity_prefer_falls_back_on_conflict() {
        let mut sched = Schedule::default();
        let id1 = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::GenerateNew,
            valid_panel_type_updates(),
        )
        .unwrap();

        // Try to create another entity with Prefer using the same UUID
        let id2 = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::Prefer(id1.entity_uuid()),
            valid_panel_type_updates(),
        )
        .unwrap();

        // Should succeed with a different UUID
        assert_ne!(id1.entity_uuid(), id2.entity_uuid());
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 2);
    }

    #[test]
    fn build_entity_prefer_from_v5_falls_back_on_conflict() {
        let mut sched = Schedule::default();
        let id1 = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::ExactFromV5 { name: "GP".into() },
            valid_panel_type_updates(),
        )
        .unwrap();

        // Try to create another entity with PreferFromV5 using the same name
        let id2 = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::PreferFromV5 { name: "GP".into() },
            valid_panel_type_updates(),
        )
        .unwrap();

        // Should succeed with a different UUID
        assert_ne!(id1.entity_uuid(), id2.entity_uuid());
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 2);
    }

    #[test]
    fn build_entity_prefer_from_v5_succeeds_when_no_conflict() {
        let mut sched = Schedule::default();
        let id1 = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::PreferFromV5 { name: "GP".into() },
            valid_panel_type_updates(),
        )
        .unwrap();

        // Create another entity with a different name
        let id2 = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::PreferFromV5 { name: "SP".into() },
            valid_panel_type_updates(),
        )
        .unwrap();

        // Both should succeed with different UUIDs
        assert_ne!(id1.entity_uuid(), id2.entity_uuid());
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 2);
    }

    #[test]
    fn build_entity_exact_recreates_tombstoned_entity() {
        let mut sched = Schedule::default();
        let id = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::GenerateNew,
            valid_panel_type_updates(),
        )
        .unwrap();

        // Remove the entity (tombstone it)
        sched.remove_entity::<PanelTypeEntityType>(id);
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 0);
        assert!(sched.is_entity_deleted(id));

        // Recreate with the same UUID using Exact - should succeed
        let id2 = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::Exact(id.entity_uuid()),
            valid_panel_type_updates(),
        )
        .unwrap();

        assert_eq!(id.entity_uuid(), id2.entity_uuid());
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 1);
        assert!(!sched.is_entity_deleted(id));
    }

    #[test]
    fn find_or_create_entity_creates_when_absent() {
        let mut sched = Schedule::default();
        let id = find_or_create_entity::<PanelTypeEntityType>(
            &mut sched,
            "GP",
            &HashSet::new(),
            false,
            valid_panel_type_updates(),
        )
        .unwrap();
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 1);
        let data = sched.get_internal::<PanelTypeEntityType>(id).unwrap();
        assert_eq!(data.data.prefix, "GP");
    }

    #[test]
    fn find_or_create_entity_updates_when_present() {
        let mut sched = Schedule::default();
        let id1 = find_or_create_entity::<PanelTypeEntityType>(
            &mut sched,
            "GP",
            &HashSet::new(),
            false,
            valid_panel_type_updates(),
        )
        .unwrap();

        let id2 = find_or_create_entity::<PanelTypeEntityType>(
            &mut sched,
            "GP",
            &HashSet::new(),
            false,
            vec![
                FieldUpdate::set("prefix", "GP"),
                FieldUpdate::set("panel_kind", "Updated Kind"),
            ],
        )
        .unwrap();

        // Same UUID preserved, count unchanged, field updated.
        assert_eq!(id1.entity_uuid(), id2.entity_uuid());
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 1);
        let data = sched.get_internal::<PanelTypeEntityType>(id2).unwrap();
        assert_eq!(data.data.panel_kind, "Updated Kind");
    }

    #[test]
    fn find_or_create_entity_recreates_after_tombstone() {
        let mut sched = Schedule::default();
        let id = find_or_create_entity::<PanelTypeEntityType>(
            &mut sched,
            "GP",
            &HashSet::new(),
            false,
            valid_panel_type_updates(),
        )
        .unwrap();

        sched.remove_entity::<PanelTypeEntityType>(id);
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 0);

        let id2 = find_or_create_entity::<PanelTypeEntityType>(
            &mut sched,
            "GP",
            &HashSet::new(),
            false,
            valid_panel_type_updates(),
        )
        .unwrap();
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 1);
        let _ = id2;
    }

    #[test]
    fn find_or_create_entity_preserves_v7_uuid() {
        // An entity created with GenerateNew (v7 UUID) must be found by field
        // scan via find_by_natural_key — its UUID is preserved, not replaced.
        let mut sched = Schedule::default();
        let v7_id = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::GenerateNew,
            valid_panel_type_updates(),
        )
        .unwrap();

        let found_id = find_or_create_entity::<PanelTypeEntityType>(
            &mut sched,
            "GP",
            &HashSet::new(),
            false,
            vec![
                FieldUpdate::set("prefix", "GP"),
                FieldUpdate::set("panel_kind", "Updated Kind"),
            ],
        )
        .unwrap();

        assert_eq!(v7_id.entity_uuid(), found_id.entity_uuid());
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 1);
        let data = sched.get_internal::<PanelTypeEntityType>(found_id).unwrap();
        assert_eq!(data.data.panel_kind, "Updated Kind");
    }

    #[test]
    fn build_entity_exact_from_v5_recreates_tombstoned_entity() {
        let mut sched = Schedule::default();
        let id = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::ExactFromV5 { name: "GP".into() },
            valid_panel_type_updates(),
        )
        .unwrap();

        // Remove the entity (tombstone it)
        sched.remove_entity::<PanelTypeEntityType>(id);
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 0);
        assert!(sched.is_entity_deleted(id));

        // Recreate with the same name using ExactFromV5 - should succeed
        let id2 = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::ExactFromV5 { name: "GP".into() },
            valid_panel_type_updates(),
        )
        .unwrap();

        assert_eq!(id.entity_uuid(), id2.entity_uuid());
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 1);
        assert!(!sched.is_entity_deleted(id));
    }

    #[test]
    fn find_or_create_entity_seen_skips_already_consumed() {
        // When the single matching entity is in the seen set, a new entity must
        // be created rather than re-using the seen one.  This mirrors the XLSX
        // duplicate-code scenario where each row must bind to a distinct entity.
        let mut sched = Schedule::default();

        let id1 = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::GenerateNew,
            valid_panel_type_updates(),
        )
        .unwrap();

        // Mark id1 as already seen.
        let mut seen = HashSet::new();
        seen.insert(id1.entity_uuid());

        // The only match is in seen, so a new entity must be created.
        let id2 = find_or_create_entity::<PanelTypeEntityType>(
            &mut sched,
            "GP",
            &seen,
            false,
            valid_panel_type_updates(),
        )
        .unwrap();

        assert_ne!(id1.entity_uuid(), id2.entity_uuid());
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 2);
    }

    #[test]
    fn edit_distance_basic() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("abc", "abc"), 0);
        assert_eq!(edit_distance("abc", ""), 3);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("GP032", "GP032"), 0);
        assert_eq!(edit_distance("GP032", "GP033"), 1);
    }
}
