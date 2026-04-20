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
//! Two consolidated macros cover the uniform cases:
//!
//! - [`stored_field!`] — scalar field backed by a `CommonData` slot.
//!   Two arms: `required` (stored as `T`) and `optional` (stored as `Option<T>`).
//!   The value type is chosen by passing an [`AsString`](crate::converter::AsString) /
//!   [`AsBoolean`](crate::converter::AsBoolean) /
//!   [`AsInteger`](crate::converter::AsInteger) /
//!   [`AsText`](crate::converter::AsText) (etc.) marker via `as:`.  The marker
//!   supplies both the [`FieldTypeItem`](crate::value::FieldTypeItem) tag
//!   (`FIELD_TYPE_ITEM`) and the CRDT annotation (`CRDT_TYPE`); conversion in
//!   both directions is delegated to the marker so the macro itself stays free
//!   of per-type logic.
//!
//! - [`edge_field!`] — edge-backed field using `Schedule::edges_from` / `edges_to`.
//!   Variants selected via `mode:`: `ro` (read-only), `rw` (read/write forward),
//!   `one` (read/write 0-or-1 forward), `rw_to` (read/write reverse — uses
//!   `source:` instead of `target:`), `add` (write-only per-item add), `remove`
//!   (write-only per-item remove).
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

// ── Stored-field macro ───────────────────────────────────────────────────────

