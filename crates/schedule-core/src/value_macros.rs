/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared `macro_rules!` helpers for creating `FieldValue` instances.
//!
//! Use the `field_value!` macro with a variant identifier, or use the explicit
//! variant macros for direct access. All macros support automatic type conversions
//! via `Into` traits.
//!
//! # Examples
//!
//! ```ignore
//! // Using field_value! with variant identifier
//! field_value!(string, "hello")
//! field_value!(text, "long description")
//! field_value!(integer, 42)
//! field_value!(float, 3.14)
//! field_value!(boolean, true)
//! field_value!(datetime, naive_datetime)
//! field_value!(duration, duration)
//! field_value!(entity_identifier, id)
//!
//! // Empty list
//! field_value!(empty_list)
//! field_empty_list!()
//!
//! // Multiple values → List
//! field_value!(string, "a", "b", "c")
//! field_value!(integer, 1, 2, 3)
//!
//! // Or use explicit variant macros
//! field_string!("hello")
//! field_text!(long_description)
//! field_integer!(42)
//! ```

#[macro_export]
macro_rules! field_value {
    // Empty list
    (empty_list) => {
        $crate::value::FieldValue::List(vec![])
    };

    // Single value with variant identifier
    ($variant:ident, $value:expr) => {
        $crate::field_value_dispatch!($variant, $value)
    };

    // Multiple values with variant identifier → List
    ($variant:ident, $($value:expr),+ $(,)?) => {
        $crate::value::FieldValue::List(vec![$($crate::field_value_dispatch!($variant, $value)),+])
    };
}

/// Internal dispatch macro for variant-identified values.
#[macro_export(local_inner_macros)]
macro_rules! field_value_dispatch {
    (string, $value:expr) => {
        $crate::value::FieldValueItem::String(Into::<String>::into($value))
    };
    (text, $value:expr) => {
        $crate::value::FieldValueItem::Text(Into::<String>::into($value))
    };
    (integer, $value:expr) => {
        $crate::value::FieldValueItem::Integer($value.into())
    };
    (float, $value:expr) => {
        $crate::value::FieldValueItem::Float($value.into())
    };
    (boolean, $value:expr) => {
        $crate::value::FieldValueItem::Boolean($value)
    };
    (datetime, $value:expr) => {
        $crate::value::FieldValueItem::DateTime($value)
    };
    (duration, $value:expr) => {
        $crate::value::FieldValueItem::Duration($value)
    };
    (entity_identifier, $value:expr) => {
        $crate::value::FieldValueItem::EntityIdentifier($value)
    };
}

#[macro_export]
macro_rules! field_empty_list {
    () => {
        $crate::value::FieldValue::List(vec![])
    };
}

#[macro_export]
macro_rules! field_string {
    ($value:expr) => {
        $crate::value::FieldValue::Single($crate::value::FieldValueItem::String(Into::<String>::into($value)))
    };
    ($($value:expr),+ $(,)?) => {
        $crate::value::FieldValue::List(vec![$($crate::value::FieldValueItem::String(Into::<String>::into($value))),+])
    };
}

#[macro_export]
macro_rules! field_text {
    ($value:expr) => {
        $crate::value::FieldValue::Single($crate::value::FieldValueItem::Text(Into::<String>::into($value)))
    };
    ($($value:expr),+ $(,)?) => {
        $crate::value::FieldValue::List(vec![$($crate::value::FieldValueItem::Text(Into::<String>::into($value))),+])
    };
}

#[macro_export]
macro_rules! field_integer {
    ($value:expr) => {
        $crate::value::FieldValue::Single($crate::value::FieldValueItem::Integer($value.into()))
    };
    ($($value:expr),+ $(,)?) => {
        $crate::value::FieldValue::List(vec![$($crate::value::FieldValueItem::Integer($value.into())),+])
    };
}

#[macro_export]
macro_rules! field_float {
    ($value:expr) => {
        $crate::value::FieldValue::Single($crate::value::FieldValueItem::Float($value.into()))
    };
    ($($value:expr),+ $(,)?) => {
        $crate::value::FieldValue::List(vec![$($crate::value::FieldValueItem::Float($value.into())),+])
    };
}

#[macro_export]
macro_rules! field_boolean {
    ($value:expr) => {
        $crate::value::FieldValue::Single($crate::value::FieldValueItem::Boolean($value))
    };
    ($($value:expr),+ $(,)?) => {
        $crate::value::FieldValue::List(vec![$($crate::value::FieldValueItem::Boolean($value)),+])
    };
}

#[macro_export]
macro_rules! field_datetime {
    ($value:expr) => {
        $crate::value::FieldValue::Single($crate::value::FieldValueItem::DateTime($value))
    };
    ($($value:expr),+ $(,)?) => {
        $crate::value::FieldValue::List(vec![$($crate::value::FieldValueItem::DateTime($value)),+])
    };
}

#[macro_export]
macro_rules! field_duration {
    ($value:expr) => {
        $crate::value::FieldValue::Single($crate::value::FieldValueItem::Duration($value))
    };
    ($($value:expr),+ $(,)?) => {
        $crate::value::FieldValue::List(vec![$($crate::value::FieldValueItem::Duration($value)),+])
    };
}

#[macro_export]
macro_rules! field_entity_identifier {
    ($value:expr) => {
        $crate::value::FieldValue::Single($crate::value::FieldValueItem::EntityIdentifier($value))
    };
    ($($value:expr),+ $(,)?) => {
        $crate::value::FieldValue::List(vec![$($crate::value::FieldValueItem::EntityIdentifier($value)),+])
    };
}
