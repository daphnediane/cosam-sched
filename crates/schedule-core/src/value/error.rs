/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Error types for field operations.

use thiserror::Error;

use crate::value::FieldValue;

// ── FieldError ───────────────────────────────────────────────────────────────

/// Top-level error for field operations.
#[derive(Debug, Error)]
pub enum FieldError {
    /// Type conversion failed.
    #[error("conversion error: {0}")]
    Conversion(#[from] ConversionError),
    /// Field value failed validation.
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),
    /// Field value failed verification after batch write.
    #[error("verification error: {0}")]
    Verification(#[from] VerificationError),
    /// Field is read-only (no write_fn).
    #[error("field '{name}' is read-only")]
    ReadOnly { name: &'static str },
    /// Field is write-only (no read_fn).
    #[error("field '{name}' is write-only")]
    WriteOnly { name: &'static str },
    /// Entity not found in the schedule.
    #[error("field '{name}': entity not found")]
    NotFound { name: &'static str },
    /// Mirroring the write to the authoritative CRDT document failed.
    #[error("field '{name}': CRDT mirror failed: {detail}")]
    Crdt {
        name: &'static str,
        detail: std::string::String,
    },
    /// Edge operation failed.
    #[error("edge operation failed: {0}")]
    Edge(#[from] crate::edge::map::EdgeError),
}

// ── ConversionError ─────────────────────────────────────────────────────────

/// Type conversion failure — wrong `FieldValue` variant or parse error.
#[derive(Debug, Error)]
pub enum ConversionError {
    /// Caller supplied the wrong variant.
    #[error("expected {expected}, got {got}")]
    WrongVariant {
        expected: &'static str,
        got: &'static str,
    },
    /// A string could not be parsed into the target type.
    #[error("parse error: {message}")]
    ParseError { message: std::string::String },
    /// Invalid edge configuration.
    #[error("invalid edge: {reason}")]
    InvalidEdge { reason: std::string::String },
}

// ── ValidationError ─────────────────────────────────────────────────────────

/// Value fails field constraints.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// A required field was absent or empty.
    #[error("field '{field}' is required")]
    Required { field: &'static str },
    /// Value is outside the allowed range.
    #[error("field '{field}': value out of range — {message}")]
    OutOfRange {
        field: &'static str,
        message: std::string::String,
    },
    /// Value violates an application-specific constraint.
    #[error("field '{field}': {message}")]
    Constraint {
        field: &'static str,
        message: std::string::String,
    },
}

// ── VerificationError ───────────────────────────────────────────────────────

/// Verification failure — field value changed during batch write.
#[derive(Debug, Error)]
pub enum VerificationError {
    /// The field value was changed by another write in the same batch.
    #[error("field '{field}': value changed during batch write — requested {requested:?}, actual {actual:?}")]
    ValueChanged {
        field: &'static str,
        requested: FieldValue,
        actual: FieldValue,
    },
    /// The field cannot be verified (e.g., `ReRead` requested but field is write-only).
    #[error("field '{field}': cannot be verified — read not available for re-read verification")]
    NotVerifiable { field: &'static str },
}
