/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared `macro_rules!` helpers for declaring uniformly-shaped
//! [`FieldDescriptor`](crate::field::FieldDescriptor) statics across entity
//! types.
//!
//! ## Convention
//!
//! Every macro here assumes the caller's `InternalData` struct follows the
//! standard shape used by `PanelInternalData`, `PanelTypeInternalData`, etc.:
//!
//! ```ignore
//! pub struct FooInternalData {
//!     pub id: FooId,
//!     pub data: FooCommonData,
//!     // ...other runtime-only fields...
//! }
//! ```
//!
//! The stored-field macros generate closures that access `d.data.$field`
//! on the `CommonData` sub-struct. Entities that break this convention must
//! hand-write their descriptors.
//!
//! ## What's covered
//!
//! Stored-field macros (uniform read/write against a single `CommonData` field):
//!
//! - [`req_string_field!`] — required, indexed `String` (Scalar CRDT).
//! - [`opt_string_field!`] — `Option<String>` (Scalar CRDT).
//! - [`opt_text_field!`] — `Option<String>` stored, `Text` CRDT + `FieldValue::Text`.
//! - [`bool_field!`] — plain `bool` (Scalar CRDT).
//! - [`opt_i64_field!`] — `Option<i64>` (Scalar CRDT).
//!
//! Edge-stub macros (placeholders until FEATURE-018 wires real edge storage):
//!
//! - [`edge_list_field!`] — read-only `FieldValue::List(Vec::new())`.
//! - [`edge_list_field_rw!`] — read empty list + no-op write.
//! - [`edge_none_field_rw!`] — read empty list + no-op write (singular edge).
//! - [`edge_mutator_field!`] — write-only no-op (for `add_*`/`remove_*`).
//!
//! ## When to hand-write instead
//!
//! Bespoke descriptors — computed fields with custom read/write logic, fields
//! with non-uniform type conversion (e.g. `TimeRange` projections), and real
//! edge mutators once FEATURE-018 lands — stay as plain
//! `FieldDescriptor { ... }` literals at the call site.

use crate::field::MatchPriority;

/// Case-insensitive substring match used by indexed string fields.
///
/// Returns the best matching priority for `query` against `value`:
/// [`MatchPriority::Exact`] for equality, [`MatchPriority::Prefix`] when
/// `value` starts with `query`, [`MatchPriority::Contains`] for substring,
/// or `None` otherwise.
pub(crate) fn substring_match(query: &str, value: &str) -> Option<MatchPriority> {
    let q = query.to_lowercase();
    let v = value.to_lowercase();
    if v == q {
        Some(MatchPriority::Exact)
    } else if v.starts_with(&q) {
        Some(MatchPriority::Prefix)
    } else if v.contains(&q) {
        Some(MatchPriority::Contains)
    } else {
        None
    }
}

// ── Stored-field macros ───────────────────────────────────────────────────────

/// Declare a required indexed `String` field descriptor
/// (`CrdtFieldType::Scalar`, indexed via [`substring_match`]).
macro_rules! req_string_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: true,
                crdt_type: $crate::value::CrdtFieldType::Scalar,
                example: $example,
                read_fn: Some($crate::field::ReadFn::Bare(|d: &$internal| {
                    Some($crate::field_string!(d.data.$field.clone()))
                })),
                write_fn: Some($crate::field::WriteFn::Bare(|d: &mut $internal, v| {
                    d.data.$field = v.into_string()?;
                    Ok(())
                })),
                index_fn: Some(|query, d: &$internal| {
                    $crate::field_macros::substring_match(query, &d.data.$field)
                }),
                verify_fn: None,
            };
    };
}
pub(crate) use req_string_field;

/// Declare an optional `String` field descriptor (`CrdtFieldType::Scalar`,
/// `FieldValue::String` variant, not indexed).
macro_rules! opt_string_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Scalar,
                example: $example,
                read_fn: Some($crate::field::ReadFn::Bare(|d: &$internal| {
                    d.data
                        .$field
                        .as_ref()
                        .map(|s| $crate::field_string!(s.clone()))
                })),
                write_fn: Some($crate::field::WriteFn::Bare(|d: &mut $internal, v| {
                    match v {
                        $crate::value::FieldValue::List(_)
                        | $crate::value::FieldValue::Single($crate::value::FieldValueItem::Text(
                            _,
                        )) => d.data.$field = None,
                        $crate::value::FieldValue::Single(
                            $crate::value::FieldValueItem::String(s),
                        ) => d.data.$field = Some(s),
                        _ => {
                            return Err($crate::value::ConversionError::WrongVariant {
                                expected: "String",
                                got: "other",
                            }
                            .into())
                        }
                    }
                    Ok(())
                })),
                index_fn: None,
                verify_fn: None,
            };
    };
}
pub(crate) use opt_string_field;

