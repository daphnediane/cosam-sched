/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Proc-macro support crate for `schedule-core`.
//!
//! Currently provides two function-like proc-macros:
//!
//! - [`define_field`] — declarative `FieldDescriptor` static + inventory
//!   submission.  Branches on parameter shape:
//!   - `accessor: <ident>` — stored field (auto crdt + read/write from accessor)
//!   - `edge: ro|rw|one|add|remove` — edge field (auto crdt from `owner` flag,
//!     auto read/write from edge mode; supports `exclusive_with: &SIBLING_FIELD`)
//!   - neither — custom field; require explicit `crdt:`, `cardinality:`,
//!     `item:`, and at least one of `read:` / `write:` closures.
//!
//! All three branches share the common parameters:
//! `name:`, `display:`, `desc:`, `aliases:`, `example:`, `order:`, `required` flag,
//! optional `read:` / `write:` / `verify:` closures (override auto-generated).
//!
//! Re-exported from `schedule-core` so callers `use schedule_core::define_field;`.

mod input;
mod output;
mod stored_input;
mod stored_output;

use proc_macro::TokenStream;

/// See crate-level documentation.
#[proc_macro]
pub fn define_field(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as input::FieldInput);
    match output::expand(&parsed) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Generate `CommonFieldData` and `FieldCallbacks` for an accessor-based field.
///
/// Returns a `(CommonFieldData, FieldCallbacks<E>)` tuple containing both
/// the field metadata and the read/write/verify callbacks inferred from
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
///         edge_kind: EdgeKind::NonEdge,
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
