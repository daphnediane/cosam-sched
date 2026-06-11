/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared field *definitions* — one file per field.
//!
//! This module is to [`crate::field`] (the field *system*: descriptors,
//! callbacks, sets, builders-of-builders) what [`crate::tables`] is to
//! [`crate::entity`]: the concrete instances built on top of the
//! infrastructure. Each submodule defines one common field as a generic
//! `const fn` builder (`name_field`, `description_field`, `note_field<K>`, …)
//! that returns a [`FieldDescriptor`](crate::field::FieldDescriptor) for any
//! entity type that opts into the relevant capability trait — each defined
//! alongside its builder: [`HasName`](name::HasName),
//! [`HasDescription`](description::HasDescription), [`HasNotes`](note::HasNotes),
//! [`HasStartTime`](time::HasStartTime), [`HasDuration`](duration::HasDuration).
//!
//! Entity modules instantiate these as per-type `static`s — choosing their own
//! `order`/`aliases` and value flavour — and register them through the usual
//! `inventory::submit!`, so a single field definition serves every opted-in
//! entity type with near-zero per-type boilerplate.

pub mod code;
pub mod description;
pub mod duration;
pub mod name;
pub mod note;
pub mod time;
