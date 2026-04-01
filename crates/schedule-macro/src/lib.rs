/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Proc-macro crate providing the [`EntityFields`](derive_entity_fields) derive macro.
//!
//! # Overview
//!
//! `#[derive(EntityFields)]` generates the field trait implementations and
//! a separate `EntityType` struct (e.g., `RoomEntityType`) for a struct defined in `schedule-data`.  
//! All generated code uses fully-qualified `crate::` paths so the consuming entity file only
//! needs `use crate::EntityFields;` plus imports for its own field types.
//!
//! # Supported attributes
//!
//! ## On direct fields
//!
//! | Attribute | Example | Effect |
//! |-----------|---------|--------|
//! | `#[field(display = "…", description = "…")]` | `#[field(display = "Room Name", description = "Short name")]` | Sets display name and description |
//! | `#[alias("a", "b")]` | `#[alias("short", "room_name")]` | Extra lookup names in the `FieldSet` name map |
//! | `#[required]` | — | Adds to the required-fields list for validation |
//! | `#[indexable(priority = N)]` | `#[indexable(priority = 180)]` | Marks field for `match_index` lookups |
//! | `#[field_name("custom")]` | — | Overrides the internal field name (default: Rust field name) |
//! | `#[field_const("CONST")]` | — | Overrides the generated static constant name |
//!
//! ## On computed fields
//!
//! | Attribute | Example | Effect |
//! |-----------|---------|--------|
//! | `#[computed_field(display = "…", description = "…")]` | — | Marks field as computed (user provides closures) |
//! | `#[read(\|entity: &T\| { … })]` | See `edge.rs` | Read closure; may take `schedule` as first param |
//! | `#[write(\|entity: &mut T, value: FieldValue\| { … })]` | See `edge.rs` | Write closure; may take `schedule` as first param |
//! | `#[validate(\|entity, value\| { … })]` | — | Validation closure (parsed but not yet wired up) |
//!
//! **Important**: Closure parameters must have explicit type annotations
//! (e.g. `entity: &Edge`). The macro cannot infer types through associated
//! type projections.
//!
//! # Generated items
//!
//! For each field, the macro generates a unit struct (e.g. `ShortNameField`)
//! with `NamedField`, `SimpleReadableField<T>`, and `SimpleWritableField<T>`
//! impls (or `ReadableField`/`WritableField` for computed fields needing
//! schedule access).  A `pub static` constant is emitted for each field.
//!
//! It also generates a separate `EntityType` struct (e.g., `RoomEntityType`) with `impl EntityType for RoomEntityType`,
//! `type Data = Room`, `TYPE_NAME` as the lowercase struct name, and a `LazyLock`-based
//! `field_set()` containing all fields, aliases, required list, and
//! indexable list.

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Field, GenericArgument, Ident as SynIdent,
    Meta, PathArguments, Type,
};

