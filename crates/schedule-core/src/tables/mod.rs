/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity type implementations — concrete entity types (Panel, Presenter, etc.).
//!
//! This module contains the concrete entity type implementations for the schedule system:
//! - [`panel`] — Panel entity type
//! - [`presenter`] — Presenter entity type
//! - [`event_room`] — EventRoom entity type
//! - [`hotel_room`] — HotelRoom entity type
//! - [`panel_type`] — PanelType entity type

pub mod event_room;
pub mod hotel_room;
pub mod panel;
pub mod panel_type;
pub mod presenter;

// Re-exports for convenience
pub use event_room::{EventRoomCommonData, EventRoomEntityType, EventRoomId};
pub use hotel_room::{HotelRoomCommonData, HotelRoomEntityType, HotelRoomId};
pub use panel::{PanelCommonData, PanelEntityType, PanelId};
pub use panel_type::{PanelTypeCommonData, PanelTypeEntityType, PanelTypeId};
pub use presenter::{PresenterCommonData, PresenterEntityType, PresenterId};
