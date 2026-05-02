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

use crate::entity::{EntityId, EntityType, UuidPreference};
use crate::field::set::{FieldSetError, FieldUpdate};
use crate::schedule::Schedule;
use crate::value::ValidationError;
use thiserror::Error;

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
}

/// Seed, populate, and validate a new entity of type `E`.
///
/// # Steps
///
/// 1. Resolve `uuid_pref` to a typed [`EntityId<E>`].
/// 2. Insert [`EntityBuildable::default_data`] into `schedule`.
/// 3. Call [`FieldSet::write_multiple`] with `updates`.  On any
///    [`FieldSetError`] the seed is removed via
///    [`Schedule::remove_entity`] (which also clears edges), and the error
///    is wrapped in [`BuildError::FieldSet`].
/// 4. Run [`EntityType::validate`] on the final internal data.  Any
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
    let id = EntityId::<E>::from_preference(uuid_pref);
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
            UuidPreference::FromV5 { name: "GP".into() },
            valid_panel_type_updates(),
        )
        .unwrap();

        let mut sched2 = Schedule::default();
        let id2 = build_entity::<PanelTypeEntityType>(
            &mut sched2,
            UuidPreference::FromV5 { name: "GP".into() },
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
}
