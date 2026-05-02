/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Proc-macro support crate for `schedule-core`.
//!
//! Currently provides three function-like proc-macros:
//!
//! - [`define_field`] — declarative `FieldDescriptor` static + inventory
//!   submission.  Branches on parameter shape:
//!   - `accessor: <ident>` — stored field (auto crdt + read/write from accessor)
//!   - `edge: ro|rw|one|add|remove` — edge field (auto crdt from `owner` flag,
//!     auto read/write from edge mode; supports `exclusive_with: &SIBLING_FIELD`)
//!   - neither — custom field; require explicit `crdt:`, `cardinality:`,
//!     `item:`, and at least one of `read:` / `write:` closures.
//!
//! - [`accessor_field_properties`] — generates `(CommonFieldData, FieldCallbacks)`
//!   tuple for accessor-based fields without custom callbacks.
//!
//! - [`edge_field_properties`] — generates `(CommonFieldData, FieldCallbacks, EdgeKind)`
//!   tuple for edge fields without custom callbacks.
//!
//! All three branches share the common parameters:
//! `name:`, `display:`, `desc:`, `aliases:`, `example:`, `order:`, `required` flag,
//! optional `read:` / `write:` / `verify:` closures (override auto-generated).
//!
//! Re-exported from `schedule-core` so callers `use schedule_core::define_field;`.

mod common_input;
mod common_output;
mod edge_input;
mod edge_output;
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

/// Generate `CommonFieldData`, `FieldCallbacks`, and `EdgeKind` for an edge field.
///
/// Returns a `(CommonFieldData, FieldCallbacks<E>, EdgeKind)` tuple containing
/// the field metadata, read/write/verify callbacks, and edge ownership information.
/// The caller constructs the `FieldDescriptor` to control other descriptor-level
/// properties.
///
/// Supports both owner edges (with optional exclusivity) and target edges (with
/// multiple source fields).
///
/// # Owner edge syntax
///
/// ```ignore
/// let (data, cb, edge_kind) = edge_field_properties! {
///     EntityType,
///     target: TargetEntityType,
///     target_field: &other_entity::FIELD_OTHER,
///     [exclusive_with: &FIELD_SIBLING,]
///     name: "field_name",
///     display: "Field Name",
///     description: "Description text",
///     aliases: &["alias1", "alias2"],
///     example: "example value",
///     order: 100,
/// };
/// ```
///
/// # Target edge syntax
///
/// ```ignore
/// let (data, cb, edge_kind) = edge_field_properties! {
///     EntityType,
///     target: TargetEntityType,
///     source_fields: &[&other_entity::FIELD_OWNER1, &other_entity::FIELD_OWNER2],
///     name: "field_name",
///     display: "Field Name",
///     description: "Description text",
///     aliases: &["alias1", "alias2"],
///     example: "example value",
///     order: 100,
/// };
/// ```
///
/// # Owner edge example
///
/// ```ignore
/// pub static FIELD_CREDITED_PRESENTERS: FieldDescriptor<PanelEntityType> = {
///     let (data, cb, edge_kind) = edge_field_properties! {
///         PanelEntityType,
///         target: PresenterEntityType,
///         target_field: &crate::tables::presenter::FIELD_PANELS,
///         exclusive_with: &FIELD_UNCREDITED_PRESENTERS,
///         name: "credited_presenters",
///         display: "Credited Presenters",
///         description: "Presenters credited on this panel.",
///         aliases: &["credited_panelists", "credited_presenter"],
///         example: "[presenter_id]",
///         order: 2710,
///     };
///     FieldDescriptor {
///         data,
///         required: false,
///         edge_kind,
///         cb,
///     }
/// };
/// inventory::submit! { CollectedNamedField(&FIELD_CREDITED_PRESENTERS) }
/// ```
#[proc_macro]
pub fn edge_field_properties(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as edge_input::EdgeInput);
    match edge_output::expand(&parsed) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
