/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Query updater implementation for modifying entities

use crate::entity::EntityType;
use crate::field::{
    BatchUpdate, DefaultFieldUpdater, FieldUpdate, FieldUpdater, FieldValue, UpdateResult,
};
use crate::schedule::Schedule;

/// Generic updater for entities
pub struct Updater<'a, T: EntityType> {
    schedule: &'a mut Schedule,
    field_updater: DefaultFieldUpdater,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: EntityType> Updater<'a, T> {
    pub fn new(schedule: &'a mut Schedule) -> Self {
        Self {
            schedule,
            field_updater: DefaultFieldUpdater,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add a new entity
    pub fn add(&mut self, entity: T::Data) -> Result<T::Id, crate::schedule::ScheduleError> {
        self.schedule.add_entity::<T>(entity)
    }

    /// Restore a soft-deleted entity
    pub fn restore(&mut self, id: T::Id) -> Result<T::Id, crate::schedule::ScheduleError> {
        // This would need to be implemented in the storage layer
        // For now, just return the ID as if restored
        Ok(id)
    }

    /// Find entity or add with defaults
    pub fn find_or_add(
        &mut self,
        matches: &[crate::query::FieldMatch],
        default_entity: T::Data,
    ) -> Result<T::Id, crate::schedule::ScheduleError> {
        // Try to find existing entity
        if let Some(id) = self
            .schedule
            .find_entities::<T>(matches, None)
            .into_iter()
            .next()
        {
            // Found existing entity, restore if inactive
            self.restore(id)
        } else {
            // Not found, add new entity
            self.add(default_entity)
        }
    }

    /// Update entity fields
    pub fn update(
        &mut self,
        id: T::Id,
        updates: &[(String, FieldValue)],
    ) -> Result<(), crate::schedule::ScheduleError> {
        self.schedule.update_entity::<T>(id, updates)
    }

    /// Update entity with field update objects
    pub fn update_with_field_updates(
        &mut self,
        id: T::Id,
        updates: &[FieldUpdate],
    ) -> Result<UpdateResult, crate::schedule::ScheduleError> {
        // Get the entity
        if let Some(entity) = self.schedule.get_entity::<T>(id) {
            let mut entity_clone = entity.clone();
            let batch = BatchUpdate {
                updates: updates.to_vec(),
                validate_after: true,
                stop_on_first_error: false,
            };

            let result = FieldUpdater::<T>::apply_batch_updates(
                &mut self.field_updater,
                &mut entity_clone,
                batch,
                self.schedule,
            );

            // Apply successful updates back to the schedule
            if result.is_success() {
                let field_updates: Vec<(String, FieldValue)> = updates
                    .iter()
                    .filter(|update| result.successful_updates.contains(&update.field_name))
                    .map(|update| (update.field_name.clone(), update.value.clone()))
                    .collect();

                self.schedule.update_entity::<T>(id, &field_updates)?;
            }

            Ok(result)
        } else {
            Err(crate::schedule::ScheduleError::EntityNotFound {
                entity_type: T::TYPE_NAME.to_string(),
                id: id.to_string(),
            })
        }
    }

    /// Apply batch updates
    pub fn apply_batch(
        &mut self,
        id: T::Id,
        batch: BatchUpdate,
    ) -> Result<UpdateResult, crate::schedule::ScheduleError> {
        // Get the entity
        if let Some(entity) = self.schedule.get_entity::<T>(id) {
            let mut entity_clone = entity.clone();
            let result = FieldUpdater::<T>::apply_batch_updates(
                &mut self.field_updater,
                &mut entity_clone,
                batch.clone(),
                self.schedule,
            );

            // Apply successful updates back to the schedule
            if result.is_success() {
                // Convert batch updates to field updates
                let field_updates: Vec<(String, FieldValue)> = batch
                    .updates
                    .iter()
                    .filter(|update| result.successful_updates.contains(&update.field_name))
                    .map(|update| (update.field_name.clone(), update.value.clone()))
                    .collect();

                self.schedule.update_entity::<T>(id, &field_updates)?;
            }

            Ok(result)
        } else {
            Err(crate::schedule::ScheduleError::EntityNotFound {
                entity_type: T::TYPE_NAME.to_string(),
                id: id.to_string(),
            })
        }
    }

    /// Soft delete entity
    pub fn delete(&mut self, _id: T::Id) -> Result<(), crate::schedule::ScheduleError> {
        // This would need to be implemented in the storage layer
        // For now, we'll just return success
        Ok(())
    }

    /// Hard delete entity
    pub fn hard_delete(&mut self, _id: T::Id) -> Result<(), crate::schedule::ScheduleError> {
        // This would need to be implemented in the storage layer
        // For now, we'll just return success
        Ok(())
    }
}
