/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field system with type-safe extraction, validation, and matching

pub mod field_set;
pub mod matching;
pub mod traits;
pub mod types;
pub mod update_logic;
pub mod validation;

use std::collections::HashMap;
use std::fmt;

// Re-export core field types
pub use field_set::*;
pub use matching::*;
pub use traits::*;
pub use types::*;
pub use update_logic::*;
pub use validation::*;

/// Universal field value type for generic operations
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    DateTime(chrono::NaiveDateTime),
    Duration(chrono::Duration),
    List(Vec<FieldValue>),
    Map(HashMap<String, FieldValue>),
    Id(String),
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::String(s) => write!(f, "{}", s),
            FieldValue::Integer(i) => write!(f, "{}", i),
            FieldValue::Float(fl) => write!(f, "{}", fl),
            FieldValue::Boolean(b) => write!(f, "{}", b),
            FieldValue::DateTime(dt) => write!(f, "{}", dt),
            FieldValue::Duration(d) => write!(f, "{}m", d.num_minutes()),
            FieldValue::List(list) => {
                write!(f, "[")?;
                for (i, item) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            FieldValue::Map(map) => {
                write!(f, "{{")?;
                for (i, (key, value)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
            FieldValue::Id(id) => write!(f, "Id({})", id),
        }
    }
}
