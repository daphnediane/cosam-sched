/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Code generation for `accessor_field_properties!`.

use proc_macro2::TokenStream;
use quote::quote;

use crate::stored_input::StoredInput;

pub fn expand(inp: &StoredInput) -> syn::Result<TokenStream> {
    let entity_type = &inp.entity_type;
    let accessor_name = &inp.accessor_name;
    let name = &inp.name;
    let display = &inp.display;
    let description = &inp.description;
    let aliases = &inp.aliases;
    let example = &inp.example;
    let order = &inp.order;

    let cardinality = &inp.cardinality;
    let item = &inp.item;

    // Determine if optional based on cardinality
    let is_optional = *cardinality == "Optional";

    // Determine required flag (default: true for Single, false for Optional/List)
    let _required = match &inp.required {
        Some(expr) => quote!(#expr),
        None => {
            if is_optional {
                quote!(false)
            } else {
                quote!(true)
            }
        }
    };

    // Generate field_type path
    let cardinality_path = match cardinality.to_string().as_str() {
        "Single" => quote!(::schedule_core::value::FieldCardinality::Single),
        "Optional" => quote!(::schedule_core::value::FieldCardinality::Optional),
        "List" => quote!(::schedule_core::value::FieldCardinality::List),
        other => {
            return Err(syn::Error::new(
                cardinality.span(),
                format!("unknown cardinality: {other}. Use Single, Optional, or List."),
            ));
        }
    };

    // Generate FieldTypeItem path
    let item_path = match item.to_string().as_str() {
        "String" => quote!(::schedule_core::value::FieldTypeItem::String),
        "Boolean" => quote!(::schedule_core::value::FieldTypeItem::Boolean),
        "Integer" => quote!(::schedule_core::value::FieldTypeItem::Integer),
        "Float" => quote!(::schedule_core::value::FieldTypeItem::Float),
        "DateTime" => quote!(::schedule_core::value::FieldTypeItem::DateTime),
        "Duration" => quote!(::schedule_core::value::FieldTypeItem::Duration),
        "Text" => quote!(::schedule_core::value::FieldTypeItem::Text),
        other => {
            return Err(syn::Error::new(
                item.span(),
                format!("unknown item type: {other}. Use String, Boolean, Integer, Float, DateTime, Duration, or Text."),
            ));
        }
    };

    // Generate marker trait path
    let marker_trait = match item.to_string().as_str() {
        "String" => quote!(::schedule_core::query::converter::AsString),
        "Boolean" => quote!(::schedule_core::query::converter::AsBoolean),
        "Integer" => quote!(::schedule_core::query::converter::AsInteger),
        "Float" => quote!(::schedule_core::query::converter::AsFloat),
        "DateTime" => quote!(::schedule_core::query::converter::AsDateTime),
        "Duration" => quote!(::schedule_core::query::converter::AsDuration),
        "Text" => quote!(::schedule_core::query::converter::AsText),
        other => {
            return Err(syn::Error::new(
                item.span(),
                format!("cannot map item type to marker trait: {other}"),
            ));
        }
    };

    // Generate read_fn
    let read_fn = if is_optional {
        quote! {
            Some(::schedule_core::field::ReadFn::Bare(
                |d: &<#entity_type as ::schedule_core::entity::EntityType>::InternalData| {
                    d.data.#accessor_name.as_ref().map(|x| {
                        ::schedule_core::value::FieldValue::Single(
                            <#marker_trait as ::schedule_core::query::converter::FieldTypeMapping>::to_field_value_item(
                                x.clone(),
                            ),
                        )
                    })
                },
            ))
        }
    } else {
        quote! {
            Some(::schedule_core::field::ReadFn::Bare(
                |d: &<#entity_type as ::schedule_core::entity::EntityType>::InternalData| {
                    Some(::schedule_core::value::FieldValue::Single(
                        <#marker_trait as ::schedule_core::query::converter::FieldTypeMapping>::to_field_value_item(
                            d.data.#accessor_name.clone(),
                        ),
                    ))
                },
            ))
        }
    };

    // Generate write_fn
    let write_fn = if is_optional {
        quote! {
            Some(::schedule_core::field::WriteFn::Bare(
                |d: &mut <#entity_type as ::schedule_core::entity::EntityType>::InternalData,
                 v: ::schedule_core::value::FieldValue| {
                    d.data.#accessor_name =
                        ::schedule_core::query::converter::convert_optional::<#marker_trait>(v)?;
                    Ok(())
                },
            ))
        }
    } else {
        quote! {
            Some(::schedule_core::field::WriteFn::Bare(
                |d: &mut <#entity_type as ::schedule_core::entity::EntityType>::InternalData,
                 v: ::schedule_core::value::FieldValue| {
                    d.data.#accessor_name =
                        ::schedule_core::query::converter::convert_required::<#marker_trait>(v)?;
                    Ok(())
                },
            ))
        }
    };

    // Generate crdt_type
    let crdt_type = quote! {
        <#marker_trait as ::schedule_core::query::converter::FieldTypeMapping>::CRDT_TYPE
    };

    // Generate the complete output - returns (CommonFieldData, FieldCallbacks) tuple
    Ok(quote! {
        {
            let data = ::schedule_core::field::CommonFieldData {
                name: #name,
                display: #display,
                description: #description,
                aliases: #aliases,
                field_type: ::schedule_core::value::FieldType(
                    #cardinality_path,
                    #item_path,
                ),
                crdt_type: #crdt_type,
                example: #example,
                order: #order,
            };
            let cb = ::schedule_core::field::FieldCallbacks {
                read_fn: #read_fn,
                write_fn: #write_fn,
                verify_fn: None,
            };
            (data, cb)
        }
    })
}
