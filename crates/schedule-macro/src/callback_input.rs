/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Input parser for `callback_field_properties!`.
//!
//! Grammar (informal):
//!
//! ```text
//! callback_field_properties! {
//!     EntityType,
//!     name: "field_name",
//!     display: "Field Name",
//!     description: "Description text",
//!     aliases: &["alias1", "alias2"],
//!     cardinality: Single | Optional | List,
//!     item: String | Boolean | Integer | Float | DateTime | Duration | Text,
//!     example: "example value",
//!     order: 100,
//!     [read: <closure or enum variant>,]
//!     [write: <closure or enum variant>,]
//!     [add: <closure or enum variant>,]
//!     [remove: <closure or enum variant>,]
//! }
//! ```

use crate::common_input::CommonMetadata;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, ExprClosure, Ident, Token, Type};

/// Parsed input for `callback_field_properties!`.
pub struct CallbackInput {
    /// The entity type (e.g., `PanelTypeEntityType`).
    pub entity_type: Type,
    /// Field cardinality (Single, Optional, List).
    pub cardinality: Ident,
    /// Field type item (String, Boolean, etc.).
    pub item: Ident,
    /// Optional entity type for EntityIdentifier items (e.g., PresenterEntityType).
    pub item_entity: Option<Type>,
    /// Read callback (closure or enum variant).
    pub read: Option<CallbackValue>,
    /// Write callback (closure or enum variant).
    pub write: Option<CallbackValue>,
    /// Add callback (closure or enum variant).
    pub add: Option<CallbackValue>,
    /// Remove callback (closure or enum variant).
    pub remove: Option<CallbackValue>,
    /// Common field metadata.
    pub common: CommonMetadata,
}

/// A callback value - either a closure or an enum variant expression.
pub enum CallbackValue {
    /// A closure expression.
    Closure(ExprClosure),
    /// An enum variant or other expression.
    Expr(Expr),
}

impl CallbackValue {
    /// Parse a callback value - detect closures by leading `|` or `move |`.
    fn parse_callback(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![|]) || input.peek(Token![move]) {
            let closure: ExprClosure = input.parse()?;
            Ok(CallbackValue::Closure(closure))
        } else {
            let expr: Expr = input.parse()?;
            Ok(CallbackValue::Expr(expr))
        }
    }
}

impl Parse for CallbackInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // EntityType,
        let entity_type: Type = input.parse()?;
        input.parse::<syn::Token![,]>()?;

        // Parse key: value pairs
        let mut cardinality = None;
        let mut item = None;
        let mut item_entity = None;
        let mut read = None;
        let mut write = None;
        let mut add = None;
        let mut remove = None;
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
                "item_entity" => {
                    let val: Type = input.parse()?;
                    item_entity = Some(val);
                }
                "read" => read = Some(CallbackValue::parse_callback(input)?),
                "write" => write = Some(CallbackValue::parse_callback(input)?),
                "add" => add = Some(CallbackValue::parse_callback(input)?),
                "remove" => remove = Some(CallbackValue::parse_callback(input)?),
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
            cardinality: cardinality
                .ok_or_else(|| syn::Error::new(input.span(), "missing 'cardinality'"))?,
            item: item.ok_or_else(|| syn::Error::new(input.span(), "missing 'item'"))?,
            item_entity,
            read,
            write,
            add,
            remove,
            common: common.into_common_metadata(input.span())?,
        })
    }
}
