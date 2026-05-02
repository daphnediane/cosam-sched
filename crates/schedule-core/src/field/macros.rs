/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Remaining `macro_rules!` helpers for field declarations and entity builders.
//!
//! Field declarations have moved to the [`schedule_macro::define_field!`]
//! function-like proc-macro (re-exported as [`crate::define_field`]); the
//! historical `stored_field!`, `edge_field!`, and `define_field!`
//! `macro_rules!` macros that previously lived here have been removed.
//!
//! This module now provides:
//! - [`accessor_callbacks!`] - generates read/write/verify callbacks for stored fields
//! - [`define_entity_builder!`] - generates a typed builder struct on top of
//!   [`FieldSet::write_multiple`](crate::field::set::FieldSet) and
//!   [`build_entity`](crate::edit::builder::build_entity) for an entity, given
//!   a list of `with_<setter> => FIELD_STATIC` entries.

// ── Accessor callbacks ─────────────────────────────────────────────────────────

/// Generate read/write/verify callback functions for a stored field accessor.
///
/// This macro generates the boilerplate closures for accessing a field on
/// [`EntityType::InternalData`] using a marker trait for type conversion.
/// Returns a [`FieldCallbacks<E>`] struct containing all three callbacks.
///
/// # Syntax
///
/// ```ignore
/// accessor_callbacks!(<entity_type>, <mode>, <accessor_name>, <MarkerTrait>)
/// ```
///
/// Where `<mode>` is one of:
/// - `required` - field is always present, uses `convert_required`/direct access
/// - `optional` - field is `Option<T>`, uses `convert_optional`/`as_ref().map()`
/// - `with_default` - field has a default value (treated like required)
///
/// # Example
///
/// ```ignore
/// static FIELD_NAME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
///     data: CommonFieldData { /* ... */ },
///     required: true,
///     edge_kind: EdgeKind::NonEdge,
///     crdt_type: AsString::CRDT_TYPE,
///     ..accessor_callbacks!(PanelEntityType, required, name, AsString)
/// };
/// ```
#[macro_export]
macro_rules! accessor_callbacks {
    ($entity:ty, required, $accessor:ident, $marker:ty) => {
        {
            let callbacks = $crate::field::FieldCallbacks {
                read_fn: Some($crate::field::ReadFn::Bare(
                    |d: &<$entity as $crate::entity::EntityType>::InternalData| {
                        Some($crate::value::FieldValue::Single(
                            <$marker as $crate::query::converter::FieldTypeMapping>::to_field_value_item(
                                d.data.$accessor.clone(),
                            ),
                        ))
                    },
                )),
                write_fn: Some($crate::field::WriteFn::Bare(
                    |d: &mut <$entity as $crate::entity::EntityType>::InternalData,
                     v: $crate::value::FieldValue| {
                        d.data.$accessor =
                            $crate::query::converter::convert_required::<$marker>(v)?;
                        Ok(())
                    },
                )),
                verify_fn: None,
            };
            callbacks
        }
    };
    ($entity:ty, optional, $accessor:ident, $marker:ty) => {
        {
            let callbacks = $crate::field::FieldCallbacks {
                read_fn: Some($crate::field::ReadFn::Bare(
                    |d: &<$entity as $crate::entity::EntityType>::InternalData| {
                        d.data.$accessor.as_ref().map(|x| {
                            $crate::value::FieldValue::Single(
                                <$marker as $crate::query::converter::FieldTypeMapping>::to_field_value_item(
                                    x.clone(),
                                ),
                            )
                        })
                    },
                )),
                write_fn: Some($crate::field::WriteFn::Bare(
                    |d: &mut <$entity as $crate::entity::EntityType>::InternalData,
                     v: $crate::value::FieldValue| {
                        d.data.$accessor =
                            $crate::query::converter::convert_optional::<$marker>(v)?;
                        Ok(())
                    },
                )),
                verify_fn: None,
            };
            callbacks
        }
    };
    ($entity:ty, with_default, $accessor:ident, $marker:ty) => {
        {
            let callbacks = $crate::field::FieldCallbacks {
                read_fn: Some($crate::field::ReadFn::Bare(
                    |d: &<$entity as $crate::entity::EntityType>::InternalData| {
                        Some($crate::value::FieldValue::Single(
                            <$marker as $crate::query::converter::FieldTypeMapping>::to_field_value_item(
                                d.data.$accessor.clone(),
                            ),
                        ))
                    },
                )),
                write_fn: Some($crate::field::WriteFn::Bare(
                    |d: &mut <$entity as $crate::entity::EntityType>::InternalData,
                     v: $crate::value::FieldValue| {
                        d.data.$accessor =
                            $crate::query::converter::convert_required::<$marker>(v)?;
                        Ok(())
                    },
                )),
                verify_fn: None,
            };
            callbacks
        }
    };
}

// ── Entity builder ───────────────────────────────────────────────────────────

/// Generate a typed builder struct for an entity on top of
/// [`FieldSet::write_multiple`] and
/// [`build_entity`](crate::edit::builder::build_entity).
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
/// [`EntityBuildable::default_data`](crate::edit::builder::EntityBuildable::default_data)
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
#[macro_export]
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
            updates: ::std::vec::Vec<$crate::field::set::FieldUpdate<$entity>>,
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
                    self.updates.push($crate::field::set::FieldUpdate::set(
                        &$field,
                        value,
                    ));
                    self
                }
            )*

            /// Create a new entity in `schedule`, seeding it via
            /// [`EntityBuildable::default_data`](crate::edit::builder::EntityBuildable::default_data),
            /// applying all queued writes, and running
            /// [`EntityType::validate`](crate::entity::EntityType::validate).
            /// Rolls back on any error.
            pub fn build(
                self,
                schedule: &mut $crate::schedule::Schedule,
            ) -> ::core::result::Result<
                $crate::entity::EntityId<$entity>,
                $crate::edit::builder::BuildError,
            > {
                $crate::edit::builder::build_entity::<$entity>(schedule, self.uuid, self.updates)
            }

            /// Apply the queued writes to an existing entity.  Does not seed
            /// a new entity and does not roll back on error.  The UUID
            /// preference stored on the builder is ignored.
            pub fn apply_to(
                self,
                id: $crate::entity::EntityId<$entity>,
                schedule: &mut $crate::schedule::Schedule,
            ) -> ::core::result::Result<(), $crate::field::set::FieldSetError> {
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
