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

use uuid::Uuid;

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
    OptionalString(Option<String>),
    OptionalInteger(Option<i64>),
    OptionalFloat(Option<f64>),
    OptionalBoolean(Option<bool>),
    OptionalDateTime(Option<chrono::NaiveDateTime>),
    OptionalDuration(Option<chrono::Duration>),
    Uuid(Uuid),
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
            FieldValue::OptionalString(opt_s) => match opt_s {
                Some(s) => write!(f, "{}", s),
                None => write!(f, "null"),
            },
            FieldValue::OptionalInteger(opt_i) => match opt_i {
                Some(i) => write!(f, "{}", i),
                None => write!(f, "null"),
            },
            FieldValue::OptionalFloat(opt_f) => match opt_f {
                Some(fl) => write!(f, "{}", fl),
                None => write!(f, "null"),
            },
            FieldValue::OptionalBoolean(opt_b) => match opt_b {
                Some(b) => write!(f, "{}", b),
                None => write!(f, "null"),
            },
            FieldValue::OptionalDateTime(opt_dt) => match opt_dt {
                Some(dt) => write!(f, "{}", dt),
                None => write!(f, "null"),
            },
            FieldValue::OptionalDuration(opt_d) => match opt_d {
                Some(d) => write!(f, "{}m", d.num_minutes()),
                None => write!(f, "null"),
            },
            FieldValue::Uuid(uuid) => write!(f, "{}", uuid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn field_value_uuid_display() {
        let uuid = Uuid::nil();
        let value = FieldValue::Uuid(uuid);
        assert_eq!(format!("{}", value), "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn field_value_uuid_clone_eq() {
        let uuid = Uuid::nil();
        let a = FieldValue::Uuid(uuid);
        let b = a.clone();
        assert_eq!(a, b);
    }
}
