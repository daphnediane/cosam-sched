/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Common output generation shared between accessor and edge field macros.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::common_input::CommonMetadata;

/// Generate CommonFieldData from common metadata and field type.
pub fn generate_common_data(metadata: &CommonMetadata, field_type: TokenStream) -> TokenStream {
    let name = &metadata.name;
    let display = &metadata.display;
    let description = &metadata.description;
    let aliases = &metadata.aliases;
    let example = &metadata.example;
    let order = &metadata.order;

    quote! {
        ::schedule_core::field::CommonFieldData {
            name: #name,
            display: #display,
            description: #description,
            aliases: #aliases,
            field_type: #field_type,
            example: #example,
            order: #order,
        }
    }
}

/// Generate FieldCardinality path from an identifier.
pub fn generate_cardinality_path(cardinality: &Ident) -> syn::Result<TokenStream> {
    match cardinality.to_string().as_str() {
        "Single" => Ok(quote!(::schedule_core::value::FieldCardinality::Single)),
        "Optional" => Ok(quote!(::schedule_core::value::FieldCardinality::Optional)),
        "List" => Ok(quote!(::schedule_core::value::FieldCardinality::List)),
        other => Err(syn::Error::new(
            cardinality.span(),
            format!("unknown cardinality: {other}. Use Single, Optional, or List."),
        )),
    }
}

/// Generate FieldTypeItem path from an identifier.
pub fn generate_item_path(item: &Ident) -> syn::Result<TokenStream> {
    match item.to_string().as_str() {
        "String" => Ok(quote!(::schedule_core::value::FieldTypeItem::String)),
        "Boolean" => Ok(quote!(::schedule_core::value::FieldTypeItem::Boolean)),
        "Integer" => Ok(quote!(::schedule_core::value::FieldTypeItem::Integer)),
        "Float" => Ok(quote!(::schedule_core::value::FieldTypeItem::Float)),
        "DateTime" => Ok(quote!(::schedule_core::value::FieldTypeItem::DateTime)),
        "Duration" => Ok(quote!(::schedule_core::value::FieldTypeItem::Duration)),
        "Text" => Ok(quote!(::schedule_core::value::FieldTypeItem::Text)),
        "EntityIdentifier" => Err(syn::Error::new(
            item.span(),
            "EntityIdentifier requires item_entity parameter to be specified",
        )),
        other => Err(syn::Error::new(
            item.span(),
            format!("unknown item type: {other}. Use String, Boolean, Integer, Float, DateTime, Duration, Text, or EntityIdentifier."),
        )),
    }
}

/// Generate FieldTypeItem path from an identifier with optional entity type.
pub fn generate_item_path_with_entity(
    item: &Ident,
    item_entity: Option<&syn::Type>,
) -> syn::Result<TokenStream> {
    match item.to_string().as_str() {
        "EntityIdentifier" => {
            let entity_type = item_entity.ok_or_else(|| {
                syn::Error::new(
                    item.span(),
                    "EntityIdentifier requires item_entity parameter",
                )
            })?;
            Ok(
                quote!(::schedule_core::value::FieldTypeItem::EntityIdentifier(#entity_type::TYPE_NAME)),
            )
        }
        _ => generate_item_path(item),
    }
}

/// Generate marker trait path from an identifier.
pub fn generate_marker_trait(item: &Ident) -> syn::Result<TokenStream> {
    match item.to_string().as_str() {
        "String" => Ok(quote!(::schedule_core::query::converter::AsString)),
        "Boolean" => Ok(quote!(::schedule_core::query::converter::AsBoolean)),
        "Integer" => Ok(quote!(::schedule_core::query::converter::AsInteger)),
        "Float" => Ok(quote!(::schedule_core::query::converter::AsFloat)),
        "DateTime" => Ok(quote!(::schedule_core::query::converter::AsDateTime)),
        "Duration" => Ok(quote!(::schedule_core::query::converter::AsDuration)),
        "Text" => Ok(quote!(::schedule_core::query::converter::AsText)),
        "EntityIdentifier" => Ok(quote!(::schedule_core::query::converter::AsUuid)),
        other => Err(syn::Error::new(
            item.span(),
            format!("cannot map item type to marker trait: {other}"),
        )),
    }
}

/// Generate FieldType from cardinality and item identifiers.
pub fn generate_field_type(cardinality: &Ident, item: &Ident) -> syn::Result<TokenStream> {
    let cardinality_path = generate_cardinality_path(cardinality)?;
    let item_path = generate_item_path(item)?;
    Ok(quote! {
        ::schedule_core::value::FieldType(
            #cardinality_path,
            #item_path,
        )
    })
}

/// Generate FieldType from cardinality, item identifier, and optional entity type.
pub fn generate_field_type_with_entity(
    cardinality: &Ident,
    item: &Ident,
    item_entity: Option<&syn::Type>,
) -> syn::Result<TokenStream> {
    let cardinality_path = generate_cardinality_path(cardinality)?;
    let item_path = generate_item_path_with_entity(item, item_entity)?;
    Ok(quote! {
        ::schedule_core::value::FieldType(
            #cardinality_path,
            #item_path,
        )
    })
}
