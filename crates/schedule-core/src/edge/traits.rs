/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge trait hierarchy: [`HalfEdge`] and [`TypedHalfEdge`].
//!
//! These types were extracted from `field.rs` so the `edge` module can own
//! edge-specific semantics independently of the general field trait hierarchy.

use crate::edge::id::EdgeRef;
use crate::edge::EdgeKind;
use crate::entity::EntityType;
use crate::field::{NamedField, TypedField};

// ── HalfEdge ─────────────────────────────────────────────────────────────────

/// Marker trait for any field that is one endpoint of a directed edge.
///
/// Only [`EdgeDescriptor<E>`] implements this trait. [`FieldDescriptor<E>`]
/// (scalar, text, list, derived fields) intentionally does **not** implement
/// it, ensuring that non-edge fields cannot be used as [`FieldNodeId`] endpoints.
///
/// Provides naming and description information, plus type-erased identity
/// via [`Self::edge_id`].
///
/// [`FieldNodeId`]: crate::edge::FieldNodeId
/// [`FieldDescriptor<E>`]: crate::field::FieldDescriptor
pub trait HalfEdge: NamedField {
    /// Returns the edge-kind (owner or target) and associated metadata.
    fn edge_kind(&self) -> &EdgeKind;

    /// Type-erased identity — the address of the `'static` descriptor singleton.
    ///
    /// Only meaningful when called on a `'static` edge field descriptor (i.e. one of the
    /// statics declared in each entity module). Returns a [`EdgeRef`] wrapper
    /// that can be used as a HashMap key.
    fn edge_id(&self) -> EdgeRef;

    /// Upcast `self` to `&'dyn NamedField`.
    fn as_named_field(&self) -> &dyn NamedField;
}

// ── TypedHalfEdge<E> ─────────────────────────────────────────────────────────

/// Entity-typed edge half-edge: combines [`HalfEdge`] with [`TypedField<E>`].
///
/// A blanket implementation covers any type that is both a [`HalfEdge`] and a
/// [`TypedField<E>`].  Used as `dyn TypedHalfEdge<E>` in [`FieldNodeId<E>`]
/// to ensure only edge fields can appear as field node ID endpoints.
///
/// [`FieldNodeId<E>`]: crate::edge::FieldNodeId
pub trait TypedHalfEdge<E: EntityType>: HalfEdge + TypedField<E> {}

impl<E: EntityType, T: HalfEdge + TypedField<E>> TypedHalfEdge<E> for T {}
