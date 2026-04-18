/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared `macro_rules!` helpers for creating [`FieldValue`](crate::value::FieldValue)
//! instances.
//!
//! Three macros cover all normal cases:
//!
//! - [`field_value!`] — type-deduced single value, `Option`, or `Vec` (via
//!   [`IntoFieldValue`](crate::value::IntoFieldValue)); also accepts `empty_list`.
//! - [`field_text!`] — explicitly creates a `Text` variant (needed because `String`
//!   and `Text` share the same Rust type but have different CRDT semantics).
//! - [`field_empty_list!`] — shorthand for `FieldValue::List(vec![])`.
//!
//! # Examples
//!
//! ```ignore
//! // Type-deduced (IntoFieldValue dispatch)
//! field_value!("hello")               // Single(String("hello"))
//! field_value!(42i64)                 // Single(Integer(42))
//! field_value!(true)                  // Single(Boolean(true))
//! field_value!(dt)                    // Single(DateTime(dt))
//! field_value!(dur)                   // Single(Duration(dur))
//! field_value!(Some("x"))             // Single(String("x"))
//! field_value!(Option::<&str>::None)  // List([])   — clear sentinel
//! field_value!(vec![1i64, 2, 3])      // List([Integer(1), Integer(2), Integer(3)])
//! field_value!(empty_list)            // List([])
//!
//! // Text variant (must name explicitly — same Rust type as String)
//! field_text!("long description")     // Single(Text("long description"))
//!
//! // Empty list shorthand
//! field_empty_list!()                 // List([])
//! ```

#[macro_export]
macro_rules! field_value {
    // Empty list
    (empty_list) => {
        $crate::value::FieldValue::List(vec![])
    };

    // Type-deduced — must be last so the `empty_list` arm matches first
    ($e:expr) => {
        $crate::value::IntoFieldValue::into_field_value($e)
    };
}

#[macro_export]
macro_rules! field_empty_list {
    () => {
        $crate::value::FieldValue::List(vec![])
    };
}

/// Creates a `FieldValue::Single(FieldValueItem::Text(...))`.
///
/// Use this instead of `field_value!` when the field uses `CrdtFieldType::Text`
/// (long prose routed to RGA CRDT storage). The Rust type `String` alone is
/// insufficient to distinguish `String` from `Text`.
#[macro_export]
macro_rules! field_text {
    ($value:expr) => {
        $crate::value::FieldValue::Single($crate::value::FieldValueItem::Text(
            Into::<String>::into($value),
        ))
    };
}
