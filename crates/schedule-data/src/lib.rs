/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule data structures and field management
//!
//! This crate provides:
//! - Entity definitions for panels, presenters, rooms, etc.
//! - Field system with read/write/validate capabilities
//! - Schedule management and conflict detection

// Re-export the EntityFields proc macro
pub use schedule_macro::EntityFields;

pub mod edge;
pub mod edge_entity_query;
pub mod entity;
pub mod field;
pub mod query;
pub mod schedule;
pub mod time;
pub mod uuid_v5;

// Re-export core types for convenience
// Note: edge and entity both have panel_to_presenter modules during migration
pub use edge::{
    presenter_to_group, EventRoomToHotelRoomStorage, GenericEdgeStorage, PanelToPanelTypeStorage,
    PanelToPresenterEdge, PanelToPresenterStorage, PresenterToGroupStorage, RelationshipCache,
};
pub use entity::*;
pub use field::*;
pub use query::*;
pub use schedule::*;
pub use time::*;
pub use uuid_v5::*;