/// Main derive macro for EntityFields with enhanced attribute support
#[proc_macro_derive(
    EntityFields,
    attributes(
        field,
        alias,
        indexable,
        required,
        field_name,
        field_const,
        computed_field,
        read,
        write,
        validate
    )
)]
pub fn derive_entity_fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let data = match &input.data {
        Data::Struct(data) => data,
        _ => panic!("EntityFields can only be derived on structs"),
    };

    let mut field_implementations: Vec<TokenStream2> = Vec::new();
    let mut field_constants: Vec<TokenStream2> = Vec::new();
    let mut computed_field_implementations: Vec<TokenStream2> = Vec::new();
    let mut field_names: Vec<String> = Vec::new();
    let mut required_field_names: Vec<String> = Vec::new();
    let mut required_field_validations: Vec<TokenStream2> = Vec::new();
    let mut alias_mappings: Vec<TokenStream2> = Vec::new();
    let mut indexable_field_names: Vec<Ident> = Vec::new();

    for field in &data.fields {
        if let Some(field_name) = &field.ident {
            let field_name_str = field_name.to_string();

            // Check for explicit field struct name
            let explicit_field_struct_name = parse_field_struct_name(&field.attrs);
            let field_struct_name = explicit_field_struct_name
                .unwrap_or_else(|| generate_field_struct_name(struct_name, &field_name_str));

            // Check if this is a computed field FIRST
            if is_computed_field(field) {
                let computed_impl = generate_computed_field(struct_name, field);
                computed_field_implementations.push(computed_impl);

                // Add computed field to field set tracking so it appears in
                // the name_map and fields list
                field_names.push(field_name_str.clone());

                // Generate alias mappings for computed fields too
                if let Some(aliases) = parse_field_aliases(&field.attrs) {
                    for alias in aliases {
                        alias_mappings.push(quote! {
                            (#alias, &#field_struct_name),
                        });
                    }
                }
            } else {
                // Parse field attributes
                let attrs = parse_field_attributes(&field.attrs);

                if let Some(field_attrs) = attrs {
                    // Validate field type for non-computed fields
                    if !is_supported_type(&field.ty) {
                        field_implementations.push(quote! {
                            compile_error!("Unsupported field type: EntityFields macro only supports String, i64, i32, u64, u32, bool, and their Option variants. Use computed fields for custom types.");
                        });
                        continue;
                    }

                    // Add to field names for field set generation
                    field_names.push(field_name_str.clone());

                    // Check if field is required
                    let is_required = has_required_attribute(&field.attrs);
                    if is_required {
                        required_field_names.push(field_name_str.clone());

                        // Generate validation for required string fields
                        if is_string_type(&field.ty) {
                            required_field_validations.push(quote! {
                                if data.#field_name.is_empty() {
                                    return Err(crate::field::validation::ValidationError::RequiredFieldMissing {
                                        field: #field_name_str.to_string()
                                    });
                                }
                            });
                        }
                    }

                    // Parse custom indexable match closure if present
                    let custom_match_closure = parse_indexable_match(&field.attrs);

                    // Generate the field implementation
                    let field_impl = generate_direct_field(
                        &field_struct_name,
                        struct_name,
                        field_name,
                        &field.ty,
                        &field_attrs,
                        custom_match_closure,
                    );
                    field_implementations.push(field_impl);

                    // Track indexable fields for IndexableField implementation
                    if field_attrs.indexable.is_some() {
                        indexable_field_names.push(field_struct_name.clone());
                    }

                    // Generate field constant
                    let explicit_constant_name = parse_field_const_name(&field.attrs);
                    let field_constant =
                        generate_field_constant(&field_struct_name, explicit_constant_name.clone());
                    field_constants.push(field_constant.clone());

                    // Generate alias mappings
                    if let Some(aliases) = parse_field_aliases(&field.attrs) {
                        for alias in aliases {
                            alias_mappings.push(quote! {
                                (#alias, &#field_struct_name),
                            });
                        }
                    }
                }
            }
        }
    }

    // Generate TYPE_NAME from struct name (PascalCase → snake_case)
    let type_name_str = pascal_to_snake_case(&struct_name.to_string());

    // Generate field set construction tokens
    let field_struct_names: Vec<Ident> = field_names
        .iter()
        .map(|name| generate_field_struct_name(struct_name, name))
        .collect();

    // Generate the entity type struct name (e.g., PanelEntityType)
    let entity_type_struct_name = Ident::new(
        &format!("{}EntityType", struct_name),
        proc_macro2::Span::call_site(),
    );

    // Generate the complete implementation
    let expanded = quote! {
        #(#field_implementations)*

        #(#computed_field_implementations)*

        /// Generated field constants
        pub mod fields {
            use super::*;

            #(#field_constants)*
        }

        /// Generated EntityType struct for #struct_name
        #[derive(Debug)]
        pub struct #entity_type_struct_name;

        impl crate::entity::EntityType for #entity_type_struct_name {
            type Data = #struct_name;

            const TYPE_NAME: &'static str = #type_name_str;

            fn field_set() -> &'static crate::field::field_set::FieldSet<Self> {
                static FIELD_SET: std::sync::LazyLock<&'static crate::field::field_set::FieldSet<#entity_type_struct_name>> =
                    std::sync::LazyLock::new(|| {
                        let field_refs: Vec<&dyn crate::field::traits::NamedField> = vec![
                            #(&#field_struct_names,)*
                        ];
                        let field_slice = field_refs.leak();

                        let name_map_entries: Vec<(&str, &dyn crate::field::traits::NamedField)> = vec![
                            #(
                                (crate::field::traits::NamedField::name(&#field_struct_names),
                                 &#field_struct_names),
                            )*
                            #(#alias_mappings)*
                        ];
                        let name_map = name_map_entries.leak();

                        let indexable_fields: &[&dyn crate::field::traits::IndexableField<#entity_type_struct_name>] = &[#(&#indexable_field_names,)*];
                        let required_vec: Vec<&str> = vec![#(#required_field_names,)*];
                        let required = required_vec.leak() as &[&str];
                        let fs = crate::field::field_set::FieldSet::new(field_slice, name_map, required, indexable_fields);
                        Box::leak(Box::new(fs))
                    });
                *FIELD_SET
            }

            fn validate(data: &Self::Data) -> Result<(), crate::field::validation::ValidationError> {
                #(#required_field_validations)*
                Ok(())
            }
        }
    };

    TokenStream::from(expanded)
}

/// Check if field has required attribute
fn has_required_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("required"))
}

