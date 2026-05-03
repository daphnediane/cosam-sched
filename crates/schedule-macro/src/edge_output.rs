/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Code generation for `edge_field_properties!`.

use crate::common_output;
use crate::edge_input::EdgeInput;
use proc_macro2::TokenStream;
use quote::quote;

pub fn expand(inp: &EdgeInput) -> syn::Result<TokenStream> {
    let _entity_type = &inp.entity_type;
    let target_type = &inp.target_type;
    let common = &inp.common;

    // Determine if owner or target edge
    let is_owner = inp.target_field.is_some();
    let is_target = inp.source_fields.is_some();

    // Generate EdgeKind
    let edge_kind = if is_owner {
        let target_field = inp.target_field.as_ref().unwrap();
        let exclusive_with = match &inp.exclusive_with {
            Some(expr) => quote!(Some(#expr)),
            None => quote!(None),
        };
        quote!(::schedule_core::edge::EdgeKind::Owner {
            target_field: #target_field,
            exclusive_with: #exclusive_with,
        })
    } else if is_target {
        let source_fields = inp.source_fields.as_ref().unwrap();
        quote!(::schedule_core::edge::EdgeKind::Target {
            source_fields: #source_fields,
        })
    } else {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "must specify either 'target_field' (for owner edges) or 'source_fields' (for target edges)",
        ));
    };

    // Generate field_type - always List of EntityIdentifier
    let field_type = quote! {
        ::schedule_core::value::FieldType(
            ::schedule_core::value::FieldCardinality::List,
            ::schedule_core::value::FieldTypeItem::EntityIdentifier(
                <#target_type as ::schedule_core::entity::EntityType>::TYPE_NAME,
            ),
        )
    };

    // Generate crdt_type - always Derived for edge fields
    let crdt_type = quote!(::schedule_core::crdt::CrdtFieldType::Derived);

    // Generate CommonFieldData using common helper
    let data = common_output::generate_common_data(common, field_type, crdt_type);

    // Generate read_fn - ReadEdge for both owner and target
    let read_fn = quote!(Some(::schedule_core::field::ReadFn::ReadEdge));

    // Generate write_fn - WriteEdge for both owner and target
    let write_fn = quote!(Some(::schedule_core::field::WriteFn::WriteEdge));

    // Generate add_fn:
    // - Owner edges: AddEdge
    // - Target edges with single source: AddEdge
    // - Target edges with multiple sources: None (TODO: implement multi-source detection)
    let add_fn = if is_owner {
        quote!(Some(::schedule_core::field::AddFn::AddEdge))
    } else if is_target {
        // TODO: Parse source_fields array to check length, return None for multiple sources
        quote!(Some(::schedule_core::field::AddFn::AddEdge))
    } else {
        quote!(None)
    };

    // Generate remove_fn:
    // - Target edges: RemoveEdge
    // - Owner edges without exclusive_with: RemoveEdge
    // - Owner edges with exclusive_with: None
    let remove_fn = if is_target {
        quote!(Some(::schedule_core::field::RemoveFn::RemoveEdge))
    } else if is_owner {
        if inp.exclusive_with.is_some() {
            quote!(None)
        } else {
            quote!(Some(::schedule_core::field::RemoveFn::RemoveEdge))
        }
    } else {
        quote!(None)
    };

    // Generate the complete output - returns (CommonFieldData, FieldCallbacks, EdgeKind) tuple
    Ok(quote! {
        {
            let data = #data;
            let cb = ::schedule_core::field::FieldCallbacks {
                read_fn: #read_fn,
                write_fn: #write_fn,
                add_fn: #add_fn,
                remove_fn: #remove_fn,
            };
            let edge_kind = #edge_kind;
            (data, cb, edge_kind)
        }
    })
}
