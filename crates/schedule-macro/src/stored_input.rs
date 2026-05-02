/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Input parser for `accessor_field_properties!`.
//!
//! Grammar (informal):
//!
//! ```text
//! accessor_field_properties! {
//!     EntityType,
//!     accessor_name,
//!     name: "field_name",
//!     display: "Field Name",
//!     description: "Description text",
//!     aliases: &["alias1", "alias2"],
//!     cardinality: Single | Optional | List,
//!     item: String | Boolean | Integer | Float | DateTime | Duration | Text,
//!     example: "example value",
//!     order: 100,
//!     [required: true | false,]
//! }
//! ```

use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Type};

/// Parsed input for `accessor_field_properties!`.
pub struct StoredInput {
    /// The entity type (e.g., `PanelTypeEntityType`).
    pub entity_type: Type,
    /// Struct field identifier (e.g., `prefix`).
    pub accessor_name: Ident,
    /// Field name (e.g., "prefix").
    pub name: Expr,
    /// Display name (e.g., "Prefix").
    pub display: Expr,
    /// Description text.
    pub description: Expr,
    /// Aliases array.
    pub aliases: Expr,
    /// Field cardinality (Single, Optional, List).
    pub cardinality: Ident,
    /// Field type item (String, Boolean, etc.).
    pub item: Ident,
    /// Example value.
    pub example: Expr,
    /// Order value.
    pub order: Expr,
    /// Whether the field is required (optional, defaults based on cardinality).
    pub required: Option<Expr>,
}

impl Parse for StoredInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // EntityType,
        let entity_type: Type = input.parse()?;
        input.parse::<syn::Token![,]>()?;

        // accessor_name,
        let accessor_name: Ident = input.parse()?;
        input.parse::<syn::Token![,]>()?;

        // Parse key: value pairs
        let mut name = None;
        let mut display = None;
        let mut description = None;
        let mut aliases = None;
        let mut cardinality = None;
        let mut item = None;
        let mut example = None;
        let mut order = None;
        let mut required = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<syn::Token![:]>()?;

            match key.to_string().as_str() {
                "name" => name = Some(input.parse()?),
                "display" => display = Some(input.parse()?),
                "description" => description = Some(input.parse()?),
                "aliases" => aliases = Some(input.parse()?),
                "cardinality" => {
                    let val: Ident = input.parse()?;
                    cardinality = Some(val);
                }
                "item" => {
                    let val: Ident = input.parse()?;
                    item = Some(val);
                }
                "example" => example = Some(input.parse()?),
                "order" => order = Some(input.parse()?),
                "required" => required = Some(input.parse()?),
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown field parameter: {other}"),
                    ));
                }
            }

            // Optional comma between pairs
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(Self {
            entity_type,
            accessor_name,
            name: name.ok_or_else(|| syn::Error::new(input.span(), "missing 'name'"))?,
            display: display.ok_or_else(|| syn::Error::new(input.span(), "missing 'display'"))?,
            description: description
                .ok_or_else(|| syn::Error::new(input.span(), "missing 'description'"))?,
            aliases: aliases.ok_or_else(|| syn::Error::new(input.span(), "missing 'aliases'"))?,
            cardinality: cardinality
                .ok_or_else(|| syn::Error::new(input.span(), "missing 'cardinality'"))?,
            item: item.ok_or_else(|| syn::Error::new(input.span(), "missing 'item'"))?,
            example: example.ok_or_else(|| syn::Error::new(input.span(), "missing 'example'"))?,
            order: order.ok_or_else(|| syn::Error::new(input.span(), "missing 'order'"))?,
            required,
        })
    }
}