/// Declare an optional prose field stored as `Option<String>` but tagged
/// `CrdtFieldType::Text`; read/write go through `FieldValue::Text`.
macro_rules! opt_text_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Text,
                example: $example,
                read_fn: Some($crate::field::ReadFn::Bare(|d: &$internal| {
                    d.data
                        .$field
                        .as_ref()
                        .map(|s| $crate::field_text!(s.clone()))
                })),
                write_fn: Some($crate::field::WriteFn::Bare(|d: &mut $internal, v| {
                    match v {
                        $crate::value::FieldValue::List(_) => d.data.$field = None,
                        $crate::value::FieldValue::Single($crate::value::FieldValueItem::Text(
                            s,
                        )) => d.data.$field = Some(s),
                        $crate::value::FieldValue::Single(
                            $crate::value::FieldValueItem::String(s),
                        ) => d.data.$field = Some(s),
                        _ => {
                            return Err($crate::value::ConversionError::WrongVariant {
                                expected: "Text",
                                got: "other",
                            }
                            .into())
                        }
                    }
                    Ok(())
                })),
                index_fn: None,
                verify_fn: None,
            };
    };
}
pub(crate) use opt_text_field;

/// Declare a plain `bool` field descriptor (`CrdtFieldType::Scalar`).
macro_rules! bool_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Scalar,
                example: $example,
                read_fn: Some($crate::field::ReadFn::Bare(|d: &$internal| {
                    Some($crate::field_boolean!(d.data.$field))
                })),
                write_fn: Some($crate::field::WriteFn::Bare(|d: &mut $internal, v| {
                    d.data.$field = v.into_bool()?;
                    Ok(())
                })),
                index_fn: None,
                verify_fn: None,
            };
    };
}
pub(crate) use bool_field;

/// Declare an optional `i64` field descriptor (`CrdtFieldType::Scalar`,
/// `FieldValue::Integer` variant).
macro_rules! opt_i64_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Scalar,
                example: $example,
                read_fn: Some($crate::field::ReadFn::Bare(|d: &$internal| {
                    d.data.$field.map(|n| $crate::field_integer!(n))
                })),
                write_fn: Some($crate::field::WriteFn::Bare(|d: &mut $internal, v| {
                    match v {
                        $crate::value::FieldValue::List(_)
                        | $crate::value::FieldValue::Single($crate::value::FieldValueItem::Text(
                            _,
                        )) => d.data.$field = None,
                        $crate::value::FieldValue::Single(
                            $crate::value::FieldValueItem::Integer(n),
                        ) => d.data.$field = Some(n),
                        _ => {
                            return Err($crate::value::ConversionError::WrongVariant {
                                expected: "Integer",
                                got: "other",
                            }
                            .into())
                        }
                    }
                    Ok(())
                })),
                index_fn: None,
                verify_fn: None,
            };
    };
}
pub(crate) use opt_i64_field;

// ── Edge-stub macros ──────────────────────────────────────────────────────────
//
// These produce `Derived` descriptors that stand in for edge-backed fields
// until FEATURE-018 wires real edge storage. Reads return an empty list
// (Some(field_value!(empty_list))); writes are accepted silently.

/// Edge-stub list field that is read-only (e.g. `inclusive_presenters`).
macro_rules! edge_list_field {
    (
        $static_name:ident, $entity:ty, $internal:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Derived,
                example: $example,
                read_fn: Some($crate::field::ReadFn::Bare(|_d: &$internal| {
                    Some($crate::field_empty_list!())
                })),
                write_fn: None,
                index_fn: None,
                verify_fn: None,
            };
    };
}
pub(crate) use edge_list_field;

/// Edge-stub list field with a no-op write (e.g. `presenters`, `event_rooms`).
macro_rules! edge_list_field_rw {
    (
        $static_name:ident, $entity:ty, $internal:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Derived,
                example: $example,
                read_fn: Some($crate::field::ReadFn::Bare(|_d: &$internal| {
                    Some($crate::field_empty_list!())
                })),
                write_fn: Some($crate::field::WriteFn::Bare(
                    |_d: &mut $internal, _v| Ok(()),
                )),
                index_fn: None,
                verify_fn: None,
            };
    };
}
pub(crate) use edge_list_field_rw;

/// Edge-stub singular field: read returns empty list, write is a no-op.
/// Used for singular edge relations such as `panel_type`.
macro_rules! edge_none_field_rw {
    (
        $static_name:ident, $entity:ty, $internal:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Derived,
                example: $example,
                read_fn: Some($crate::field::ReadFn::Bare(|_d: &$internal| {
                    Some($crate::field_empty_list!())
                })),
                write_fn: Some($crate::field::WriteFn::Bare(
                    |_d: &mut $internal, _v| Ok(()),
                )),
                index_fn: None,
                verify_fn: None,
            };
    };
}
pub(crate) use edge_none_field_rw;

/// Edge-stub mutator: write-only no-op (used for `add_*` / `remove_*` fields).
macro_rules! edge_mutator_field {
    (
        $static_name:ident, $entity:ty, $internal:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Derived,
                example: $example,
                read_fn: None,
                write_fn: Some($crate::field::WriteFn::Bare(
                    |_d: &mut $internal, _v| Ok(()),
                )),
                index_fn: None,
                verify_fn: None,
            };
    };
}
pub(crate) use edge_mutator_field;
