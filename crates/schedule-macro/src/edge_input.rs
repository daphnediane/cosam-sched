/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Input parser for `edge_field_properties!`.
//!
//! Grammar (informal):
//!
//! ```text
//! edge_field_properties! {
//!     EntityType,
//!     target: TargetEntityType,
//!     target_field: &other_entity::FIELD_OTHER,  // for owner edges
//!     [exclusive_with: &FIELD_SIBLING,]         // optional for owner edges
//!     OR
//!     source_fields: &[&other_entity::FIELD_OWNER1, &other_entity::FIELD_OWNER2],  // for target edges
//!     name: "field_name",
//!     display: "Field Name",
//!     description: "Description text",
//!     aliases: &["alias1", "alias2"],
//!     example: "example value",
//!     order: 100,
//! }
//! ```

use crate::common_input::CommonMetadata;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Type};

/// Parsed input for `edge_field_properties!`.
pub struct EdgeInput {
    /// The entity type (e.g., `PanelEntityType`).
    pub entity_type: Type,
    /// The target entity type (e.g., `PresenterEntityType`).
    pub target_type: Type,
    /// Target field reference (for owner edges).
    pub target_field: Option<Expr>,
    /// Source fields array (for target edges).
    pub source_fields: Option<Expr>,
    /// Exclusive sibling field (for owner edges).
    pub exclusive_with: Option<Expr>,
    /// Common field metadata.
    pub common: CommonMetadata,
}

impl Parse for EdgeInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // EntityType,
        let entity_type: Type = input.parse()?;
        input.parse::<syn::Token![,]>()?;

        // target: TargetEntityType,
        let target_key: Ident = input.parse()?;
        if target_key != "target" {
            return Err(syn::Error::new(
                target_key.span(),
                format!("expected 'target', got '{target_key}'"),
            ));
        }
        input.parse::<syn::Token![:]>()?;
        let target_type: Type = input.parse()?;
        input.parse::<syn::Token![,]>()?;

        // Parse edge-specific parameters first
        let mut target_field = None;
        let mut source_fields = None;
        let mut exclusive_with = None;
        let mut common = crate::common_input::CommonMetadataOptions::default();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<syn::Token![:]>()?;

            // Try to parse as edge-specific field
            let value = input.parse::<Expr>()?;
            match key.to_string().as_str() {
                "target_field" => target_field = Some(value),
                "source_fields" => source_fields = Some(value),
                "exclusive_with" => exclusive_with = Some(value),
                _ => {
                    // Try to parse as common metadata
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

        // Validate that we have either target_field (owner) or source_fields (target)
        let is_owner = target_field.is_some();
        let is_target = source_fields.is_some();
        if is_owner == is_target {
            return Err(syn::Error::new(
                input.span(),
                "must specify either 'target_field' (for owner edges) or 'source_fields' (for target edges), but not both",
            ));
        }

        Ok(Self {
            entity_type,
            target_type,
            target_field,
            source_fields,
            exclusive_with,
            common: common.into_common_metadata(input.span())?,
        })
    }
}
