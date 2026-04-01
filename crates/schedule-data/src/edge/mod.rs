/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge system for schedule-data relationships

pub mod storage;
pub mod traits;

// Edge type modules
pub mod event_room_to_hotel_room;
pub mod panel_to_event_room;
pub mod panel_to_panel_type;
pub mod panel_to_presenter;
pub mod presenter_to_group;

pub use storage::*;
pub use traits::*;

// Re-export edge types
pub use event_room_to_hotel_room::EventRoomToHotelRoomEdge;
pub use panel_to_event_room::PanelToEventRoomEdge;
pub use panel_to_panel_type::PanelToPanelTypeEdge;
pub use panel_to_presenter::PanelToPresenterEdge;
pub use presenter_to_group::PresenterToGroupEdge;
