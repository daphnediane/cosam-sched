/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field update strategies and validation

use super::{FieldError, FieldValue, ValidationError, ValidatorAccessKind, WriteAccessKind};
use crate::entity::EntityType;
use crate::schedule::Schedule;
use std::collections::HashMap;

/// Field update strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateStrategy {
    /// Replace the current value entirely
    Replace,
    /// Merge with current value (for lists, maps)
    Merge,
    /// Update only if current value is null/empty
    SetIfNull,
    /// Append to current value (for lists)
    Append,
    /// Remove from current value (for lists)
    Remove,
}

/// Field update operation
#[derive(Debug, Clone)]
pub struct FieldUpdate {
    pub field_name: String,
    pub value: FieldValue,
    pub strategy: UpdateStrategy,
}

impl FieldUpdate {
    pub fn new(field_name: impl Into<String>, value: impl Into<FieldValue>) -> Self {
        Self {
            field_name: field_name.into(),
            value: value.into(),
            strategy: UpdateStrategy::Replace,
        }
    }

    pub fn with_strategy(mut self, strategy: UpdateStrategy) -> Self {
        self.strategy = strategy;
        self
    }
}

/// Batch field update operation
#[derive(Debug, Clone)]
pub struct BatchUpdate {
    pub updates: Vec<FieldUpdate>,
    pub validate_after: bool,
    pub stop_on_first_error: bool,
}

impl BatchUpdate {
    pub fn new() -> Self {
        Self {
            updates: Vec::new(),
            validate_after: true,
            stop_on_first_error: false,
        }
    }

    pub fn add_update(mut self, update: FieldUpdate) -> Self {
        self.updates.push(update);
        self
    }

    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate_after = validate;
        self
    }

    pub fn stop_on_error(mut self, stop: bool) -> Self {
        self.stop_on_first_error = stop;
        self
    }
}

impl Default for BatchUpdate {
    fn default() -> Self {
        Self::new()
    }
}

/// Field update result
#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub successful_updates: Vec<String>,
    pub failed_updates: Vec<(String, FieldError)>,
    pub warnings: Vec<String>,
    pub validation_errors: Vec<ValidationError>,
}

