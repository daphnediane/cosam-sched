/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Proc-macro support crate for `schedule-core`.
//!
//! Provides function-like proc-macros for generating field descriptors:
//!
//! - [`accessor_field_properties`] — generates `(CommonFieldData, FieldCallbacks)`
//!   tuple for accessor-based fields without custom callbacks.
//!
//! - [`callback_field_properties`] — generates `(CommonFieldData, FieldCallbacks)`
//!   tuple for fields with custom callbacks (closures or enum variants).

mod callback_input;
mod callback_output;
mod common_input;
mod common_output;
mod stored_input;
mod stored_output;

use proc_macro::TokenStream;

/// Generate `CommonFieldData` and `FieldCallbacks` for an accessor-based field.
///
/// Returns a `(CommonFieldData, FieldCallbacks<E>)` tuple containing both
/// the field metadata and the read/write/add/remove callbacks inferred from
/// the field's cardinality and item type. The caller constructs the
/// `FieldDescriptor` to control `required`, `edge_kind`, and other
/// descriptor-level properties.
///
/// # Syntax
///
/// ```ignore
/// let (data, cb) = accessor_field_properties! {
///     EntityType,
///     accessor_name,
///     name: "field_name",
///     display: "Field Name",
///     description: "Description text",
///     aliases: &["alias1", "alias2"],
///     cardinality: Single | Optional | List,
///     item: String | Boolean | Integer | Float | DateTime | Duration | Text,
///     example: "example value",
///     order: 100,
///     [required: true | false,]
/// };
/// ```
///
/// The `required` parameter is optional and defaults to `true` for `Single`
/// cardinality and `false` for `Optional` or `List`.
///
/// # Example
///
/// ```ignore
/// pub static FIELD_PREFIX: FieldDescriptor<PanelTypeEntityType> = {
///     let (data, cb) = accessor_field_properties! {
///         PanelTypeEntityType,
///         prefix,
///         name: "prefix",
///         display: "Prefix",
///         description: "Two-letter Uniq ID prefix for panels of this type.",
///         aliases: &["uniq_id_prefix"],
///         cardinality: Single,
///         item: String,
///         example: "GP",
///         order: 0,
///     };
///     FieldDescriptor {
///         data,
///         required: true,
///         cb,
///     }
/// };
/// inventory::submit! { CollectedNamedField(&FIELD_PREFIX) }
/// ```
#[proc_macro]
pub fn accessor_field_properties(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as stored_input::StoredInput);
    match stored_output::expand(&parsed) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Generate `CommonFieldData` and `FieldCallbacks` with custom callbacks.
///
/// Returns a `(CommonFieldData, FieldCallbacks<E>)` tuple containing both
/// the field metadata and the specified callbacks. The caller constructs the
/// `FieldDescriptor` or `EdgeDescriptor` to control other descriptor-level
/// properties.
///
/// This macro is useful when you need custom callback behavior that differs
/// from the auto-generated behavior of `accessor_field_properties!`.
///
/// # Syntax
///
/// ```ignore
/// let (data, cb) = callback_field_properties! {
///     EntityType,
///     name: "field_name",
///     display: "Field Name",
///     description: "Description text",
///     aliases: &["alias1", "alias2"],
///     cardinality: Single | Optional | List,
///     item: String | Boolean | Integer | Float | DateTime | Duration | Text,
///     example: "example value",
///     order: 100,
///     [read: <closure or enum variant>,]
///     [write: <closure or enum variant>,]
///     [add: <closure or enum variant>,]
///     [remove: <closure or enum variant>,]
/// };
/// ```
///
/// The callback parameters are optional. If not provided, they default to `None`.
/// Callbacks can be either closures or enum variants.
///
/// **Closures**: Detected by leading `|` or `move |`. The macro automatically
/// wraps closures in the appropriate enum variant based on arity:
/// - Read closures: 1 arg → `ReadFn::Bare`, 2 args → `ReadFn::Schedule`
/// - Write closures: 2 args → `WriteFn::Bare`, 3 args → `WriteFn::Schedule`
/// - Add closures: 2 args → `AddFn::Bare`, 3 args → `AddFn::Schedule`
/// - Remove closures: 2 args → `RemoveFn::Bare`, 3 args → `RemoveFn::Schedule`
///
/// **Enum variants**: Use directly without wrapping, e.g.:
/// - `WriteFn::Schedule`
/// - `AddFn::Schedule`
/// - `RemoveFn::Schedule`
///
/// # Example
///
/// ```ignore
/// pub static FIELD_CUSTOM: FieldDescriptor<PanelEntityType> = {
///     let (data, cb) = callback_field_properties! {
///         PanelEntityType,
///         name: "custom_field",
///         display: "Custom Field",
///         description: "A field with custom callbacks.",
///         aliases: &[],
///         cardinality: Optional,
///         item: String,
///         example: "example",
///         order: 100,
///         read: ReadFn::Schedule(|sched, id| { … }),
///         write: WriteFn::Schedule(|sched, id, val| { … }),
///     };
///     FieldDescriptor {
///         data,
///         required: false,
///         cb,
///     }
/// };
/// inventory::submit! { CollectedNamedField(&FIELD_CUSTOM) }
/// ```
#[proc_macro]
pub fn callback_field_properties(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as callback_input::CallbackInput);
    match callback_output::expand(&parsed) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
