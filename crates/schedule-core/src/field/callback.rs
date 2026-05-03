/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field callback functions for read/write operations.

use crate::entity::{EntityId, EntityType};
use crate::schedule::Schedule;
use crate::value::{FieldError, FieldValue};
use crate::FullEdge;

// ── FieldCallbacks<E> ───────────────────────────────────────────────────────────

/// Callback functions for field read/write operations.
///
/// This struct groups the callback functions needed for field operations
/// into a single unit, improving code organization and reducing boilerplate.
pub struct FieldCallbacks<E: EntityType> {
    /// Read implementation. `None` means write-only.
    pub read_fn: Option<ReadFn<E>>,
    /// Write implementation. `None` means read-only.
    pub write_fn: Option<WriteFn<E>>,
    /// Add implementation. `None` means no add.
    pub add_fn: Option<AddFn<E>>,
    /// Remove implementation. `None` means no remove.
    pub remove_fn: Option<RemoveFn<E>>,
}

// ── ReadFn<E> ─────────────────────────────────────────────────────────────────

/// How a field reads its value: directly from [`EntityType::InternalData`], or
/// via a [`Schedule`] lookup by [`EntityId`].
pub enum ReadFn<E: EntityType> {
    /// Data-only read — no schedule access needed.
    Bare(fn(&E::InternalData) -> Option<FieldValue>),
    /// Schedule-aware read — fn receives `(&Schedule, EntityId<E>)` and
    /// performs its own entity lookup internally.
    Schedule(fn(&Schedule, EntityId<E>) -> Option<FieldValue>),
    /// Get Entities connected to this entity via a list of full edges.
    ReadEdges { edges: &'static [&'static FullEdge] },
    /// Read our edge -- to do remove and add to EdgeReadFn
    ReadEdge,
}

// ── WriteFn<E> ────────────────────────────────────────────────────────────────

/// How a field writes its value: directly into [`EntityType::InternalData`], or
/// via a [`Schedule`] lookup by [`EntityId`].
///
/// The `Schedule` variant avoids the double-`&mut` borrow problem: the fn
/// receives `(&mut Schedule, EntityId<E>)` with no `&mut InternalData`
/// parameter and handles its own lookup/release internally.
pub enum WriteFn<E: EntityType> {
    /// Data-only write — no schedule access needed.
    Bare(fn(&mut E::InternalData, FieldValue) -> Result<(), FieldError>),
    /// Schedule-aware write — used for edge mutations (e.g. `add_presenters`).
    Schedule(fn(&mut Schedule, EntityId<E>, FieldValue) -> Result<(), FieldError>),
    /// Add to an edge where both near and far are specified (for other fields)
    ///
    /// TODO: This should be removed in favor of AddFn
    AddEdge {
        edge: FullEdge,
        exclusive_with: Option<FullEdge>,
    },
    /// Remove from an edge where both near and far are specified (for other fields)
    ///
    /// TODO: This should be removed in favor of RemoveFn
    RemoveEdge { edge: FullEdge },
    /// Write our edge -- to do remove and add to EdgeWriteFn
    WriteEdge,
}

// ── AddFn<E> ────────────────────────────────────────────────────────────────

/// How a field appends its value: directly into [`EntityType::InternalData`], or
/// via a [`Schedule`] lookup by [`EntityId`].
///
/// The `Schedule` variant avoids the double-`&mut` borrow problem: the fn
/// receives `(&mut Schedule, EntityId<E>)` with no `&mut InternalData`
/// parameter and handles its own lookup/release internally.
pub enum AddFn<E: EntityType> {
    /// Data-only append — no schedule access needed.
    Bare(fn(&mut E::InternalData, FieldValue) -> Result<(), FieldError>),
    /// Schedule-aware append — used for edge mutations (e.g. `add_presenters`).
    Schedule(fn(&mut Schedule, EntityId<E>, FieldValue) -> Result<(), FieldError>),
    /// Add to our edge
    AddEdge,
}

// ── RemoveFn<E> ────────────────────────────────────────────────────────────────

/// How a field removes from its value: directly into [`EntityType::InternalData`], or
/// via a [`Schedule`] lookup by [`EntityId`].
///
/// The `Schedule` variant avoids the double-`&mut` borrow problem: the fn
/// receives `(&mut Schedule, EntityId<E>)` with no `&mut InternalData`
/// parameter and handles its own lookup/release internally.
pub enum RemoveFn<E: EntityType> {
    /// Data-only remove — no schedule access needed.
    Bare(fn(&mut E::InternalData, FieldValue) -> Result<(), FieldError>),
    /// Schedule-aware remove — used for edge mutations (e.g. `add_presenters`).
    Schedule(fn(&mut Schedule, EntityId<E>, FieldValue) -> Result<(), FieldError>),
    /// Remove from our edge
    RemoveEdge,
}
