/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Code generation for `define_field!`.
//!
//! Branches on parameter shape:
//!
//! - `accessor:` present → stored field.  Auto-generates `crdt_type`,
//!   `field_type`, `read_fn`, `write_fn` from the accessor + `as:` converter +
//!   `required` / `optional` / `with_default` flag.
//! - `edge:` present → edge field.  Auto-generates `crdt_type`,
//!   `field_type`, `read_fn`, `write_fn` from the `edge: <mode>` (`ro`, `rw`,
//!   `one`, `add`, `remove`) + optional `owner` flag + optional
//!   `exclusive_with: &SIBLING_FIELD`.
//! - neither → custom field.  Caller must supply explicit `crdt:`,
//!   `cardinality:`, `item:`, and at least one of `read:` / `write:` closures.

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Expr, Ident, Type};

use crate::input::FieldInput;

pub fn expand(inp: &FieldInput) -> syn::Result<TokenStream> {
    if inp.kv("accessor").is_some() {
        expand_stored(inp)
    } else if inp.kv("edge").is_some() {
        expand_edge(inp)
    } else {
        expand_custom(inp)
    }
}

// ── Common header ───────────────────────────────────────────────────────────

/// Emit the `pub static FOO: TYPE = FieldDescriptor { … };` plus inventory
/// submission, given the body fields as a token stream.
fn emit_static(inp: &FieldInput, body: TokenStream) -> TokenStream {
    let attrs = &inp.attrs;
    // Default to `pub` when no visibility is supplied: field statics are
    // typically referenced across modules (e.g. as `target_field:` for edges).
    let vis: TokenStream = match &inp.vis {
        syn::Visibility::Inherited => quote!(pub),
        v => quote!(#v),
    };
    let name = &inp.static_name;
    let ty = &inp.static_type;
    quote! {
        #(#attrs)*
        #vis static #name: #ty = ::schedule_core::field::FieldDescriptor {
            #body
        };
        ::inventory::submit! {
            ::schedule_core::field::CollectedNamedField(&#name)
        }
    }
}

/// Pull out the common (name/display/desc/aliases/example/order) parameters.
fn common_meta(inp: &FieldInput) -> syn::Result<TokenStream> {
    let name_lit = inp.require_str("name")?;
    let display_lit = inp.require_str("display")?;
    let desc_lit = inp.require_str("desc")?;
    let aliases = inp.aliases()?;
    let example_lit = inp.require_str("example")?;
    let order_expr = inp.require_expr("order")?;
    Ok(quote! {
        name: #name_lit,
        display: #display_lit,
        description: #desc_lit,
        aliases: #aliases,
        example: #example_lit,
        order: #order_expr,
    })
}

// ── Stored fields ───────────────────────────────────────────────────────────

fn expand_stored(inp: &FieldInput) -> syn::Result<TokenStream> {
    let accessor: Ident = syn::parse2(
        inp.kv("accessor")
            .ok_or_else(|| syn::Error::new(inp.static_name.span(), "internal: missing accessor"))?
            .clone(),
    )?;
    let entity = entity_type_from_static_type(&inp.static_type)?;

    // Required mutually-exclusive flag: required | optional | with_default.
    let required = inp.flag("required");
    let optional = inp.flag("optional");
    let with_default = inp.flag("with_default");
    let n_set = [required, optional, with_default]
        .iter()
        .filter(|b| **b)
        .count();
    if n_set != 1 {
        return Err(syn::Error::new(
            accessor.span(),
            "stored field requires exactly one of `required`, `optional`, or `with_default`",
        ));
    }

    let marker: Type = inp.require_type("as")?;

    let cardinality = if optional {
        quote!(::schedule_core::value::FieldCardinality::Optional)
    } else {
        quote!(::schedule_core::value::FieldCardinality::Single)
    };

    let required_lit = if required {
        quote!(true)
    } else {
        quote!(false)
    };

    // Auto-generated read/write closures (allow override via explicit `read:`/`write:`).
    let read_fn = if let Some(c) = inp.closure("read") {
        quote!(Some(::schedule_core::field::ReadFn::Schedule(#c)))
    } else if optional {
        quote! {
            Some(::schedule_core::field::ReadFn::Bare(
                |d: &<#entity as ::schedule_core::entity::EntityType>::InternalData| {
                    d.data.#accessor.as_ref().map(|x| {
                        ::schedule_core::value::FieldValue::Single(
                            <#marker as ::schedule_core::converter::FieldTypeMapping>::to_field_value_item(
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
                |d: &<#entity as ::schedule_core::entity::EntityType>::InternalData| {
                    Some(::schedule_core::value::FieldValue::Single(
                        <#marker as ::schedule_core::converter::FieldTypeMapping>::to_field_value_item(
                            d.data.#accessor.clone(),
                        ),
                    ))
                },
            ))
        }
    };

    let write_fn = if let Some(c) = inp.closure("write") {
        quote!(Some(::schedule_core::field::WriteFn::Schedule(#c)))
    } else if optional {
        quote! {
            Some(::schedule_core::field::WriteFn::Bare(
                |d: &mut <#entity as ::schedule_core::entity::EntityType>::InternalData,
                 v: ::schedule_core::value::FieldValue| {
                    d.data.#accessor =
                        ::schedule_core::converter::convert_optional::<#marker>(v)?;
                    Ok(())
                },
            ))
        }
    } else {
        quote! {
            Some(::schedule_core::field::WriteFn::Bare(
                |d: &mut <#entity as ::schedule_core::entity::EntityType>::InternalData,
                 v: ::schedule_core::value::FieldValue| {
                    d.data.#accessor =
                        ::schedule_core::converter::convert_required::<#marker>(v)?;
                    Ok(())
                },
            ))
        }
    };

    let verify_fn = if let Some(c) = inp.closure("verify") {
        quote!(Some(::schedule_core::field::VerifyFn::Schedule(#c)))
    } else {
        quote!(None)
    };

    let meta = common_meta(inp)?;

    let body = quote! {
        #meta
        required: #required_lit,
        crdt_type: <#marker as ::schedule_core::converter::FieldTypeMapping>::CRDT_TYPE,
        field_type: ::schedule_core::value::FieldType(
            #cardinality,
            <#marker as ::schedule_core::converter::FieldTypeMapping>::FIELD_TYPE_ITEM,
        ),
        read_fn: #read_fn,
        write_fn: #write_fn,
        verify_fn: #verify_fn,
    };

    Ok(emit_static(inp, body))
}

// ── Edge fields ─────────────────────────────────────────────────────────────

fn expand_edge(inp: &FieldInput) -> syn::Result<TokenStream> {
    let edge_mode_ts = inp.kv("edge").unwrap();
    let edge_mode: Ident = syn::parse2(edge_mode_ts.clone())?;
    let edge_mode_str = edge_mode.to_string();
    let entity = entity_type_from_static_type(&inp.static_type)?;
    let target: Type = inp.require_type("target")?;
    let target_field: Expr = inp.require_expr("target_field")?;
    let owner = inp.flag("owner");
    let exclusive_with = inp.opt_expr("exclusive_with")?;

    // crdt_type
    let crdt_type = match (edge_mode_str.as_str(), owner) {
        ("ro" | "rw" | "one", true) => {
            quote!(::schedule_core::value::CrdtFieldType::EdgeOwner { target_field: #target_field })
        }
        ("ro" | "rw" | "one", false) => {
            quote!(::schedule_core::value::CrdtFieldType::EdgeTarget)
        }
        ("add" | "remove", _) => {
            quote!(::schedule_core::value::CrdtFieldType::Derived)
        }
        _ => {
            return Err(syn::Error::new(
                edge_mode.span(),
                format!("unknown edge mode `{edge_mode_str}` (expected ro|rw|one|add|remove)"),
            ));
        }
    };

    // field_type — list of EntityIdentifier(target_type).
    let field_type = quote! {
        ::schedule_core::value::FieldType(
            ::schedule_core::value::FieldCardinality::List,
            ::schedule_core::value::FieldTypeItem::EntityIdentifier(
                <#target as ::schedule_core::entity::EntityType>::TYPE_NAME,
            ),
        )
    };

    let static_name = &inp.static_name;

    // Auto-gen read fn (ro/rw/one) — connected_entities lookup.
    let auto_read = match edge_mode_str.as_str() {
        "ro" | "rw" | "one" => Some(quote! {
            Some(::schedule_core::field::ReadFn::Schedule(
                |sched: &::schedule_core::schedule::Schedule,
                 id: ::schedule_core::entity::EntityId<#entity>| {
                    let node = ::schedule_core::field_node_id::FieldNodeId::new(id, &#static_name);
                    let ids = sched.connected_entities::<#target>(node, #target_field);
                    Some(::schedule_core::schedule::entity_ids_to_field_value(ids))
                },
            ))
        }),
        _ => None,
    };

    // Auto-gen write fn — depends on mode + exclusive_with.
    let auto_write = match edge_mode_str.as_str() {
        "ro" => None,
        "rw" | "one" => Some(generate_rw_write(
            &entity,
            &target,
            static_name,
            &target_field,
            exclusive_with.as_ref(),
        )),
        "add" => Some(generate_add_write(
            &entity,
            &target,
            static_name,
            &target_field,
            exclusive_with.as_ref(),
        )),
        "remove" => Some(generate_remove_write(
            &entity,
            &target,
            static_name,
            &target_field,
        )),
        _ => unreachable!(),
    };

    let read_fn = if let Some(c) = inp.closure("read") {
        quote!(Some(::schedule_core::field::ReadFn::Schedule(#c)))
    } else if let Some(r) = auto_read {
        r
    } else {
        quote!(None)
    };
    let write_fn = if let Some(c) = inp.closure("write") {
        quote!(Some(::schedule_core::field::WriteFn::Schedule(#c)))
    } else if let Some(w) = auto_write {
        w
    } else {
        quote!(None)
    };
    let verify_fn = if let Some(c) = inp.closure("verify") {
        quote!(Some(::schedule_core::field::VerifyFn::Schedule(#c)))
    } else {
        quote!(None)
    };

    let meta = common_meta(inp)?;
    let body = quote! {
        #meta
        required: false,
        crdt_type: #crdt_type,
        field_type: #field_type,
        read_fn: #read_fn,
        write_fn: #write_fn,
        verify_fn: #verify_fn,
    };

    Ok(emit_static(inp, body))
}

fn generate_rw_write(
    entity: &Type,
    target: &Type,
    static_name: &Ident,
    target_field: &Expr,
    exclusive_with: Option<&Expr>,
) -> TokenStream {
    let exclusivity_prelude = if let Some(exclusive_with) = exclusive_with {
        // For rw fields with exclusive_with, we need to remove from the sibling field
        // before setting this field. The sibling field is on the same entity.
        quote! {
            let __near = unsafe{ ::schedule_core::field_node_id::RuntimeFieldNodeId::new_unchecked(id.entity_uuid(), #exclusive_with) };
            for __r in &ids {
                sched.edge_remove(
                    __near,
                    ::schedule_core::field_node_id::FieldNodeId::new(*__r, #target_field),
                );
            }
        }
    } else {
        quote!()
    };
    quote! {
        Some(::schedule_core::field::WriteFn::Schedule(
            |sched: &mut ::schedule_core::schedule::Schedule,
             id: ::schedule_core::entity::EntityId<#entity>,
             val: ::schedule_core::value::FieldValue| {
                let ids =
                    ::schedule_core::schedule::field_value_to_entity_ids::<#target>(val)?;
                #exclusivity_prelude
                sched.edge_set(
                    ::schedule_core::field_node_id::FieldNodeId::new(id, &#static_name),
                    #target_field,
                    ids,
                );
                Ok(())
            },
        ))
    }
}

fn generate_add_write(
    entity: &Type,
    target: &Type,
    static_name: &Ident,
    target_field: &Expr,
    exclusive_with: Option<&Expr>,
) -> TokenStream {
    let exclusivity_prelude = if let Some(sibling) = exclusive_with {
        // For add fields with exclusive_with, we need to remove from the sibling field
        // before adding to this field. The sibling field is on the same entity.
        quote! {
            sched.edge_remove(
                    unsafe{ ::schedule_core::field_node_id::RuntimeFieldNodeId::new_unchecked(id.entity_uuid(), #sibling) },
                    ::schedule_core::field_node_id::FieldNodeId::new(r, #target_field),
            );
        }
    } else {
        quote!()
    };
    quote! {
        Some(::schedule_core::field::WriteFn::Schedule(
            |sched: &mut ::schedule_core::schedule::Schedule,
             id: ::schedule_core::entity::EntityId<#entity>,
             val: ::schedule_core::value::FieldValue| {
                let ids =
                    ::schedule_core::schedule::field_value_to_entity_ids::<#target>(val)?;
                for r in ids {
                    #exclusivity_prelude
                    sched.edge_add(
                        ::schedule_core::field_node_id::FieldNodeId::new(id, &#static_name),
                        ::schedule_core::field_node_id::FieldNodeId::new(r, #target_field),
                    );
                }
                Ok(())
            },
        ))
    }
}

fn generate_remove_write(
    entity: &Type,
    target: &Type,
    static_name: &Ident,
    target_field: &Expr,
) -> TokenStream {
    quote! {
        Some(::schedule_core::field::WriteFn::Schedule(
            |sched: &mut ::schedule_core::schedule::Schedule,
             id: ::schedule_core::entity::EntityId<#entity>,
             val: ::schedule_core::value::FieldValue| {
                let ids =
                    ::schedule_core::schedule::field_value_to_entity_ids::<#target>(val)?;
                for r in ids {
                    sched.edge_remove(
                        ::schedule_core::field_node_id::FieldNodeId::new(id, &#static_name),
                        ::schedule_core::field_node_id::FieldNodeId::new(r, #target_field),
                    );
                }
                Ok(())
            },
        ))
    }
}

// ── Custom fields ───────────────────────────────────────────────────────────

fn expand_custom(inp: &FieldInput) -> syn::Result<TokenStream> {
    // Required: crdt:, cardinality:, item:.
    let crdt_ident = inp.opt_ident("crdt")?.ok_or_else(|| {
        syn::Error::new(
            inp.static_name.span(),
            "custom define_field! requires `crdt:` (e.g. `crdt: Derived`)",
        )
    })?;
    let crdt_type = match crdt_ident.to_string().as_str() {
        "Scalar" => quote!(::schedule_core::value::CrdtFieldType::Scalar),
        "Text" => quote!(::schedule_core::value::CrdtFieldType::Text),
        "List" => quote!(::schedule_core::value::CrdtFieldType::List),
        "Derived" => quote!(::schedule_core::value::CrdtFieldType::Derived),
        "EdgeTarget" => quote!(::schedule_core::value::CrdtFieldType::EdgeTarget),
        other => {
            return Err(syn::Error::new(
                crdt_ident.span(),
                format!(
                    "unknown crdt variant `{other}` \
                     (use `edge:` to declare EdgeOwner fields)"
                ),
            ));
        }
    };

    let card_ident = inp.opt_ident("cardinality")?.ok_or_else(|| {
        syn::Error::new(
            inp.static_name.span(),
            "custom define_field! requires `cardinality:` (single|optional|list)",
        )
    })?;
    let cardinality = match card_ident.to_string().as_str() {
        "single" => quote!(::schedule_core::value::FieldCardinality::Single),
        "optional" => quote!(::schedule_core::value::FieldCardinality::Optional),
        "list" => quote!(::schedule_core::value::FieldCardinality::List),
        other => {
            return Err(syn::Error::new(
                card_ident.span(),
                format!("unknown cardinality `{other}`"),
            ));
        }
    };

    let item_expr: Expr = inp.require_expr("item")?;

    let required = inp.flag("required");
    let required_lit = if required {
        quote!(true)
    } else {
        quote!(false)
    };

    let read_fn = if let Some(c) = inp.closure("read") {
        // Detect Bare vs Schedule by closure parameter count.
        match closure_arity(c) {
            1 => quote!(Some(::schedule_core::field::ReadFn::Bare(#c))),
            2 => quote!(Some(::schedule_core::field::ReadFn::Schedule(#c))),
            n => {
                return Err(syn::Error::new(
                    c.span(),
                    format!("read closure must take 1 (Bare) or 2 (Schedule) args; got {n}"),
                ));
            }
        }
    } else {
        quote!(None)
    };
    let write_fn = if let Some(c) = inp.closure("write") {
        match closure_arity(c) {
            2 => quote!(Some(::schedule_core::field::WriteFn::Bare(#c))),
            3 => quote!(Some(::schedule_core::field::WriteFn::Schedule(#c))),
            n => {
                return Err(syn::Error::new(
                    c.span(),
                    format!("write closure must take 2 (Bare) or 3 (Schedule) args; got {n}"),
                ));
            }
        }
    } else {
        quote!(None)
    };
    let verify_fn = if let Some(c) = inp.closure("verify") {
        match closure_arity(c) {
            2 => quote!(Some(::schedule_core::field::VerifyFn::Bare(#c))),
            3 => quote!(Some(::schedule_core::field::VerifyFn::Schedule(#c))),
            n => {
                return Err(syn::Error::new(
                    c.span(),
                    format!("verify closure must take 2 (Bare) or 3 (Schedule) args; got {n}"),
                ));
            }
        }
    } else if let Some(ts) = inp.kv("verify") {
        // Allow `verify: ReRead` shorthand.
        let id: Ident = syn::parse2(ts.clone())?;
        if id == "ReRead" {
            quote!(Some(::schedule_core::field::VerifyFn::ReRead))
        } else {
            return Err(syn::Error::new(
                id.span(),
                "verify: must be a closure or `ReRead`",
            ));
        }
    } else {
        quote!(None)
    };

    let meta = common_meta(inp)?;
    let body = quote! {
        #meta
        required: #required_lit,
        crdt_type: #crdt_type,
        field_type: ::schedule_core::value::FieldType(#cardinality, #item_expr),
        read_fn: #read_fn,
        write_fn: #write_fn,
        verify_fn: #verify_fn,
    };
    Ok(emit_static(inp, body))
}

fn closure_arity(c: &syn::ExprClosure) -> usize {
    c.inputs.len()
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Extract the `E` from a `FieldDescriptor<E>` type.
fn entity_type_from_static_type(ty: &Type) -> syn::Result<Type> {
    let Type::Path(tp) = ty else {
        return Err(syn::Error::new(
            ty.span(),
            "expected `FieldDescriptor<E>` static type",
        ));
    };
    let last =
        tp.path.segments.last().ok_or_else(|| {
            syn::Error::new(ty.span(), "expected `FieldDescriptor<E>` static type")
        })?;
    if last.ident != "FieldDescriptor" {
        return Err(syn::Error::new(
            last.ident.span(),
            "expected `FieldDescriptor<E>` static type",
        ));
    }
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return Err(syn::Error::new(
            last.ident.span(),
            "FieldDescriptor takes one type argument: `FieldDescriptor<E>`",
        ));
    };
    let arg = args
        .args
        .first()
        .ok_or_else(|| syn::Error::new(args.span(), "FieldDescriptor takes one type argument"))?;
    let syn::GenericArgument::Type(t) = arg else {
        return Err(syn::Error::new(arg.span(), "expected a type argument"));
    };
    Ok(t.clone())
}
