/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Input parser for `define_field!`.
//!
//! Grammar (informal):
//!
//! ```text
//! define_field! {
//!     [#[doc/attr])]*
//!     [pub|pub(...)]? static IDENT : TYPE,
//!     <param>,*
//! }
//! ```
//!
//! `<param>` is one of:
//!
//! - `key: <expr>` for value parameters (most things)
//! - `key: <ident>` for tag parameters (`edge: rw`, `cardinality: list`, `crdt: Scalar`, …)
//! - bare flag identifiers (`required`, `optional`, `with_default`, `owner`)
//! - `read: |…| { … }` etc. for closure parameters (parsed as `ExprClosure`)

use proc_macro2::TokenStream;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Expr, ExprClosure, Ident, LitStr, Token, Type, Visibility};

/// One parameter inside the `{ … }` body, after the static decl.
pub enum Param {
    /// `key: <expr>` — generic value parameter.
    KeyValue { key: Ident, value: TokenStream },
    /// `read: |…| { … }` / `write: |…| { … }` — closure.
    Closure { key: Ident, closure: ExprClosure },
    /// Bare flag (`required`, `optional`, `with_default`, `owner`).
    Flag(Ident),
}

/// Top-level parsed input.
pub struct FieldInput {
    /// Outer attributes (doc comments etc.) on the `static`.
    pub attrs: Vec<Attribute>,
    /// Visibility of the generated `static`.
    pub vis: Visibility,
    /// Name of the generated `static`.
    pub static_name: Ident,
    /// Full type, expected to be `FieldDescriptor<E>`.
    pub static_type: Type,
    /// Parsed parameters in source order.
    pub params: Vec<Param>,
}

impl Parse for FieldInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // [#[attr])]* [vis] static IDENT : TYPE,
        let attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        input.parse::<Token![static]>()?;
        let static_name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let static_type: Type = input.parse()?;
        input.parse::<Token![,]>()?;

        // Comma-separated parameters until end of stream.
        let mut params = Vec::new();
        while !input.is_empty() {
            params.push(parse_param(input)?);
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(Self {
            attrs,
            vis,
            static_name,
            static_type,
            params,
        })
    }
}

fn parse_param(input: ParseStream) -> syn::Result<Param> {
    // Use parse_any so reserved words like `as` can be parameter keys.
    let key: Ident = Ident::parse_any(input)?;
    if !input.peek(Token![:]) {
        // Bare flag.
        return Ok(Param::Flag(key));
    }
    input.parse::<Token![:]>()?;
    // Closure params are recognized by name + the leading `|` or `move |`.
    let is_closure_key = matches!(key.to_string().as_str(), "read" | "write");
    if is_closure_key && (input.peek(Token![|]) || input.peek(Token![move])) {
        let closure: ExprClosure = input.parse()?;
        return Ok(Param::Closure { key, closure });
    }
    // Otherwise, parse the value as a token stream up to the next top-level `,`.
    // We can't just parse as `Expr` because some values are types (`AsString`),
    // tag idents (`rw`), or expressions like `&FIELD_X`.  Collect raw tokens.
    let value = collect_until_comma(input)?;
    Ok(Param::KeyValue { key, value })
}

/// Greedily consume tokens until either end-of-stream or a top-level comma.
/// Brace/bracket/paren groups are consumed wholesale.
fn collect_until_comma(input: ParseStream) -> syn::Result<TokenStream> {
    use proc_macro2::TokenTree;
    let mut out = TokenStream::new();
    while !input.is_empty() {
        if input.peek(Token![,]) {
            break;
        }
        let tt: TokenTree = input.parse()?;
        out.extend(std::iter::once(tt));
    }
    Ok(out)
}

/// Convenience accessors used by `output::expand`.
impl FieldInput {
    /// Find the first key-value param matching `key`; returns the raw token stream.
    pub fn kv(&self, key: &str) -> Option<&TokenStream> {
        self.params.iter().find_map(|p| match p {
            Param::KeyValue { key: k, value } if k == key => Some(value),
            _ => None,
        })
    }

    /// Find the first closure param matching `key`.
    pub fn closure(&self, key: &str) -> Option<&ExprClosure> {
        self.params.iter().find_map(|p| match p {
            Param::Closure { key: k, closure } if k == key => Some(closure),
            _ => None,
        })
    }

    /// Whether a bare flag with the given name was set.
    pub fn flag(&self, name: &str) -> bool {
        self.params
            .iter()
            .any(|p| matches!(p, Param::Flag(k) if k == name))
    }

    /// Required string-literal value (returns Err if missing or wrong type).
    pub fn require_str(&self, key: &str) -> syn::Result<LitStr> {
        let ts = self.kv(key).ok_or_else(|| {
            syn::Error::new(
                self.static_name.span(),
                format!("missing required parameter `{key}:` in define_field!"),
            )
        })?;
        syn::parse2::<LitStr>(ts.clone())
    }

    /// Optional value parsed as `Expr`.
    pub fn opt_expr(&self, key: &str) -> syn::Result<Option<Expr>> {
        match self.kv(key) {
            None => Ok(None),
            Some(ts) => Ok(Some(syn::parse2::<Expr>(ts.clone())?)),
        }
    }

    /// Required value parsed as `Expr`.
    pub fn require_expr(&self, key: &str) -> syn::Result<Expr> {
        let ts = self.kv(key).ok_or_else(|| {
            syn::Error::new(
                self.static_name.span(),
                format!("missing required parameter `{key}:` in define_field!"),
            )
        })?;
        syn::parse2::<Expr>(ts.clone())
    }

    /// Optional value parsed as a single `Ident` (e.g. tag values `rw`, `Scalar`).
    pub fn opt_ident(&self, key: &str) -> syn::Result<Option<Ident>> {
        match self.kv(key) {
            None => Ok(None),
            Some(ts) => Ok(Some(syn::parse2::<Ident>(ts.clone())?)),
        }
    }

    /// Required value parsed as `Type`.
    pub fn require_type(&self, key: &str) -> syn::Result<Type> {
        let ts = self.kv(key).ok_or_else(|| {
            syn::Error::new(
                self.static_name.span(),
                format!("missing required parameter `{key}:` in define_field!"),
            )
        })?;
        syn::parse2::<Type>(ts.clone())
    }

    /// Punctuated-list of aliases (parsed as a generic expression — accepts
    /// `&[]`, `&["a", "b"]`, etc.).
    pub fn aliases(&self) -> syn::Result<Expr> {
        match self.kv("aliases") {
            Some(ts) => syn::parse2::<Expr>(ts.clone()),
            None => syn::parse_str::<Expr>("&[]"),
        }
    }
}
