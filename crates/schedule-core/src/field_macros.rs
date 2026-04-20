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
//! Edge-backed field macros (use `Schedule::edges_from` / `edges_to`):
//!
//! - [`edge_list_field!`] — read-only list of neighbors via `edges_from`.
//! - [`edge_list_field_rw!`] — read + write (forward direction, `edge_set`).
//! - [`edge_list_field_to_rw!`] — read + write (reverse direction, `edge_set_to`).
//! - [`edge_none_field_rw!`] — read + write (forward, `edge_set`; singular edge).
//! - [`edge_add_field!`] — write-only, `edge_add` for each item.
//! - [`edge_remove_field!`] — write-only, `edge_remove` for each item.
//!
//! ## When to hand-write instead
//!
//! Bespoke descriptors — computed fields with custom read/write logic, fields
//! with non-uniform type conversion (e.g. `TimeRange` projections), BFS
//! transitive-closure fields (inclusive_groups, inclusive_panels, etc.) — stay
//! as plain `FieldDescriptor { ... }` literals wrapped in [`define_field!`].
//!
//! ## Hand-written descriptor registration
//!
//! Use [`define_field!`] to declare any hand-written `FieldDescriptor` static;
//! it bundles the `static` declaration with the required `inventory::submit!`
//! call so the field is never accidentally omitted from the registry.

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
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::Single,
                    $crate::value::FieldTypeItem::String,
                ),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Bare(|d: &$internal| {
                    Some($crate::field_value!(d.data.$field.clone()))
                })),
                write_fn: Some($crate::field::WriteFn::Bare(|d: &mut $internal, v| {
                    d.data.$field = v.into_string()?;
                    Ok(())
                })),
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
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::Optional,
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
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::Optional,
                    $crate::value::FieldTypeItem::Text,
                ),
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
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::Single,
                    $crate::value::FieldTypeItem::Boolean,
                ),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Bare(|d: &$internal| {
                    Some($crate::field_value!(d.data.$field))
                })),
                write_fn: Some($crate::field::WriteFn::Bare(|d: &mut $internal, v| {
                    d.data.$field = v.into_bool()?;
                    Ok(())
                })),
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
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::Optional,
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
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::List,
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
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::List,
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
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::List,
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
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::List,
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
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::List,
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
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::List,
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

// ── Entity builder ───────────────────────────────────────────────────────────

