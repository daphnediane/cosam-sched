/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge trait hierarchy: [`HalfEdge`].
//!
//! These types were extracted from `field.rs` so the `edge` module can own
//! edge-specific semantics independently of the general field trait hierarchy.

use crate::edge::EdgeKind;
use crate::field::NamedField;

// ── HalfEdge ─────────────────────────────────────────────────────────────────

/// Marker trait for any field that is one endpoint of a directed edge.
///
/// Only [`EdgeDescriptor<E>`] implements this trait. [`FieldDescriptor<E>`]
/// (scalar, text, list, derived fields) intentionally does **not** implement
/// it, ensuring that non-edge fields cannot be used as edge endpoints.
///
/// Provides naming and description information, plus type-erased identity
/// via [`Self::edge_id`].
///
/// [`FieldDescriptor<E>`]: crate::field::FieldDescriptor
pub trait HalfEdge: NamedField {
    /// Returns the edge-kind (owner or target) and associated metadata.
    fn edge_kind(&self) -> &EdgeKind;

    /// Type-erased identity — the address of the `'static` descriptor singleton.
    ///
    /// Only meaningful when called on a `'static` edge field descriptor (i.e. one of the
    /// statics declared in each entity module). Returns a `'static dyn HalfEdge` reference
    /// that can be used as a HashMap key via pointer address.
    fn edge_id(&self) -> &'static dyn HalfEdge;

    /// Upcast `self` to `&'dyn NamedField`.
    fn as_named_field(&self) -> &dyn NamedField;
}