/// Declare a stored-field [`FieldDescriptor`](crate::field::FieldDescriptor)
/// backed by a plain `CommonData` field (read/write go through `d.data.$field`).
///
/// Conversion between [`FieldValue`](crate::value::FieldValue) and the Rust
/// storage type is delegated to a [`FieldTypeMapping`](crate::converter::FieldTypeMapping)
/// marker (e.g. [`AsString`](crate::converter::AsString),
/// [`AsBoolean`](crate::converter::AsBoolean),
/// [`AsInteger`](crate::converter::AsInteger),
/// [`AsText`](crate::converter::AsText)).  The marker supplies both the
/// [`FieldTypeItem`](crate::value::FieldTypeItem) tag (via
/// `FIELD_TYPE_ITEM`) and the CRDT annotation (via `CRDT_TYPE`), and its
/// `from_field_value_item` / `to_field_value_item` methods drive the generated
/// read/write closures via [`convert_required`](crate::converter::convert_required)
/// and [`convert_optional`](crate::converter::convert_optional).
///
/// Three arms cover the common storage/required combinations:
///
/// | keyword        | storage     | `required` flag | typical use |
/// |----------------|-------------|-----------------|-------------|
/// | `required`     | `T`         | `true`          | mandatory scalar (e.g. name) |
/// | `optional`     | `Option<T>` | `false`         | optional scalar |
/// | `with_default` | `T`         | `false`         | scalar with implicit default (e.g. `bool`) |
///
/// Example:
///
/// ```ignore
/// stored_field!(FIELD_PREFIX, PanelTypeEntityType, prefix,
///     required, as: AsString,
///     name: "prefix", display: "Prefix",
///     desc: "Two-letter Uniq ID prefix for panels of this type.",
///     aliases: &["uniq_id_prefix"], example: "GP", order: 0);
///
/// stored_field!(FIELD_BW, PanelTypeEntityType, bw,
///     optional, as: AsString,
///     name: "bw", display: "BW",
///     desc: "Black & white variant of the color.",
///     aliases: &[], example: "#000", order: 110);
/// ```
macro_rules! stored_field {
    // ── required: stored as T ─────────────────────────────────────────
    (
        $static_name:ident, $entity:ty, $field:ident,
        required, as: $marker:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr $(,)?
    ) => {
        #[doc = concat!("**", $display, "** \u{2014} ", $desc)]
        #[doc = ""]
        #[doc = concat!("Required scalar field. Example: `", $example, "`.")]
        pub static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: true,
                crdt_type: <$marker as $crate::converter::FieldTypeMapping>::CRDT_TYPE,
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::Single,
                    <$marker as $crate::converter::FieldTypeMapping>::FIELD_TYPE_ITEM,
                ),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Bare(
                    |d: &<$entity as $crate::entity::EntityType>::InternalData| {
                        Some($crate::value::FieldValue::Single(
                            <$marker as $crate::converter::FieldTypeMapping>::to_field_value_item(
                                d.data.$field.clone(),
                            ),
                        ))
                    },
                )),
                write_fn: Some($crate::field::WriteFn::Bare(
                    |d: &mut <$entity as $crate::entity::EntityType>::InternalData,
                     v: $crate::value::FieldValue| {
                        d.data.$field = $crate::converter::convert_required::<$marker>(v)?;
                        Ok(())
                    },
                )),
                verify_fn: None,
            };
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };

    // ── with_default: stored as T, but not flagged as required ───────
    // (for fields that always have a value but default naturally, e.g. bool)
    (
        $static_name:ident, $entity:ty, $field:ident,
        with_default, as: $marker:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr $(,)?
    ) => {
        #[doc = concat!("**", $display, "** \u{2014} ", $desc)]
        #[doc = ""]
        #[doc = concat!("Scalar field with implicit default. Example: `", $example, "`.")]
        pub static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: <$marker as $crate::converter::FieldTypeMapping>::CRDT_TYPE,
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::Single,
                    <$marker as $crate::converter::FieldTypeMapping>::FIELD_TYPE_ITEM,
                ),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Bare(
                    |d: &<$entity as $crate::entity::EntityType>::InternalData| {
                        Some($crate::value::FieldValue::Single(
                            <$marker as $crate::converter::FieldTypeMapping>::to_field_value_item(
                                d.data.$field.clone(),
                            ),
                        ))
                    },
                )),
                write_fn: Some($crate::field::WriteFn::Bare(
                    |d: &mut <$entity as $crate::entity::EntityType>::InternalData,
                     v: $crate::value::FieldValue| {
                        d.data.$field = $crate::converter::convert_required::<$marker>(v)?;
                        Ok(())
                    },
                )),
                verify_fn: None,
            };
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };

    // ── optional: stored as Option<T> ────────────────────────────────
    (
        $static_name:ident, $entity:ty, $field:ident,
        optional, as: $marker:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr $(,)?
    ) => {
        #[doc = concat!("**", $display, "** \u{2014} ", $desc)]
        #[doc = ""]
        #[doc = concat!("Optional scalar field. Example: `", $example, "`.")]
        pub static $static_name: $crate::field::FieldDescriptor<$entity> =
            $crate::field::FieldDescriptor {
                name: $name,
                display: $display,
                description: $desc,
                aliases: $aliases,
                required: false,
                crdt_type: <$marker as $crate::converter::FieldTypeMapping>::CRDT_TYPE,
                field_type: $crate::value::FieldType(
                    $crate::value::FieldCardinality::Optional,
                    <$marker as $crate::converter::FieldTypeMapping>::FIELD_TYPE_ITEM,
                ),
                example: $example,
                order: $order,
                read_fn: Some($crate::field::ReadFn::Bare(
                    |d: &<$entity as $crate::entity::EntityType>::InternalData| {
                        d.data.$field.as_ref().map(|x| {
                            $crate::value::FieldValue::Single(
                                <$marker as $crate::converter::FieldTypeMapping>::to_field_value_item(
                                    x.clone(),
                                ),
                            )
                        })
                    },
                )),
                write_fn: Some($crate::field::WriteFn::Bare(
                    |d: &mut <$entity as $crate::entity::EntityType>::InternalData,
                     v: $crate::value::FieldValue| {
                        d.data.$field = $crate::converter::convert_optional::<$marker>(v)?;
                        Ok(())
                    },
                )),
                verify_fn: None,
            };
        inventory::submit! { $crate::entity::CollectedField::<$entity>(&$static_name) }
    };
}
pub(crate) use stored_field;

// ── Edge-field macro ──────────────────────────────────────────────────────────
//
// Produces `Derived` descriptors for edge-backed fields using
// `Schedule::edges_from` / `edge_set` / `edge_add` / `edge_remove`.
// All use `ReadFn::Schedule` or `WriteFn::Schedule` so the descriptor does not
// need `InternalData` access.

