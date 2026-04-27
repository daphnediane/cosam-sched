/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Proc-macro support crate for `schedule-core`.
//!
//! Currently provides a single function-like proc-macro:
//!
//! - [`define_field`] — declarative `FieldDescriptor` static + inventory
//!   submission.  Branches on parameter shape:
//!   - `accessor: <ident>` — stored field (auto crdt + read/write from accessor)
//!   - `edge: ro|rw|one|add|remove` — edge field (auto crdt from `owner` flag,
//!     auto read/write from edge mode; supports `exclusive_with: &SIBLING_FIELD`)
//!   - neither — custom field; require explicit `crdt:`, `cardinality:`,
//!     `item:`, and at least one of `read:` / `write:` closures.
//!
//! All three branches share the common parameters:
//! `name:`, `display:`, `desc:`, `aliases:`, `example:`, `order:`, `required` flag,
//! optional `read:` / `write:` / `verify:` closures (override auto-generated).
//!
//! Re-exported from `schedule-core` so callers `use schedule_core::define_field;`.

mod input;
mod output;

use proc_macro::TokenStream;

/// See crate-level documentation.
#[proc_macro]
pub fn define_field(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as input::FieldInput);
    match output::expand(&parsed) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
