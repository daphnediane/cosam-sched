/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge system: typed edge descriptors, edge maps,
//! transitive edge caches, and field node IDs.

pub mod cache;
pub mod descriptor;
pub mod id;
pub mod map;

pub use cache::TransitiveEdgeCache;
pub use descriptor::{EdgeKind, HalfEdgeDescriptor};
pub use id::FullEdge;
pub use map::RawEdgeMap;