/// Declare an edge-backed [`FieldDescriptor`](crate::field::FieldDescriptor).
///
/// The `mode:` token selects read/write semantics and direction:
///
/// | `mode:` | Neighbor kw | Direction | Read | Write |
/// |---------|-------------|-----------|------|-------|
/// | `ro`    | `target:`   | forward   | `edges_from`   | — |
/// | `rw`    | `target:`   | forward   | `edges_from`   | `edge_set` (replace) |
/// | `one`   | `target:`   | forward   | `edges_from`   | `edge_set` (0-or-1) |
/// | `rw_to` | `source:`   | reverse   | `edges_to`     | `edge_set_to` |
/// | `add`   | `target:`   | forward   | —              | `edge_add` (per item) |
/// | `remove`| `target:`   | forward   | —              | `edge_remove` (per item) |
///
/// # Example
///
/// ```ignore
/// edge_field!(FIELD_PRESENTERS, PanelEntityType, mode: rw, target: PresenterEntityType,
///     name: "presenters", display: "Presenters",
///     desc: "Presenters for this panel.",
///     aliases: &["presenter"], example: "[]", order: 2700);
/// ```
macro_rules! edge_field {
    // ── mode: ro ──────────────────────────────────────────────────────
    (
        $static_name:ident, $entity:ty, mode: ro, target: $target_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr $(,)?
    ) => {
        #[doc = concat!("**", $display, "** \u{2014} ", $desc)]
        #[doc = ""]
        #[doc = concat!("Type: read-only `List<EntityId<", stringify!($target_entity), ">>` (edge-backed).")]
        pub static $static_name: $crate::field::FieldDescriptor<$entity> =
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

    // ── mode: rw ──────────────────────────────────────────────────────
    (
        $static_name:ident, $entity:ty, mode: rw, target: $target_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr $(,)?
    ) => {
        #[doc = concat!("**", $display, "** \u{2014} ", $desc)]
        #[doc = ""]
        #[doc = concat!("Type: read/write `List<EntityId<", stringify!($target_entity), ">>` (edge-backed, replaces on write).")]
        pub static $static_name: $crate::field::FieldDescriptor<$entity> =
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

    // ── mode: one ─────────────────────────────────────────────────────
    // Structurally identical to `rw`; the 0-or-1 constraint is semantic.
    (
        $static_name:ident, $entity:ty, mode: one, target: $target_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr $(,)?
    ) => {
        #[doc = concat!("**", $display, "** \u{2014} ", $desc)]
        #[doc = ""]
        #[doc = concat!("Type: read/write `Option<EntityId<", stringify!($target_entity), ">>` (0-or-1 edge).")]
        pub static $static_name: $crate::field::FieldDescriptor<$entity> =
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

    // ── mode: rw_to ───────────────────────────────────────────────────
    (
        $static_name:ident, $entity:ty, mode: rw_to, source: $source_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr $(,)?
    ) => {
        #[doc = concat!("**", $display, "** \u{2014} ", $desc)]
        #[doc = ""]
        #[doc = concat!("Type: read/write `List<EntityId<", stringify!($source_entity), ">>` (reverse edges; sources pointing at this entity).")]
        pub static $static_name: $crate::field::FieldDescriptor<$entity> =
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

    // ── mode: add ─────────────────────────────────────────────────────
    (
        $static_name:ident, $entity:ty, mode: add, target: $target_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr $(,)?
    ) => {
        #[doc = concat!("**", $display, "** \u{2014} ", $desc)]
        #[doc = ""]
        #[doc = concat!("Type: write-only `List<EntityId<", stringify!($target_entity), ">>` (each item is added as a new edge).")]
        pub static $static_name: $crate::field::FieldDescriptor<$entity> =
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

    // ── mode: remove ──────────────────────────────────────────────────
    (
        $static_name:ident, $entity:ty, mode: remove, target: $target_entity:ty,
        name: $name:literal, display: $display:literal, desc: $desc:literal,
        aliases: $aliases:expr, example: $example:literal,
        order: $order:expr $(,)?
    ) => {
        #[doc = concat!("**", $display, "** \u{2014} ", $desc)]
        #[doc = ""]
        #[doc = concat!("Type: write-only `List<EntityId<", stringify!($target_entity), ">>` (each item is removed as an edge).")]
        pub static $static_name: $crate::field::FieldDescriptor<$entity> =
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
pub(crate) use edge_field;

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
/// [`FieldSet::write_multiple`] and
/// [`build_entity`](crate::builder::build_entity).
///
/// The call site lists one `with_<setter> => &FIELD_STATIC` entry per field
/// that should be settable through the builder.  Each setter takes any
/// [`IntoFieldValue`](crate::value::IntoFieldValue)-typed value, so callers
/// pass native Rust types (`&str`, `bool`, `Option<T>`, `Vec<T>`, etc.)
/// without constructing `FieldValue` by hand.
///
/// The field descriptor statics must be in scope (they are now `pub`, so they
/// can be re-exported if needed).
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
            /// All setters accept any [`IntoFieldValue`](crate::value::IntoFieldValue) type.
            ///
            /// Conversion or validation errors surface at [`Self::build`] or
            /// [`Self::apply_to`] time.
            ///
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
                #[doc = concat!("Writes to [`", stringify!($field), "`].")]
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
