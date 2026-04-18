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
//! `FieldDescriptor { ... }` literals wrapped in [`define_field!`].
//!
//! ## Hand-written descriptor registration
//!
//! Use [`define_field!`] to declare any hand-written `FieldDescriptor` static;
//! it bundles the `static` declaration with the required `inventory::submit!`
//! call so the field is never accidentally omitted from the registry.

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
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: true,
                crdt_type: $crate::value::CrdtFieldType::Scalar,
                field_type: $crate::value::FieldType::Single($crate::value::FieldTypeItem::String),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Bare(|d: &$internal| {
                    Some($crate::field_value!(d.data.$field.clone()))
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
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use req_string_field;

/// Declare an optional `String` field descriptor (`CrdtFieldType::Scalar`,
/// `FieldValue::String` variant, not indexed).
macro_rules! opt_string_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Scalar,
                field_type: $crate::value::FieldType::Optional(
                    $crate::value::FieldTypeItem::String,
                ),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Bare(|d: &$internal| {
                    d.data
                        .$field
                        .as_ref()
                        .map(|s| $crate::field_value!(s.clone()))
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
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use opt_string_field;

/// Declare an optional prose field stored as `Option<String>` but tagged
/// `CrdtFieldType::Text`; read/write go through `FieldValue::Text`.
macro_rules! opt_text_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Text,
                field_type: $crate::value::FieldType::Optional($crate::value::FieldTypeItem::Text),
                example: $example,
                order: $order,
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
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use opt_text_field;

/// Declare a plain `bool` field descriptor (`CrdtFieldType::Scalar`).
macro_rules! bool_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Scalar,
                field_type: $crate::value::FieldType::Single($crate::value::FieldTypeItem::Boolean),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Bare(|d: &$internal| {
                    Some($crate::field_value!(d.data.$field))
                })),
                write_fn: Some($crate::field::WriteFn::Bare(|d: &mut $internal, v| {
                    d.data.$field = v.into_bool()?;
                    Ok(())
                })),
                index_fn: None,
                verify_fn: None,
            };
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use bool_field;

/// Declare an optional `i64` field descriptor (`CrdtFieldType::Scalar`,
/// `FieldValue::Integer` variant).
macro_rules! opt_i64_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, $field:ident,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Scalar,
                field_type: $crate::value::FieldType::Optional(
                    $crate::value::FieldTypeItem::Integer,
                ),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Bare(|d: &$internal| {
                    d.data.$field.map(|n| $crate::field_value!(n))
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
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use opt_i64_field;

// ── Edge macros ───────────────────────────────────────────────────────────────
//
// These produce `Derived` descriptors for edge-backed fields backed by
// `Schedule::edges_from` / `edge_set` / `edge_add` / `edge_remove`.
// All use `ReadFn::Schedule` or `WriteFn::Schedule` so the field descriptor
// can call the generic edge methods without needing `InternalData` access.

/// Read-only edge list field — reads via `Schedule::edges_from::<L, R>`.
///
/// Used for reverse/computed edges such as `inclusive_presenters`, `event_rooms`
/// on `HotelRoom`, and `panels` on `EventRoom` and `PanelType`.
macro_rules! edge_list_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, target: $target_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Derived,
                field_type: $crate::value::FieldType::List(
                    $crate::value::FieldTypeItem::EntityIdentifier(
                        <$target_entity as $crate::entity::EntityType>::TYPE_NAME,
                    ),
                ),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Schedule(
                    |sched: &$crate::schedule::Schedule, id: $crate::entity::EntityId<$entity>| {
                        let ids = sched.edges_from::<$entity, $target_entity>(id);
                        Some($crate::schedule::entity_ids_to_field_value(ids))
                    },
                )),
                write_fn: None,
                index_fn: None,
                verify_fn: None,
            };
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use edge_list_field;

/// Read-write edge list field.
///
/// Read: `edges_from::<L, R>` — forward neighbors.
/// Write: `edge_set::<L, R>` — replace all R-type neighbors.
macro_rules! edge_list_field_rw {
    (
        $static_name:ident, $entity:ty, $internal:ty, target: $target_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Derived,
                field_type: $crate::value::FieldType::List(
                    $crate::value::FieldTypeItem::EntityIdentifier(
                        <$target_entity as $crate::entity::EntityType>::TYPE_NAME,
                    ),
                ),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Schedule(
                    |sched: &$crate::schedule::Schedule, id: $crate::entity::EntityId<$entity>| {
                        let ids = sched.edges_from::<$entity, $target_entity>(id);
                        Some($crate::schedule::entity_ids_to_field_value(ids))
                    },
                )),
                write_fn: Some($crate::field::WriteFn::Schedule(
                    |sched: &mut $crate::schedule::Schedule,
                     id: $crate::entity::EntityId<$entity>,
                     val: $crate::value::FieldValue| {
                        let ids =
                            $crate::schedule::field_value_to_entity_ids::<$target_entity>(val)?;
                        sched.edge_set::<$entity, $target_entity>(id, ids);
                        Ok(())
                    },
                )),
                index_fn: None,
                verify_fn: None,
            };
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use edge_list_field_rw;

/// Read-write edge list field for the reverse (to) direction.
///
/// Read: `edges_to::<L, R>` — sources pointing to this entity.
/// Write: `edge_set_to::<L, R>` — replace all L-type sources.
///
/// Used for `members` on `Presenter` (groups list the members that have a
/// forward edge TO them).
macro_rules! edge_list_field_to_rw {
    (
        $static_name:ident, $entity:ty, $internal:ty, source: $source_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Derived,
                field_type: $crate::value::FieldType::List(
                    $crate::value::FieldTypeItem::EntityIdentifier(
                        <$source_entity as $crate::entity::EntityType>::TYPE_NAME,
                    ),
                ),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Schedule(
                    |sched: &$crate::schedule::Schedule, id: $crate::entity::EntityId<$entity>| {
                        let ids = sched.edges_to::<$source_entity, $entity>(id);
                        Some($crate::schedule::entity_ids_to_field_value(ids))
                    },
                )),
                write_fn: Some($crate::field::WriteFn::Schedule(
                    |sched: &mut $crate::schedule::Schedule,
                     id: $crate::entity::EntityId<$entity>,
                     val: $crate::value::FieldValue| {
                        let ids =
                            $crate::schedule::field_value_to_entity_ids::<$source_entity>(val)?;
                        sched.edge_set_to::<$source_entity, $entity>(id, ids);
                        Ok(())
                    },
                )),
                index_fn: None,
                verify_fn: None,
            };
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use edge_list_field_to_rw;

/// Write-only edge-add mutator field (for `add_*` fields).
///
/// Each item in the written `FieldValue::List` is added as a new edge.
macro_rules! edge_add_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, target: $target_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Derived,
                field_type: $crate::value::FieldType::List(
                    $crate::value::FieldTypeItem::EntityIdentifier(
                        <$target_entity as $crate::entity::EntityType>::TYPE_NAME,
                    ),
                ),
                example: $example,
                order: $order,
                read_fn: None,
                write_fn: Some($crate::field::WriteFn::Schedule(
                    |sched: &mut $crate::schedule::Schedule,
                     id: $crate::entity::EntityId<$entity>,
                     val: $crate::value::FieldValue| {
                        let ids =
                            $crate::schedule::field_value_to_entity_ids::<$target_entity>(val)?;
                        for r in ids {
                            sched.edge_add::<$entity, $target_entity>(id, r);
                        }
                        Ok(())
                    },
                )),
                index_fn: None,
                verify_fn: None,
            };
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use edge_add_field;

/// Write-only edge-remove mutator field (for `remove_*` fields).
///
/// Each item in the written `FieldValue::List` is removed as an edge.
macro_rules! edge_remove_field {
    (
        $static_name:ident, $entity:ty, $internal:ty, target: $target_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Derived,
                field_type: $crate::value::FieldType::List(
                    $crate::value::FieldTypeItem::EntityIdentifier(
                        <$target_entity as $crate::entity::EntityType>::TYPE_NAME,
                    ),
                ),
                example: $example,
                order: $order,
                read_fn: None,
                write_fn: Some($crate::field::WriteFn::Schedule(
                    |sched: &mut $crate::schedule::Schedule,
                     id: $crate::entity::EntityId<$entity>,
                     val: $crate::value::FieldValue| {
                        let ids =
                            $crate::schedule::field_value_to_entity_ids::<$target_entity>(val)?;
                        for r in ids {
                            sched.edge_remove::<$entity, $target_entity>(id, r);
                        }
                        Ok(())
                    },
                )),
                index_fn: None,
                verify_fn: None,
            };
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use edge_remove_field;

/// Alias for `edge_list_field_rw!` — used for singular (0-or-1) edge fields.
///
/// Structurally identical to `edge_list_field_rw!`; the 0-or-1 constraint is
/// semantic, not enforced at the macro level.
macro_rules! edge_none_field_rw {
    (
        $static_name:ident, $entity:ty, $internal:ty, target: $target_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr
    ) => {
        static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: $crate::value::CrdtFieldType::Derived,
                field_type: $crate::value::FieldType::List(
                    $crate::value::FieldTypeItem::EntityIdentifier(
                        <$target_entity as $crate::entity::EntityType>::TYPE_NAME,
                    ),
                ),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Schedule(
                    |sched: &$crate::schedule::Schedule, id: $crate::entity::EntityId<$entity>| {
                        let ids = sched.edges_from::<$entity, $target_entity>(id);
                        Some($crate::schedule::entity_ids_to_field_value(ids))
                    },
                )),
                write_fn: Some($crate::field::WriteFn::Schedule(
                    |sched: &mut $crate::schedule::Schedule,
                     id: $crate::entity::EntityId<$entity>,
                     val: $crate::value::FieldValue| {
                        let ids =
                            $crate::schedule::field_value_to_entity_ids::<$target_entity>(val)?;
                        sched.edge_set::<$entity, $target_entity>(id, ids);
                        Ok(())
                    },
                )),
                index_fn: None,
                verify_fn: None,
            };
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use edge_none_field_rw;

// ── Hand-written descriptor registration ─────────────────────────────────────

/// Declare a hand-written [`FieldDescriptor`](crate::field::FieldDescriptor)
/// static and register it with the `inventory` registry in one step.
///
/// Wrap any bespoke `FieldDescriptor { ... }` literal that doesn't fit the
/// stored-field macros. The entity type `E` is inferred from the static's
/// declared type — no second type argument is needed at the call site.
///
/// # Example
///
/// ```ignore
/// define_field!(
///     /// Optional long name, indexed for name-based search.
///     static FIELD_LONG_NAME: FieldDescriptor<EventRoomEntityType> = FieldDescriptor {
///         name: "long_name",
///         // ...
///     }
/// );
/// ```
macro_rules! define_field {
    (
        $(#[$attr:meta])*
        $vis:vis static $static_name:ident : $ty:ty = $init:expr
    ) => {
        $(#[$attr])*
        $vis static $static_name: $ty = $init;
        inventory::submit! {
            $crate::entity::CollectedField::<_>(&$static_name)
        }
    };
}
pub(crate) use define_field;
