/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Query finder implementation for searching entities

use super::FieldMatch;
use crate::entity::EntityState;
use crate::schedule::{storage::TypedStorage, Schedule};

/// Generic finder for entities
pub struct Finder<'a, T: TypedStorage + Sized> {
    schedule: &'a Schedule,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: TypedStorage + Sized> Finder<'a, T> {
    pub fn new(schedule: &'a Schedule) -> Self {
        Self {
            schedule,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Find all entity IDs
    pub fn list_all(&self) -> Vec<uuid::NonNilUuid> {
        self.schedule.entities.find::<T>(&[], None)
    }

    /// Find entity IDs by state
    pub fn list_by_state(&self, state: EntityState) -> Vec<uuid::NonNilUuid> {
        let options = crate::query::QueryOptions::new().with_state(state);
        self.schedule.entities.find::<T>(&[], Some(options))
    }

    /// Find all entities as objects
    pub fn all(&self) -> Vec<&T::Data> {
        self.schedule.entities.get_many::<T>(&[], None)
    }

    /// Find entities by state as objects
    pub fn all_by_state(&self, state: EntityState) -> Vec<&T::Data> {
        let options = crate::query::QueryOptions::new().with_state(state);
        self.schedule.entities.get_many::<T>(&[], Some(options))
    }

    /// Find entities matching field conditions
    pub fn find(&self, matches: &[FieldMatch]) -> Vec<uuid::NonNilUuid> {
        self.schedule.entities.find::<T>(matches, None)
    }

    /// Find entities matching field conditions with options
    pub fn find_with_options(
        &self,
        matches: &[FieldMatch],
        options: crate::query::QueryOptions,
    ) -> Vec<uuid::NonNilUuid> {
        self.schedule.entities.find::<T>(matches, Some(options))
    }

    /// Get entities matching field conditions
    pub fn lookup(&self, matches: &[FieldMatch]) -> Vec<&T::Data> {
        self.schedule.entities.get_many::<T>(matches, None)
    }

    /// Get entities matching field conditions with options
    pub fn lookup_with_options(
        &self,
        matches: &[FieldMatch],
        options: crate::query::QueryOptions,
    ) -> Vec<&T::Data> {
        self.schedule.entities.get_many::<T>(matches, Some(options))
    }

    /// Find single entity by field match (returns first match)
    pub fn find_one(&self, matches: &[FieldMatch]) -> Option<uuid::NonNilUuid> {
        let options = crate::query::QueryOptions::new().with_limit(1);
        self.find_with_options(matches, options).into_iter().next()
    }

    /// Get single entity by field match (returns first match)
    pub fn get_one(&self, matches: &[FieldMatch]) -> Option<&T::Data> {
        let options = crate::query::QueryOptions::new().with_limit(1);
        self.lookup_with_options(matches, options)
            .into_iter()
            .next()
    }

    /// Check if any entity matches the conditions
    pub fn exists(&self, matches: &[FieldMatch]) -> bool {
        self.find_one(matches).is_some()
    }

    /// Count entities matching conditions
    pub fn count(&self, matches: &[FieldMatch]) -> usize {
        self.find(matches).len()
    }

    /// Count entities by state
    pub fn count_by_state(&self, state: EntityState) -> usize {
        self.list_by_state(state).len()
    }
}
