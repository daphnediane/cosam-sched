/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field update strategies and validation

use super::{FieldError, FieldValue, ValidationError};
use crate::entity::EntityType;
use crate::schedule::Schedule;

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
        schedule: &mut Schedule,
    ) -> Result<(), FieldError>;

    /// Apply batch updates
    fn apply_batch_updates(
        &mut self,
        entity: &mut T::Data,
        batch: BatchUpdate,
        schedule: &mut Schedule,
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
        schedule: &mut Schedule,
    ) -> Result<(), FieldError>;
}

// TODO(Step 5): Port DefaultFieldUpdater to new trait model
// The previous implementation used T::fields(), FieldDescriptor, ValidatorAccessKind,
// and WriteAccessKind which no longer exist. It needs to be rewritten to use
// EntityType::field_set() and the NamedField/SimpleReadableField/SimpleWritableField traits.
pub struct DefaultFieldUpdater;

impl DefaultFieldUpdater {
    // TODO: Implement DefaultFieldUpdater
}

impl<T: EntityType> FieldUpdater<T> for DefaultFieldUpdater {
    fn update_field(
        &mut self,
        _entity: &mut T::Data,
        _field_name: &str,
        _value: FieldValue,
        _strategy: UpdateStrategy,
        _schedule: &mut Schedule,
    ) -> Result<(), FieldError> {
        // TODO: Implement update_field
        unimplemented!()
    }

    fn apply_batch_updates(
        &mut self,
        _entity: &mut T::Data,
        _batch: BatchUpdate,
        _schedule: &mut Schedule,
    ) -> UpdateResult {
        // TODO: Implement apply_batch_updates
        unimplemented!()
    }

    fn detect_conflicts(
        &self,
        _entity: &T::Data,
        _updates: &[FieldUpdate],
        _schedule: &Schedule,
    ) -> Vec<UpdateConflict> {
        // TODO: Implement detect_conflicts
        unimplemented!()
    }

    fn resolve_conflicts(
        &mut self,
        _entity: &mut T::Data,
        _conflicts: Vec<UpdateConflict>,
        _resolution: ConflictResolution,
        _schedule: &mut Schedule,
    ) -> Result<(), FieldError> {
        // TODO: Implement resolve_conflicts
        unimplemented!()
    }
}