impl UpdateResult {
    pub fn new() -> Self {
        Self {
            successful_updates: Vec::new(),
            failed_updates: Vec::new(),
            warnings: Vec::new(),
            validation_errors: Vec::new(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.failed_updates.is_empty() && self.validation_errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn add_success(&mut self, field_name: String) {
        self.successful_updates.push(field_name);
    }

    pub fn add_failure(&mut self, field_name: String, error: FieldError) {
        self.failed_updates.push((field_name, error));
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn add_validation_error(&mut self, error: ValidationError) {
        self.validation_errors.push(error);
    }
}

impl Default for UpdateResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Keep the existing value
    KeepExisting,
    /// Use the new value
    UseNew,
    /// Merge both values (if possible)
    Merge,
    /// Fail the operation
    Fail,
}

/// Field update conflict
#[derive(Debug, Clone)]
pub struct UpdateConflict {
    pub field_name: String,
    pub existing_value: FieldValue,
    pub new_value: FieldValue,
    pub conflict_type: ConflictType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    /// Values are different
    ValueMismatch,
    /// Type conversion failed
    TypeConversion,
    /// Validation failed
    Validation,
    /// Field is read-only
    ReadOnly,
}

/// Trait for handling field updates
pub trait FieldUpdater<T: EntityType> {
    /// Update a single field
    fn update_field(
        &mut self,
        entity: &mut T::Data,
        field_name: &str,
        value: FieldValue,
        strategy: UpdateStrategy,
        schedule: &Schedule,
    ) -> Result<(), FieldError>;

    /// Apply batch updates
    fn apply_batch_updates(
        &mut self,
        entity: &mut T::Data,
        batch: BatchUpdate,
        schedule: &Schedule,
    ) -> UpdateResult;

    /// Detect conflicts between updates
    fn detect_conflicts(
        &self,
        entity: &T::Data,
        updates: &[FieldUpdate],
        schedule: &Schedule,
    ) -> Vec<UpdateConflict>;

    /// Resolve conflicts using the specified strategy
    fn resolve_conflicts(
        &mut self,
        entity: &mut T::Data,
        conflicts: Vec<UpdateConflict>,
        resolution: ConflictResolution,
        schedule: &Schedule,
    ) -> Result<(), FieldError>;
}

/// Default field updater implementation
pub struct DefaultFieldUpdater;

impl DefaultFieldUpdater {
    fn should_run_custom_validator<T: EntityType>(
        field: &crate::field::FieldDescriptor<T>,
        successful_write_count: usize,
    ) -> bool {
        if successful_write_count > 1 {
            return true;
        }

        if field.write_access_kind() != WriteAccessKind::Computed {
            return false;
        }

        field.validator_access_kind() != ValidatorAccessKind::None
    }
}

impl<T: EntityType> FieldUpdater<T> for DefaultFieldUpdater {
    fn update_field(
        &mut self,
        entity: &mut T::Data,
        field_name: &str,
        value: FieldValue,
        strategy: UpdateStrategy,
        schedule: &Schedule,
    ) -> Result<(), FieldError> {
        let field = T::fields()
            .iter()
            .find(|f| f.name == field_name)
            .ok_or_else(|| {
                FieldError::ValidationError(ValidationError::ValidationFailed {
                    field: field_name.to_string(),
                    reason: "Field not found".to_string(),
                })
            })?;

        match strategy {
            UpdateStrategy::Replace => field.write(entity, value),
            UpdateStrategy::SetIfNull => {
                if field.read(entity, schedule).is_none() {
                    field.write(entity, value)
                } else {
                    Ok(())
                }
            }
            UpdateStrategy::Merge | UpdateStrategy::Append => {
                let merged_value = match (field.read(entity, schedule), value) {
                    (Some(FieldValue::List(mut existing)), FieldValue::List(new_items)) => {
                        existing.extend(new_items);
                        FieldValue::List(existing)
                    }
                    (_, v) => v,
                };

                field.write(entity, merged_value)
            }
            UpdateStrategy::Remove => {
                let reduced_value = match (field.read(entity, schedule), value) {
                    (Some(FieldValue::List(mut existing)), FieldValue::List(to_remove)) => {
                        existing.retain(|item| !to_remove.contains(item));
                        FieldValue::List(existing)
                    }
                    (Some(current), _) => current,
                    (None, _) => return Ok(()),
                };

                field.write(entity, reduced_value)
            }
        }
    }

    fn apply_batch_updates(
        &mut self,
        entity: &mut T::Data,
        batch: BatchUpdate,
        schedule: &Schedule,
    ) -> UpdateResult {
        let mut result = UpdateResult::new();

        for update in &batch.updates {
            match FieldUpdater::<T>::update_field(
                self,
                entity,
                &update.field_name,
                update.value.clone(),
                update.strategy,
                schedule,
            ) {
                Ok(()) => {
                    result.add_success(update.field_name.clone());
                }
                Err(error) => {
                    result.add_failure(update.field_name.clone(), error);
                    if batch.stop_on_first_error {
                        break;
                    }
                }
            }
        }

        let mut successful_write_counts: HashMap<String, usize> = HashMap::new();
        for field_name in &result.successful_updates {
            *successful_write_counts
                .entry(field_name.clone())
                .or_insert(0) += 1;
        }

        // Second pass validation after all updates were applied in order.
        if batch.validate_after {
            for update in &batch.updates {
                if !result.successful_updates.contains(&update.field_name) {
                    continue;
                }

                if let Some(field) = T::fields().iter().find(|f| f.name == update.field_name) {
                    let expected_value = match field.field_type.try_convert(&update.value) {
                        Ok(v) => v,
                        Err(_) => update.value.clone(),
                    };

                    if let Some(actual_value) = field.read(entity, schedule) {
                        if actual_value != expected_value {
                            result.add_validation_error(ValidationError::ValidationFailed {
                                field: update.field_name.clone(),
                                reason: format!(
                                    "written value mismatch: expected '{}', got '{}'",
                                    expected_value, actual_value
                                ),
                            });
                            if batch.stop_on_first_error {
                                return result;
                            }
                        }
                    }

                    let successful_write_count = successful_write_counts
                        .get(&update.field_name)
                        .copied()
                        .unwrap_or(0);

                    if Self::should_run_custom_validator(field, successful_write_count) {
                        if let Err(error) = field.validate_write(entity, &expected_value) {
                            result.add_validation_error(error);
                            if batch.stop_on_first_error {
                                return result;
                            }
                        }
                    }
                }
            }

            if let Err(error) = T::validate(entity) {
                result.add_validation_error(error);
            }
        }

        result
    }

    fn detect_conflicts(
        &self,
        entity: &T::Data,
        updates: &[FieldUpdate],
        schedule: &Schedule,
    ) -> Vec<UpdateConflict> {
        let mut conflicts = Vec::new();

        for update in updates {
            if let Some(field) = T::fields().iter().find(|f| f.name == update.field_name) {
                if let Some(current_value) = field.extract(entity, schedule) {
                    // Check for value mismatch
                    if current_value != update.value {
                        conflicts.push(UpdateConflict {
                            field_name: update.field_name.clone(),
                            existing_value: current_value,
                            new_value: update.value.clone(),
                            conflict_type: ConflictType::ValueMismatch,
                        });
                    }
                }
            }
        }

        conflicts
    }

    fn resolve_conflicts(
        &mut self,
        entity: &mut T::Data,
        conflicts: Vec<UpdateConflict>,
        resolution: ConflictResolution,
        schedule: &Schedule,
    ) -> Result<(), FieldError> {
        for conflict in conflicts {
            match resolution {
                ConflictResolution::KeepExisting => {
                    // Do nothing, keep existing value
                }
                ConflictResolution::UseNew => {
                    // Apply the new value
                    FieldUpdater::<T>::update_field(
                        self,
                        entity,
                        &conflict.field_name,
                        conflict.new_value,
                        UpdateStrategy::Replace,
                        schedule,
                    )?;
                }
                ConflictResolution::Merge => {
                    // Try to merge values (simplified for now)
                    if let (FieldValue::List(mut existing), FieldValue::List(new)) =
                        (conflict.existing_value.clone(), conflict.new_value.clone())
                    {
                        existing.extend(new);
                        FieldUpdater::<T>::update_field(
                            self,
                            entity,
                            &conflict.field_name,
                            FieldValue::List(existing),
                            UpdateStrategy::Replace,
                            schedule,
                        )?;
                    } else {
                        // Can't merge, use new value
                        FieldUpdater::<T>::update_field(
                            self,
                            entity,
                            &conflict.field_name,
                            conflict.new_value,
                            UpdateStrategy::Replace,
                            schedule,
                        )?;
                    }
                }
                ConflictResolution::Fail => {
                    return Err(FieldError::ValidationError(
                        ValidationError::ValidationFailed {
                            field: conflict.field_name,
                            reason: "Update conflict detected".to_string(),
                        },
                    ));
                }
            }
        }

        Ok(())
    }
}
