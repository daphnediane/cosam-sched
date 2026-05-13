/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity type modules.
//!
//! Each entity type (Panel, Presenter, EventRoom, etc.) has its own module
//! defining the data structures, field descriptors, and entity type implementation.

pub mod event_room;
pub mod hotel_room;
pub mod panel;
pub mod panel_type;
pub mod presenter;
pub mod timeline;

// Re-exports for convenience
pub use event_room::{EventRoomCommonData, EventRoomEntityType, EventRoomId};
pub use hotel_room::{HotelRoomCommonData, HotelRoomEntityType, HotelRoomId};
pub use panel::{PanelCommonData, PanelEntityType, PanelId};
pub use panel_type::{PanelTypeCommonData, PanelTypeEntityType, PanelTypeId};
pub use presenter::{PresenterCommonData, PresenterEntityType, PresenterId};
pub use timeline::{TimelineCommonData, TimelineEntityType, TimelineId};
