/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edit command system — [`EditCommand`], [`EditHistory`], and [`EditContext`].
//!
//! All mutations to the schedule go through this module.  Each change is
//! captured as a reversible [`EditCommand`], enabling undo/redo via
//! [`EditHistory`].  [`EditContext`] is the top-level facade that owns both a
//! [`Schedule`] and an [`EditHistory`] and provides the public mutation API.
//!
//! ## Key design properties
//!
//! - **Data-only commands**: every variant stores only [`RuntimeEntityId`],
//!   `&'static str` field names, and [`FieldValue`].  No closures or
//!   `Box<dyn Any>`.
//! - **`EditCommand: Clone`**: all stored types are `Copy`/`Clone`.
//! - **Field selection**: `AddEntity` and `RemoveEntity` snapshots contain
//!   only fields that are both readable *and* writable (i.e.
//!   `read_fn.is_some() && write_fn.is_some()`).  Read-only computed fields
//!   and write-only modifier fields are excluded.
//! - **Stable identity**: `AddEntity` redo and `RemoveEntity` undo always
//!   recreate the entity with its original UUID via `UuidPreference::Exact`.
//! - **CRDT hook**: every applied command passes through [`EditContext::apply`],
//!   which is the natural integration point for generating CRDT operations
//!   in Phase 4.

pub mod builder;
pub mod command;
pub mod context;
pub mod history;

// Re-exports from submodules
pub use command::{add_entity_cmd, snapshot_entity, EditCommand, EditError};
pub use context::EditContext;
pub use history::EditHistory;
