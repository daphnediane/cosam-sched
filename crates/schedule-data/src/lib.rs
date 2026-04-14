/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! `schedule-data` — core schedule data model.
//!
//! Provides the entity/field/macro system for the cosam scheduling tool.
//! See the `entity` module for available entity types and their field definitions.

pub use schedule_macro::EntityFields;

pub mod crdt;
pub mod edit;
pub mod entity;
pub mod field;
pub mod schedule;
pub mod time;

// Re-export frequently used types at crate root
pub use crdt::{ActorId, AutomergeDocument, CrdtDocument, CrdtOp, CrdtScalar, DeviceConfig};
pub use edit::{EditCommand, EditContext, EditHistory};
pub use entity::{EntityKind, EntityType, InternalData, TypedId};
pub use entity::{
    EventRoom, EventRoomId, HotelRoom, HotelRoomId, Panel, PanelId, PanelType, PanelTypeId,
    Presenter, PresenterId, PresenterRank, PresenterSortRank,
};
pub use field::FieldValue;
pub use schedule::{BuildError, Schedule};
