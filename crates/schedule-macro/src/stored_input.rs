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

use crate::common_input::CommonMetadata;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Type};

/// Parsed input for `accessor_field_properties!`.
pub struct StoredInput {
    /// The entity type (e.g., `PanelTypeEntityType`).
    pub entity_type: Type,
    /// Struct field identifier (e.g., `prefix`).
    pub accessor_name: Ident,
    /// Field cardinality (Single, Optional, List).
    pub cardinality: Ident,
    /// Field type item (String, Boolean, etc.).
    pub item: Ident,
    /// Whether the field is required (optional, defaults based on cardinality).
    pub required: Option<Expr>,
    /// Common field metadata.
    pub common: CommonMetadata,
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
        let mut cardinality = None;
        let mut item = None;
        let mut required = None;
        let mut common = crate::common_input::CommonMetadataOptions::default();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<syn::Token![:]>()?;

            match key.to_string().as_str() {
                "cardinality" => {
                    let val: Ident = input.parse()?;
                    cardinality = Some(val);
                }
                "item" => {
                    let val: Ident = input.parse()?;
                    item = Some(val);
                }
                "required" => required = Some(input.parse()?),
                _ => {
                    // Try to parse as common metadata
                    let value = input.parse::<Expr>()?;
                    if !crate::common_input::try_parse_common_field(&key, value, &mut common) {
                        return Err(syn::Error::new(
                            key.span(),
                            format!("unknown field parameter: {}", key),
                        ));
                    }
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
            cardinality: cardinality
                .ok_or_else(|| syn::Error::new(input.span(), "missing 'cardinality'"))?,
            item: item.ok_or_else(|| syn::Error::new(input.span(), "missing 'item'"))?,
            required,
            common: common.into_common_metadata(input.span())?,
        })
    }
}
