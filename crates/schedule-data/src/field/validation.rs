/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field validation system

use std::fmt;

/// Validation error types
#[derive(Debug, Clone)]
pub enum ValidationError {
    RequiredFieldMissing {
        field: String,
    },
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },
    ValidationFailed {
        field: String,
        reason: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::RequiredFieldMissing { field } => {
                write!(f, "Required field '{}' is missing", field)
            }
            ValidationError::InvalidValue {
                field,
                value,
                reason,
            } => {
                write!(
                    f,
                    "Invalid value '{}' for field '{}': {}",
                    value, field, reason
                )
            }
            ValidationError::ValidationFailed { field, reason } => {
                write!(f, "Validation failed for field '{}': {}", field, reason)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Conversion error types
#[derive(Debug, Clone)]
pub enum ConversionError {
    InvalidFormat,
    InvalidTimestamp,
    UnsupportedType,
    OutOfRange,
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionError::InvalidFormat => write!(f, "Invalid format"),
            ConversionError::InvalidTimestamp => write!(f, "Invalid timestamp"),
            ConversionError::UnsupportedType => write!(f, "Unsupported type"),
            ConversionError::OutOfRange => write!(f, "Value out of range"),
        }
    }
}

impl std::error::Error for ConversionError {}

/// Field error types
#[derive(Debug, Clone)]
pub enum FieldError {
    CannotStoreComputedField,
    CannotStoreRelationshipField,
    ConversionError(ConversionError),
    ValidationError(ValidationError),
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldError::CannotStoreComputedField => {
                write!(f, "Cannot store computed field")
            }
            FieldError::CannotStoreRelationshipField => {
                write!(f, "Cannot store relationship field")
            }
            FieldError::ConversionError(e) => write!(f, "Conversion error: {}", e),
            FieldError::ValidationError(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl std::error::Error for FieldError {}

impl From<ConversionError> for FieldError {
    fn from(error: ConversionError) -> Self {
        Self::ConversionError(error)
    }
}

impl From<ValidationError> for FieldError {
    fn from(error: ValidationError) -> Self {
        Self::ValidationError(error)
    }
}

/// Validation context for collecting multiple errors
#[derive(Debug, Clone, Default)]
pub struct ValidationContext {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

impl ValidationContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn result(self) -> Result<(), Vec<ValidationError>> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }
}

/// Trait for field validation
pub trait FieldValidator {
    fn validate(&self, value: &super::FieldValue) -> Result<(), ValidationError>;
}

/// Required field validator
#[derive(Debug, Clone)]
pub struct RequiredValidator;

impl FieldValidator for RequiredValidator {
    fn validate(&self, value: &super::FieldValue) -> Result<(), ValidationError> {
        match value {
            super::FieldValue::String(s) if s.is_empty() => {
                Err(ValidationError::RequiredFieldMissing {
                    field: "required".to_string(),
                })
            }
            super::FieldValue::List(list) if list.is_empty() => {
                Err(ValidationError::RequiredFieldMissing {
                    field: "required".to_string(),
                })
            }
            _ => Ok(()),
        }
    }
}

/// String length validator
#[derive(Debug, Clone)]
pub struct LengthValidator {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

impl LengthValidator {
    pub fn new(min: Option<usize>, max: Option<usize>) -> Self {
        Self { min, max }
    }
}

impl FieldValidator for LengthValidator {
    fn validate(&self, value: &super::FieldValue) -> Result<(), ValidationError> {
        if let super::FieldValue::String(s) = value {
            let len = s.len();
            if let Some(min) = self.min {
                if len < min {
                    return Err(ValidationError::InvalidValue {
                        field: "length".to_string(),
                        value: s.clone(),
                        reason: format!("String too short (minimum {} characters)", min),
                    });
                }
            }
            if let Some(max) = self.max {
                if len > max {
                    return Err(ValidationError::InvalidValue {
                        field: "length".to_string(),
                        value: s.clone(),
                        reason: format!("String too long (maximum {} characters)", max),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Numeric range validator
#[derive(Debug, Clone)]
pub struct RangeValidator {
    pub min: Option<i64>,
    pub max: Option<i64>,
}

impl RangeValidator {
    pub fn new(min: Option<i64>, max: Option<i64>) -> Self {
        Self { min, max }
    }
}

impl FieldValidator for RangeValidator {
    fn validate(&self, value: &super::FieldValue) -> Result<(), ValidationError> {
        if let super::FieldValue::Integer(i) = value {
            if let Some(min) = self.min {
                if *i < min {
                    return Err(ValidationError::InvalidValue {
                        field: "range".to_string(),
                        value: i.to_string(),
                        reason: format!("Value too small (minimum {})", min),
                    });
                }
            }
            if let Some(max) = self.max {
                if *i > max {
                    return Err(ValidationError::InvalidValue {
                        field: "range".to_string(),
                        value: i.to_string(),
                        reason: format!("Value too large (maximum {})", max),
                    });
                }
            }
        }
        Ok(())
    }
}
