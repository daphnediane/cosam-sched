/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge system: half-edge traits, typed edge descriptors, edge maps,
//! transitive edge caches, and field node IDs.

pub mod cache;
pub mod descriptor;
pub mod id;
pub mod map;
pub mod traits;

pub use cache::TransitiveEdgeCache;
pub use descriptor::{EdgeDescriptor, EdgeKind};
pub use id::{DynamicFieldNodeId, EdgeRef, FieldNodeId, FullEdge, RuntimeFieldNodeId};
pub use map::RawEdgeMap;
pub use traits::{HalfEdge, TypedHalfEdge};