/// Generate a typed builder struct for an entity on top of
/// [`FieldSet::write_multiple`] (FEATURE-046) and
/// [`build_entity`](crate::builder::build_entity) (FEATURE-017).
///
/// The call site lists one `with_<setter> => &FIELD_STATIC` entry per field
/// that should be settable through the builder.  Each setter takes any
/// [`IntoFieldValue`](crate::value::IntoFieldValue)-typed value, so callers
/// pass native Rust types (`&str`, `bool`, `Option<T>`, `Vec<T>`, etc.)
/// without constructing `FieldValue` by hand.
///
/// Must be invoked in the same module as the `FIELD_*` statics it references,
/// since those statics are module-private.
///
/// # Generated API
///
/// ```ignore
/// pub struct FooBuilder { .. }
///
/// impl FooBuilder {
///     pub fn new() -> Self;
///     pub fn with_uuid_preference(self, p: UuidPreference) -> Self;
///     // one setter per listed field:
///     pub fn with_<setter>(self, v: impl IntoFieldValue) -> Self;
///     // terminal operations:
///     pub fn build(self, schedule: &mut Schedule) -> Result<EntityId<Foo>, BuildError>;
///     pub fn apply_to(self, id: EntityId<Foo>, schedule: &mut Schedule)
///         -> Result<(), FieldSetError>;
/// }
///
/// impl Default for FooBuilder { fn default() -> Self { Self::new() } }
/// ```
///
/// `build` creates a new entity by seeding via
/// [`EntityBuildable::default_data`](crate::builder::EntityBuildable::default_data)
/// and applying the queued writes through `write_multiple`, with rollback on
/// any failure.  `apply_to` reuses the same queue against an existing
/// entity without insertion or rollback.
///
/// Each entry is a setter identifier and a **path** to a `FieldDescriptor`
/// static.  The macro inserts the `&` when resolving the descriptor, so do
/// not write one.  Caller-supplied `///` doc comments on each entry are
/// forwarded onto the generated setter; the macro appends a line pointing
/// back at the underlying `FIELD_*` static.
///
/// # Example
///
/// ```ignore
/// define_entity_builder! {
///     /// Typed builder for `PanelType` entities.
///     PanelTypeBuilder for PanelTypeEntityType {
///         /// Set the two-letter Uniq ID prefix (e.g. `"GP"`).
///         with_prefix      => FIELD_PREFIX,
///         /// Set the human-readable kind name.
///         with_panel_kind  => FIELD_PANEL_KIND,
///         /// Set the CSS color for color-mode rendering.
///         with_color       => FIELD_COLOR,
///     }
/// }
/// ```
macro_rules! define_entity_builder {
    (
        $(#[$attr:meta])*
        $builder:ident for $entity:ty {
            $(
                $(#[$setter_attr:meta])*
                $setter:ident => $field:path
            ),* $(,)?
        }
    ) => {
        $(#[$attr])*
        pub struct $builder {
            uuid: $crate::entity::UuidPreference,
            updates: ::std::vec::Vec<(
                $crate::field_set::FieldRef<$entity>,
                $crate::value::FieldValue,
            )>,
        }

        impl $builder {
            /// Start a fresh builder.  The default UUID preference is
            /// [`UuidPreference::GenerateNew`](crate::entity::UuidPreference::GenerateNew).
            #[must_use]
            pub fn new() -> Self {
                Self {
                    uuid: $crate::entity::UuidPreference::GenerateNew,
                    updates: ::std::vec::Vec::new(),
                }
            }

            /// Override the UUID preference used at [`Self::build`] time.
            #[must_use]
            pub fn with_uuid_preference(
                mut self,
                preference: $crate::entity::UuidPreference,
            ) -> Self {
                self.uuid = preference;
                self
            }

            $(
                $(#[$setter_attr])*
                #[doc = ""]
                #[doc = concat!(
                    "Writes to the `",
                    stringify!($field),
                    "` field descriptor.  Accepts any \
                     [`IntoFieldValue`](crate::value::IntoFieldValue) type; \
                     conversion or validation errors surface at \
                     [`Self::build`] / [`Self::apply_to`] time."
                )]
                #[must_use]
                pub fn $setter(
                    mut self,
                    value: impl $crate::value::IntoFieldValue,
                ) -> Self {
                    self.updates.push((
                        $crate::field_set::FieldRef::Descriptor(&$field),
                        $crate::value::IntoFieldValue::into_field_value(value),
                    ));
                    self
                }
            )*

            /// Create a new entity in `schedule`, seeding it via
            /// [`EntityBuildable::default_data`](crate::builder::EntityBuildable::default_data),
            /// applying all queued writes, and running
            /// [`EntityType::validate`](crate::entity::EntityType::validate).
            /// Rolls back on any error.
            pub fn build(
                self,
                schedule: &mut $crate::schedule::Schedule,
            ) -> ::core::result::Result<
                $crate::entity::EntityId<$entity>,
                $crate::builder::BuildError,
            > {
                $crate::builder::build_entity::<$entity>(schedule, self.uuid, self.updates)
            }

            /// Apply the queued writes to an existing entity.  Does not seed
            /// a new entity and does not roll back on error.  The UUID
            /// preference stored on the builder is ignored.
            pub fn apply_to(
                self,
                id: $crate::entity::EntityId<$entity>,
                schedule: &mut $crate::schedule::Schedule,
            ) -> ::core::result::Result<(), $crate::field_set::FieldSetError> {
                <$entity as $crate::entity::EntityType>::field_set()
                    .write_multiple(id, schedule, &self.updates)
            }
        }

        impl ::core::default::Default for $builder {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}
pub(crate) use define_entity_builder;
