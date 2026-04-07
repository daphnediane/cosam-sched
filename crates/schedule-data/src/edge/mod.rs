/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge system for schedule-data relationships

pub mod generic;
pub mod traits;

// Edge type modules
pub mod event_room_to_hotel_room;
pub mod panel_to_event_room;
pub mod panel_to_panel_type;
pub mod panel_to_presenter;
pub mod presenter_to_group;

pub use generic::GenericEdgeStorage;
pub use traits::{Edge, EdgeError, EdgeId, EdgeStorage, EdgeType};

// Re-export edge types
pub use event_room_to_hotel_room::{EventRoomToHotelRoomEdge, EventRoomToHotelRoomStorage};
pub use panel_to_event_room::PanelToEventRoomEdge;
pub use panel_to_panel_type::{PanelToPanelTypeEdge, PanelToPanelTypeStorage};
pub use panel_to_presenter::{PanelToPresenterEdge, PanelToPresenterStorage};
pub use presenter_to_group::{PresenterToGroupEdge, PresenterToGroupStorage, RelationshipCache};
