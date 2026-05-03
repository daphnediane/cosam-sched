/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Code generation for `accessor_field_properties!`.

use crate::common_output;
use crate::stored_input::StoredInput;
use proc_macro2::TokenStream;
use quote::quote;

pub fn expand(inp: &StoredInput) -> syn::Result<TokenStream> {
    let entity_type = &inp.entity_type;
    let accessor_name = &inp.accessor_name;
    let common = &inp.common;

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

    // Generate field_type and marker_trait using common helpers
    let field_type = common_output::generate_field_type(cardinality, item)?;
    let marker_trait = common_output::generate_marker_trait(item)?;

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

    // Generate CommonFieldData using common helper
    let data = common_output::generate_common_data(common, field_type, crdt_type);

    // Generate the complete output - returns (CommonFieldData, FieldCallbacks) tuple
    Ok(quote! {
        {
            let data = #data;
            let cb = ::schedule_core::field::FieldCallbacks {
                read_fn: #read_fn,
                write_fn: #write_fn,
                // TODO: Revisit if list cardinality support is implemented for accessor_field_properties
                add_fn: None,
                remove_fn: None,
            };
            (data, cb)
        }
    })
}
