/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Code generation for `callback_field_properties!`.

use crate::callback_input::{CallbackInput, CallbackValue};
use crate::common_output;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

pub fn expand(inp: &CallbackInput) -> syn::Result<TokenStream> {
    let entity_type = &inp.entity_type;
    let common = &inp.common;
    let cardinality = &inp.cardinality;
    let item = &inp.item;
    let item_entity = inp.item_entity.as_ref();

    // Generate field_type and marker_trait using common helpers
    let field_type =
        common_output::generate_field_type_with_entity(cardinality, item, item_entity)?;
    let marker_trait = common_output::generate_marker_trait(item)?;

    // Generate crdt_type based on cardinality and item type
    // List cardinality always uses List CRDT type
    // Single/Optional use the marker trait's CRDT type
    let cardinality_str = cardinality.to_string();
    let crdt_type = if cardinality_str == "List" {
        quote!(::schedule_core::crdt::CrdtFieldType::List)
    } else {
        quote! {
            <#marker_trait as ::schedule_core::query::converter::FieldTypeMapping>::CRDT_TYPE
        }
    };

    // Generate CommonFieldData using common helper (without crdt_type)
    let data = common_output::generate_common_data(common, field_type);

    // Generate callbacks - handle both closures and enum variants
    let read_fn = match &inp.read {
        Some(cb) => generate_read_callback(cb)?,
        None => quote!(None),
    };

    let write_fn = match &inp.write {
        Some(cb) => generate_write_callback(cb)?,
        None => quote!(None),
    };

    let add_fn = match &inp.add {
        Some(cb) => generate_add_callback(cb)?,
        None => quote!(None),
    };

    let remove_fn = match &inp.remove {
        Some(cb) => generate_remove_callback(cb)?,
        None => quote!(None),
    };

    // Generate the complete output - returns (CommonFieldData, crdt_type, FieldCallbacks) tuple
    Ok(quote! {
        {
            let data = #data;
            let crdt_type = #crdt_type;
            let cb = ::schedule_core::field::FieldCallbacks::<#entity_type> {
                read_fn: #read_fn,
                write_fn: #write_fn,
                add_fn: #add_fn,
                remove_fn: #remove_fn,
            };
            (data, crdt_type, cb)
        }
    })
}

/// Generate a callback expression.
///
/// # Parameters
///
/// - `cb`: The callback value (closure or expression)
/// - `enum_name`: The enum variant name (e.g., "ReadFn", "WriteFn")
/// - `callback_name`: The callback type name for error messages (e.g., "read", "write")
/// - `bare_arity`: Expected arity for Bare variant
/// - `schedule_arity`: Expected arity for Schedule variant
fn generate_callback(
    cb: &CallbackValue,
    enum_name: &str,
    callback_name: &str,
    bare_arity: usize,
    schedule_arity: usize,
) -> syn::Result<TokenStream> {
    Ok(match cb {
        CallbackValue::Closure(closure) => {
            let arity = closure.inputs.len();
            let enum_ident = proc_macro2::Ident::new(enum_name, proc_macro2::Span::call_site());
            match arity {
                a if a == bare_arity => {
                    quote!(Some(::schedule_core::field::#enum_ident::Bare(#closure)))
                }
                a if a == schedule_arity => {
                    quote!(Some(::schedule_core::field::#enum_ident::Schedule(#closure)))
                }
                n => {
                    return Err(syn::Error::new(
                        closure.span(),
                        format!(
                            "{} closure must take {} (Bare) or {} (Schedule) args; got {}",
                            callback_name, bare_arity, schedule_arity, n
                        ),
                    ))
                }
            }
        }
        CallbackValue::Expr(expr) => quote!(Some(#expr)),
    })
}

/// Generate a read callback expression.
fn generate_read_callback(cb: &CallbackValue) -> syn::Result<TokenStream> {
    generate_callback(cb, "ReadFn", "read", 1, 2)
}

/// Generate a write callback expression.
fn generate_write_callback(cb: &CallbackValue) -> syn::Result<TokenStream> {
    generate_callback(cb, "WriteFn", "write", 2, 3)
}

/// Generate an add callback expression.
fn generate_add_callback(cb: &CallbackValue) -> syn::Result<TokenStream> {
    generate_callback(cb, "AddFn", "add", 2, 3)
}

/// Generate a remove callback expression.
fn generate_remove_callback(cb: &CallbackValue) -> syn::Result<TokenStream> {
    generate_callback(cb, "RemoveFn", "remove", 2, 3)
}
