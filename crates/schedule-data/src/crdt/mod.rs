/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CRDT abstraction layer for offline collaborative editing.
//!
//! This module defines the abstraction boundary between the entity/field system
//! and the CRDT backend.  The entity and edit layers interact with
//! [`CrdtDocument`] only — they never import automerge types directly.
//!
//! ## Key types
//!
//! - [`ActorId`] — per-device persistent identity for attributing operations.
//! - [`DeviceConfig`] — loads/saves the actor ID from the OS config directory.
//! - [`CrdtOp`] / [`CrdtScalar`] — field-level operation and scalar value types.
//! - [`CrdtDocument`] — trait over a CRDT-backed schedule document.
//! - [`AutomergeDocument`] — production implementation backed by automerge.
//!
//! ## Design
//!
//! See `docs/crdt-design.md` for the settled design decisions, including:
//!
//! - Single automerge document per schedule (one merge call syncs everything)
//! - LWW scalars for structured fields; Text RGA for prose; List for relationships
//! - Soft-delete only — no hard deletes, entities grow monotonically
//! - Per-device UUID v4 actor identity, stored via `directories` crate

mod actor;
mod automerge_backend;
mod backend;
mod ops;

pub use actor::{ActorConfigError, ActorId, DeviceConfig};
// Note: no DEFAULT_APP_NAME — callers pass their own app binary name
// (e.g. "cosam-editor" or "cosam-modify") to DeviceConfig::load_or_create.
pub use automerge_backend::{AutomergeDocError, AutomergeDocument};
pub use backend::CrdtDocument;
pub use ops::{CrdtOp, CrdtScalar};
