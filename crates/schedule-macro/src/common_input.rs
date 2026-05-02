/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Common input structures and parsing shared between accessor and edge field macros.

use syn::Expr;
use syn::Ident;

/// Common field metadata shared by both accessor and edge field macros.
pub struct CommonMetadata {
    /// Field name (e.g., "field_name").
    pub name: Expr,
    /// Display name (e.g., "Field Name").
    pub display: Expr,
    /// Description text.
    pub description: Expr,
    /// Aliases array.
    pub aliases: Expr,
    /// Example value.
    pub example: Expr,
    /// Order value.
    pub order: Expr,
}

/// Options for common metadata during parsing.
#[derive(Default)]
pub struct CommonMetadataOptions {
    pub name: Option<Expr>,
    pub display: Option<Expr>,
    pub description: Option<Expr>,
    pub aliases: Option<Expr>,
    pub example: Option<Expr>,
    pub order: Option<Expr>,
}

impl CommonMetadataOptions {
    /// Convert options to CommonMetadata, validating that all required fields are present.
    pub fn into_common_metadata(self, span: proc_macro2::Span) -> syn::Result<CommonMetadata> {
        Ok(CommonMetadata {
            name: self
                .name
                .ok_or_else(|| syn::Error::new(span, "missing name"))?,
            display: self
                .display
                .ok_or_else(|| syn::Error::new(span, "missing display"))?,
            description: self
                .description
                .ok_or_else(|| syn::Error::new(span, "missing description"))?,
            aliases: self
                .aliases
                .ok_or_else(|| syn::Error::new(span, "missing aliases"))?,
            example: self
                .example
                .ok_or_else(|| syn::Error::new(span, "missing example"))?,
            order: self
                .order
                .ok_or_else(|| syn::Error::new(span, "missing order"))?,
        })
    }
}

/// Try to parse a key-value pair as common metadata.
///
/// Returns `true` if the key was recognized and the value was stored,
/// `false` if the key is not common metadata (caller should handle it).
///
/// # Arguments
///
/// * `key` - The field key
/// * `value` - The parsed value expression
/// * `opts` - Common metadata options to update
pub fn try_parse_common_field(key: &Ident, value: Expr, opts: &mut CommonMetadataOptions) -> bool {
    match key.to_string().as_str() {
        "name" => {
            opts.name = Some(value);
            true
        }
        "display" => {
            opts.display = Some(value);
            true
        }
        "description" => {
            opts.description = Some(value);
            true
        }
        "aliases" => {
            opts.aliases = Some(value);
            true
        }
        "example" => {
            opts.example = Some(value);
            true
        }
        "order" => {
            opts.order = Some(value);
            true
        }
        _ => false,
    }
}