/// Check if a field is a computed field
fn is_computed_field(field: &Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("computed_field"))
}

/// Generate field struct name from entity and field name
fn generate_field_struct_name(struct_name: &SynIdent, field_name: &str) -> Ident {
    let entity_name = struct_name.to_string();
    let upper = field_name
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i == 0 || field_name.chars().nth(i - 1).unwrap_or('_') == '_' {
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
        .collect::<String>()
        .replace('_', "");

    Ident::new(&format!("{}Field", upper), proc_macro2::Span::call_site())
}

/// Parse field struct name from attributes
fn parse_field_struct_name(attrs: &[Attribute]) -> Option<Ident> {
    for attr in attrs {
        if attr.path().is_ident("field_name") {
            if let Meta::List(meta_list) = &attr.meta {
                for token in meta_list.tokens.clone() {
                    if let Ok(name_value) = syn::parse2::<syn::MetaNameValue>(quote!(#token)) {
                        if name_value.path.is_ident("name") {
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(lit_str),
                                ..
                            }) = name_value.value
                            {
                                return Some(Ident::new(&lit_str.value(), lit_str.span()));
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Parse field const name from attributes
fn parse_field_const_name(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("field_const") {
            if let Meta::List(meta_list) = &attr.meta {
                for token in meta_list.tokens.clone() {
                    if let Ok(name_value) = syn::parse2::<syn::MetaNameValue>(quote!(#token)) {
                        if name_value.path.is_ident("name") {
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(lit_str),
                                ..
                            }) = name_value.value
                            {
                                return Some(lit_str.value());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Parse field aliases from attributes
fn parse_field_aliases(attrs: &[Attribute]) -> Option<Vec<String>> {
    let mut aliases = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("alias") {
            if let Meta::List(meta_list) = &attr.meta {
                for token in meta_list.tokens.clone() {
                    if let Ok(lit_str) = syn::parse2::<syn::LitStr>(quote!(#token)) {
                        aliases.push(lit_str.value());
                    }
                }
            }
        }
    }

    if aliases.is_empty() {
        None
    } else {
        Some(aliases)
    }
}

/// Parse indexable match closure from indexable attribute
fn parse_indexable_match(attrs: &[Attribute]) -> Option<TokenStream2> {
    for attr in attrs {
        if attr.path().is_ident("indexable") {
            if let Meta::List(meta_list) = &attr.meta {
                // Look for closure pattern in the tokens
                let tokens_str = meta_list.tokens.to_string();

                // Check if there's a closure (starts with |)
                if let Some(pipe_start) = tokens_str.find('|') {
                    if let Some(closure_end) = tokens_str[pipe_start..].find("}") {
                        let closure_str = &tokens_str[pipe_start..pipe_start + closure_end + 1];

                        // Parse the closure tokens directly
                        if let Ok(closure_tokens) =
                            syn::parse_str::<proc_macro2::TokenStream>(closure_str)
                        {
                            return Some(closure_tokens);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Parse field attributes from struct field attributes
fn parse_field_attributes(attrs: &[Attribute]) -> Option<FieldAttributes> {
    let mut display = None;
    let mut description = None;
    let mut indexable = None;

    for attr in attrs {
        if attr.path().is_ident("field") {
            // Parse field attribute like #[field(display = "...", description = "...")]
            if let Meta::List(meta_list) = &attr.meta {
                // Parse the tokens as a comma-separated list of name=value pairs
                let tokens_str = meta_list.tokens.to_string();

                // Simple parsing for display and description
                if tokens_str.contains("display") {
                    if let Some(start) = tokens_str.find("display") {
                        if let Some(equals) = tokens_str[start..].find('=') {
                            let value_start = start + equals + 1;
                            if let Some(end) = tokens_str[value_start..].find(',') {
                                let value = &tokens_str[value_start..value_start + end];
                                display = Some(value.trim().trim_matches('"').to_string());
                            } else {
                                let value = &tokens_str[value_start..];
                                display = Some(value.trim().trim_matches('"').to_string());
                            }
                        }
                    }
                }

                if tokens_str.contains("description") {
                    if let Some(start) = tokens_str.find("description") {
                        if let Some(equals) = tokens_str[start..].find('=') {
                            let value_start = start + equals + 1;
                            if let Some(end) = tokens_str[value_start..].find(',') {
                                let value = &tokens_str[value_start..value_start + end];
                                description = Some(value.trim().trim_matches('"').to_string());
                            } else {
                                let value = &tokens_str[value_start..];
                                description = Some(value.trim().trim_matches('"').to_string());
                            }
                        }
                    }
                }
            }
        } else if attr.path().is_ident("indexable") {
            // Parse indexable attribute with optional priority and/or closure
            let mut priority = 100; // default priority

            if let Meta::List(meta_list) = &attr.meta {
                // Parse tokens to find priority
                let tokens_str = meta_list.tokens.to_string();
                if tokens_str.contains("priority") {
                    if let Some(start) = tokens_str.find("priority") {
                        if let Some(equals) = tokens_str[start..].find('=') {
                            let value_start = start + equals + 1;
                            let value_str = &tokens_str[value_start..];
                            if let Some(end) = value_str.find(',') {
                                let value = &value_str[..end];
                                if let Ok(priority_val) = value.trim().parse::<u8>() {
                                    priority = priority_val;
                                }
                            } else if let Ok(priority_val) = value_str.trim().parse::<u8>() {
                                priority = priority_val;
                            }
                        }
                    }
                }
            }
            indexable = Some(IndexableAttributes { priority });
        }
    }

    match (display, description) {
        (Some(display), Some(description)) => Some(FieldAttributes {
            display,
            description,
            indexable,
        }),
        _ => None,
    }
}

/// Check if type is a string type
fn is_string_type(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            segment.ident == "String"
        } else {
            false
        }
    } else {
        false
    }
}

/// Generate direct field implementation
fn generate_direct_field(
    field_struct_name: &Ident,
    struct_name: &SynIdent,
    field_name: &SynIdent,
    field_type: &Type,
    attrs: &FieldAttributes,
    custom_match_closure: Option<TokenStream2>,
) -> TokenStream2 {
    // Generate the entity type struct name (e.g., PanelEntityType)
    let entity_type_struct_name = Ident::new(
        &format!("{}EntityType", struct_name),
        proc_macro2::Span::call_site(),
    );

    // Since we validate types before calling this function, we can assume the type is supported
    let display = &attrs.display;
    let description = &attrs.description;
    let field_name_str = field_name.to_string();

    // Check if the field type supports automatic conversion
    let supports_write = supports_automatic_write(field_type);

    // Generate field value conversion based on type
    let read_conversion = match get_field_type_category(field_type) {
        FieldTypeCategory::String => {
            quote! {
                Some(crate::field::FieldValue::String(entity.#field_name.clone()))
            }
        }
        FieldTypeCategory::Integer => {
            quote! {
                Some(crate::field::FieldValue::Integer(entity.#field_name))
            }
        }
        FieldTypeCategory::Boolean => {
            quote! {
                Some(crate::field::FieldValue::Boolean(entity.#field_name))
            }
        }
        FieldTypeCategory::Map => {
            quote! {
                Some(crate::field::FieldValue::Map(entity.#field_name.clone()))
            }
        }
        FieldTypeCategory::OptionalString => {
            quote! {
                Some(crate::field::FieldValue::OptionalString(entity.#field_name.clone()))
            }
        }
        FieldTypeCategory::OptionalInteger => {
            quote! {
                Some(crate::field::FieldValue::OptionalInteger(entity.#field_name))
            }
        }
        FieldTypeCategory::OptionalBoolean => {
            quote! {
                Some(crate::field::FieldValue::OptionalBoolean(entity.#field_name))
            }
        }
        FieldTypeCategory::EntityId => {
            quote! {
                Some(crate::field::FieldValue::EntityId(entity.#field_name))
            }
        }
        FieldTypeCategory::InternalId => {
            quote! {
                Some(crate::field::FieldValue::InternalId(entity.#field_name))
            }
        }
    };

    let write_conversion = if supports_write {
        match get_field_type_category(field_type) {
            FieldTypeCategory::String => {
                quote! {
                    if let crate::field::FieldValue::String(v) = value {
                        entity.#field_name = v;
                        Ok(())
                    } else {
                        Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
                    }
                }
            }
            FieldTypeCategory::Integer => {
                quote! {
                    if let crate::field::FieldValue::Integer(v) = value {
                        entity.#field_name = v;
                        Ok(())
                    } else {
                        Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
                    }
                }
            }
            FieldTypeCategory::Boolean => {
                quote! {
                    if let crate::field::FieldValue::Boolean(v) = value {
                        entity.#field_name = v;
                        Ok(())
                    } else {
                        Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
                    }
                }
            }
            FieldTypeCategory::OptionalString => {
                quote! {
                    if let crate::field::FieldValue::OptionalString(v) = value {
                        entity.#field_name = v;
                        Ok(())
                    } else {
                        Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
                    }
                }
            }
            FieldTypeCategory::OptionalInteger => {
                quote! {
                    if let crate::field::FieldValue::OptionalInteger(v) = value {
                        entity.#field_name = v;
                        Ok(())
                    } else {
                        Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
                    }
                }
            }
            FieldTypeCategory::Map => {
                quote! {
                    Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
                }
            }
            FieldTypeCategory::OptionalBoolean => {
                quote! {
                    if let crate::field::FieldValue::OptionalBoolean(v) = value {
                        entity.#field_name = v;
                        Ok(())
                    } else {
                        Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
                    }
                }
            }
            FieldTypeCategory::EntityId => {
                quote! {
                    if let crate::field::FieldValue::EntityId(v) = value {
                        entity.#field_name = v;
                        Ok(())
                    } else if let crate::field::FieldValue::Integer(v) = value {
                        entity.#field_name = v as u64;
                        Ok(())
                    } else {
                        Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
                    }
                }
            }
            FieldTypeCategory::InternalId => {
                quote! {
                    if let crate::field::FieldValue::InternalId(v) = value {
                        entity.#field_name = v;
                        Ok(())
                    } else {
                        Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
                    }
                }
            }
        }
    } else {
        quote! {
            Err(crate::field::FieldError::CannotStoreComputedField)
        }
    };

    let simple_writable_impl = if supports_write {
        quote! {
            impl crate::field::traits::SimpleWritableField<#entity_type_struct_name> for #field_struct_name
            where
                #entity_type_struct_name: crate::entity::EntityType,
                Self: crate::field::traits::NamedField + 'static + Send + Sync
            {
                fn write(&self, entity: &mut <#entity_type_struct_name as crate::entity::EntityType>::Data, value: crate::field::FieldValue) -> Result<(), crate::field::FieldError> {
                    #write_conversion
                }

                fn is_write_computed(&self) -> bool {
                    false
                }
            }
        }
    } else {
        quote! {}
    };

    // Generate IndexableField implementation if the field is indexable
    let indexable_impl = if let Some(indexable_attrs) = &attrs.indexable {
        let priority = indexable_attrs.priority;

        // Pre-compute scaled priorities using constants from schedule-data
        let scaled_exact = ((255u16 * priority as u16) / 255u16) as u8; // EXACT_MATCH = 255
        let scaled_strong = ((200u16 * priority as u16) / 255u16) as u8; // STRONG_MATCH = 200 (starts with)
        let scaled_average = ((100u16 * priority as u16) / 255u16) as u8; // AVERAGE_MATCH = 100 (word boundary)
        let scaled_weak = ((50u16 * priority as u16) / 255u16) as u8; // WEAK_MATCH = 50 (contains)

        // Use custom match closure if provided, otherwise generate default logic
        let match_logic = if let Some(closure) = custom_match_closure {
            quote! {
                {
                    // Inject scaled priority values into closure scope
                    let scaled_exact = #scaled_exact;
                    let scaled_strong = #scaled_strong;
                    let scaled_average = #scaled_average;
                    let scaled_weak = #scaled_weak;

                    (#closure)(entity, query)
                }
            }
        } else {
            // Generate default matching logic based on field type
            match get_field_type_category(field_type) {
                FieldTypeCategory::String | FieldTypeCategory::OptionalString => {
                    quote! {
                        if query.is_empty() {
                            None
                        } else if let Some(field_value) = crate::field::traits::SimpleReadableField::<#entity_type_struct_name>::read(self, entity) {
                            if let crate::field::FieldValue::String(s) = field_value {
                                let query_lower = query.to_lowercase();
                                let s_lower = s.to_lowercase();

                                if s_lower == query_lower {
                                    // Use pre-computed scaled exact match
                                    Some(#scaled_exact)
                                } else if s_lower.starts_with(&query_lower) {
                                    // Use pre-computed scaled strong match (starts with)
                                    Some(#scaled_strong)
                                } else if regex::Regex::new(&format!(r"\b{}\b", regex::escape(&query_lower)))
                                    .unwrap()
                                    .is_match(&s_lower) {
                                    // Use pre-computed scaled average match (word boundary)
                                    Some(#scaled_average)
                                } else if s_lower.contains(&query_lower) {
                                    // Use pre-computed scaled weak match (contains)
                                    Some(#scaled_weak)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                }
                FieldTypeCategory::Integer
                | FieldTypeCategory::OptionalInteger
                | FieldTypeCategory::EntityId
                | FieldTypeCategory::InternalId => {
                    quote! {
                        if query.is_empty() {
                            None
                        } else if let Some(field_value) = crate::field::traits::SimpleReadableField::<#entity_type_struct_name>::read(self, entity) {
                            if let crate::field::FieldValue::Integer(i) = field_value {
                                if i.to_string() == query {
                                    Some(#scaled_exact)
                                } else if i.to_string().contains(query) {
                                    Some(#scaled_strong)
                                } else {
                                    None
                                }
                            } else if let crate::field::FieldValue::EntityId(id) = field_value {
                                if id.to_string() == query {
                                    Some(#scaled_exact)
                                } else if id.to_string().contains(query) {
                                    Some(#scaled_strong)
                                } else {
                                    None
                                }
                            } else if let crate::field::FieldValue::InternalId(id) = field_value {
                                if id.to_string() == query {
                                    Some(#scaled_exact)
                                } else if id.to_string().contains(query) {
                                    Some(#scaled_strong)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                }
                FieldTypeCategory::Boolean | FieldTypeCategory::OptionalBoolean => {
                    quote! {
                        if query.is_empty() {
                            None
                        } else if let Some(field_value) = crate::field::traits::SimpleReadableField::<#entity_type_struct_name>::read(self, entity) {
                            if let crate::field::FieldValue::Boolean(b) = field_value {
                                if b.to_string().to_lowercase() == query.to_lowercase() {
                                    Some(#scaled_exact)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                }
                FieldTypeCategory::Map => {
                    quote! {
                        None // Map fields are not indexable
                    }
                }
            }
        };

        quote! {
            impl crate::field::traits::IndexableField<#entity_type_struct_name> for #field_struct_name
            where
                #entity_type_struct_name: crate::entity::EntityType,
                Self: crate::field::traits::NamedField + 'static + Send + Sync
            {
                fn is_indexable(&self) -> bool {
                    true
                }

                fn match_field(&self, query: &str, entity: &<#entity_type_struct_name as crate::entity::EntityType>::Data) -> Option<crate::field::traits::MatchPriority> {
                    #match_logic
                }

                fn index_priority(&self) -> u8 {
                    #priority
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #[derive(Debug)]
        pub struct #field_struct_name;

        impl crate::field::traits::NamedField for #field_struct_name {
            fn name(&self) -> &'static str {
                #field_name_str
            }

            fn display_name(&self) -> &'static str {
                #display
            }

            fn description(&self) -> &'static str {
                #description
            }
        }

        impl crate::field::traits::SimpleReadableField<#entity_type_struct_name> for #field_struct_name
        where
            #entity_type_struct_name: crate::entity::EntityType,
            Self: crate::field::traits::NamedField + 'static + Send + Sync
        {
            fn read(&self, entity: &<#entity_type_struct_name as crate::entity::EntityType>::Data) -> Option<crate::field::FieldValue> {
                #read_conversion
            }

            fn is_read_computed(&self) -> bool {
                false
            }
        }

        #simple_writable_impl
        #indexable_impl
    }
}

fn is_supported_type(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            let ident = segment.ident.to_string();

            // Only basic types and their Option variants are supported
            matches!(
                ident.as_str(),
                "String"
                    | "i64"
                    | "i32"
                    | "u64"
                    | "u32"
                    | "bool"
                    | "Option"
                    | "HashMap"
                    | "EntityId"
                    | "InternalId"
            )
        } else {
            false
        }
    } else {
        false
    }
}

fn supports_automatic_write(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            let ident = segment.ident.to_string();

            // Only basic types support automatic writing (maps don't support automatic writing)
            matches!(
                ident.as_str(),
                "String" | "i64" | "i32" | "u64" | "u32" | "bool" | "EntityId" | "InternalId"
            )
        } else {
            false
        }
    } else {
        false
    }
}

/// Generate computed field implementation
fn generate_computed_field(struct_name: &SynIdent, field: &Field) -> TokenStream2 {
    let field_name = field.ident.as_ref().unwrap();
    let field_struct_name = generate_field_struct_name(struct_name, &field_name.to_string());
    let field_name_str = field_name.to_string();

    // Generate the entity type struct name (e.g., PanelEntityType)
    let entity_type_struct_name = Ident::new(
        &format!("{}EntityType", struct_name),
        proc_macro2::Span::call_site(),
    );

    // Parse computed field attributes
    let mut display = "Computed Field".to_string();
    let mut description = "A computed field".to_string();
    let mut read_closure = None;
    let mut write_closure = None;
    let mut needs_schedule = false;

    // Parse computed_field attributes
    for attr in &field.attrs {
        if attr.path().is_ident("computed_field") {
            if let Meta::List(meta_list) = &attr.meta {
                // Parse tokens to find display and description
                let tokens_str = meta_list.tokens.to_string();
                if tokens_str.contains("display") {
                    if let Some(start) = tokens_str.find("display") {
                        if let Some(equals) = tokens_str[start..].find('=') {
                            let value_start = start + equals + 1;
                            if let Some(end) = tokens_str[value_start..].find(',') {
                                let value = &tokens_str[value_start..value_start + end];
                                display = value.trim().trim_matches('"').to_string();
                            } else {
                                let value = &tokens_str[value_start..];
                                display = value.trim().trim_matches('"').to_string();
                            }
                        }
                    }
                }

                if tokens_str.contains("description") {
                    if let Some(start) = tokens_str.find("description") {
                        if let Some(equals) = tokens_str[start..].find('=') {
                            let value_start = start + equals + 1;
                            if let Some(end) = tokens_str[value_start..].find(',') {
                                let value = &tokens_str[value_start..value_start + end];
                                description = value.trim().trim_matches('"').to_string();
                            } else {
                                let value = &tokens_str[value_start..];
                                description = value.trim().trim_matches('"').to_string();
                            }
                        }
                    }
                }
            }
        } else if attr.path().is_ident("read") {
            // Extract the closure from the read attribute
            if let Meta::List(meta_list) = &attr.meta {
                read_closure = Some(meta_list.tokens.clone());

                // Check if the closure takes a schedule parameter by examining the signature
                let tokens_str = meta_list.tokens.to_string();
                if tokens_str.contains("schedule,") || tokens_str.contains("|schedule") {
                    needs_schedule = true;
                }
            }
        } else if attr.path().is_ident("write") {
            // Extract the closure from the write attribute
            if let Meta::List(meta_list) = &attr.meta {
                write_closure = Some(meta_list.tokens.clone());

                // Check if the closure takes a schedule parameter by examining the signature
                let tokens_str = meta_list.tokens.to_string();
                if tokens_str.contains("schedule,") || tokens_str.contains("|schedule") {
                    needs_schedule = true;
                }
            }
        }
    }

    // Generate appropriate trait implementations based on capabilities
    let mut trait_impls = Vec::new();

    // Always implement NamedField
    trait_impls.push(quote! {
        impl crate::field::traits::NamedField for #field_struct_name {
            fn name(&self) -> &'static str {
                #field_name_str
            }

            fn display_name(&self) -> &'static str {
                #display
            }

            fn description(&self) -> &'static str {
                #description
            }
        }
    });

    // Implement ReadableField or SimpleReadableField based on schedule dependency
    if let Some(closure) = read_closure {
        if needs_schedule {
            trait_impls.push(quote! {
                impl crate::field::traits::ReadableField<#entity_type_struct_name> for #field_struct_name
                where
                    Self: crate::field::traits::NamedField + 'static + Send + Sync
                {
                    fn read(&self, entity: &<#entity_type_struct_name as crate::entity::EntityType>::Data, schedule: &crate::schedule::Schedule) -> Option<crate::field::FieldValue> {
                        let entity: &#struct_name = entity;
                        (#closure)(schedule, entity)
                    }

                    fn is_read_computed(&self) -> bool {
                        true
                    }
                }
            });
        } else {
            trait_impls.push(quote! {
                impl crate::field::traits::SimpleReadableField<#entity_type_struct_name> for #field_struct_name
                where
                    Self: crate::field::traits::NamedField + 'static + Send + Sync
                {
                    fn read(&self, entity: &<#entity_type_struct_name as crate::entity::EntityType>::Data) -> Option<crate::field::FieldValue> {
                        let entity: &#struct_name = entity;
                        (#closure)(entity)
                    }

                    fn is_read_computed(&self) -> bool {
                        true
                    }
                }
            });
        }
    }

    // Implement WritableField or SimpleWritableField for write operations
    if let Some(ref closure) = write_closure {
        // Check if the closure takes a schedule parameter
        let tokens_str = closure.to_string();
        if tokens_str.contains("schedule,") || tokens_str.contains("|schedule") {
            trait_impls.push(quote! {
                impl crate::field::traits::WritableField<#entity_type_struct_name> for #field_struct_name
                where
                    Self: crate::field::traits::NamedField + 'static + Send + Sync
                {
                    fn write(&self, schedule: &crate::schedule::Schedule, entity: &mut <#entity_type_struct_name as crate::entity::EntityType>::Data, value: crate::field::FieldValue) -> Result<(), crate::field::FieldError> {
                        let entity: &mut #struct_name = entity;
                        let value: crate::field::FieldValue = value;
                        (#closure)(schedule, entity, value)
                    }

                    fn is_write_computed(&self) -> bool {
                        true
                    }
                }
            });
        } else {
            trait_impls.push(quote! {
                impl crate::field::traits::SimpleWritableField<#entity_type_struct_name> for #field_struct_name
                where
                    Self: crate::field::traits::NamedField + 'static + Send + Sync
                {
                    fn write(&self, entity: &mut <#entity_type_struct_name as crate::entity::EntityType>::Data, value: crate::field::FieldValue) -> Result<(), crate::field::FieldError> {
                        let entity: &mut #struct_name = entity;
                        let value: crate::field::FieldValue = value;
                        (#closure)(entity, value)
                    }

                    fn is_write_computed(&self) -> bool {
                        true
                    }
                }
            });
        }
    }

    quote! {
        #[derive(Debug)]
        pub struct #field_struct_name;

        #(#trait_impls)*
    }
}

/// Generate field constant
fn generate_field_constant(
    field_struct_name: &Ident,
    explicit_name: Option<String>,
) -> TokenStream2 {
    let constant_name = if let Some(name) = explicit_name {
        Ident::new(&name, field_struct_name.span())
    } else {
        Ident::new(
            &format!("FIELD_{}", field_struct_name.to_string().to_uppercase()),
            field_struct_name.span(),
        )
    };

    quote! {
        pub static #constant_name: #field_struct_name = #field_struct_name;
    }
}

/// Convert PascalCase struct name to snake_case for TYPE_NAME
fn pascal_to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Field attributes
#[derive(Debug)]
struct FieldAttributes {
    display: String,
    description: String,
    indexable: Option<IndexableAttributes>,
}

#[derive(Debug)]
struct IndexableAttributes {
    priority: u8,
}

/// Categorize field types for conversion
#[derive(Debug)]
enum FieldTypeCategory {
    String,
    Integer,
    Boolean,
    Map,
    EntityId,
    InternalId,
    OptionalString,
    OptionalInteger,
    OptionalBoolean,
}

fn get_field_type_category(ty: &Type) -> FieldTypeCategory {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            let ident = segment.ident.to_string();

            if ident == "String" {
                FieldTypeCategory::String
            } else if ident == "i64" || ident == "i32" || ident == "u64" || ident == "u32" {
                FieldTypeCategory::Integer
            } else if ident == "bool" {
                FieldTypeCategory::Boolean
            } else if ident == "HashMap" {
                // Check if it's HashMap<String, FieldValue>
                if let PathArguments::AngleBracketed(angle_bracketed) = &segment.arguments {
                    if angle_bracketed.args.len() == 2 {
                        if let (
                            Some(GenericArgument::Type(key_type)),
                            Some(GenericArgument::Type(value_type)),
                        ) = (
                            angle_bracketed.args.first(),
                            angle_bracketed.args.iter().nth(1),
                        ) {
                            if let Type::Path(key_path) = key_type {
                                if let Some(key_segment) = key_path.path.segments.last() {
                                    if key_segment.ident == "String" {
                                        if let Type::Path(value_path) = value_type {
                                            if let Some(value_segment) =
                                                value_path.path.segments.last()
                                            {
                                                if value_segment.ident == "FieldValue" {
                                                    return FieldTypeCategory::Map;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                panic!("HashMap must be HashMap<String, FieldValue>")
            } else if ident == "EntityId" {
                FieldTypeCategory::EntityId
            } else if ident == "InternalId" {
                FieldTypeCategory::InternalId
            } else if ident == "Option" {
                if let PathArguments::AngleBracketed(angle_bracketed) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_type)) = angle_bracketed.args.first() {
                        match get_field_type_category(inner_type) {
                            FieldTypeCategory::String => FieldTypeCategory::OptionalString,
                            FieldTypeCategory::Integer => FieldTypeCategory::OptionalInteger,
                            FieldTypeCategory::Boolean => FieldTypeCategory::OptionalBoolean,
                            _ => panic!("Unsupported optional field type"),
                        }
                    } else {
                        panic!("Option without inner type")
                    }
                } else {
                    panic!("Option without angle bracketed arguments")
                }
            } else {
                panic!("Unsupported field type: {}", ident)
            }
        } else {
            panic!("Empty path in field type")
        }
    } else {
        panic!("Unsupported field type format")
    }
}
