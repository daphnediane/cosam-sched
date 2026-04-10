/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! V5 UUID generation for deterministic edge identities
//!
//! V5 UUIDs provide deterministic, reproducible identifiers based on a namespace
//! and name input. For edge-entities, this ensures that the same relationship
//! between two entities always produces the same UUID, enabling natural collision
//! detection and idempotent creation.

use uuid::{NonNilUuid, Uuid};

/// Namespace UUID for PanelToPresenter edges
/// Generated as UUIDv5(UUID_NAMESPACE_DNS, "cosam.panel_to_presenter")
pub const PANEL_TO_PRESENTER_NAMESPACE: Uuid =
    uuid::uuid!("6ba7b810-9dad-11d1-80b4-00c04fd430c8");

/// Namespace UUID for PresenterToGroup edges
/// Generated as UUIDv5(UUID_NAMESPACE_DNS, "cosam.presenter_to_group")
pub const PRESENTER_TO_GROUP_NAMESPACE: Uuid =
    uuid::uuid!("6ba7b811-9dad-11d1-80b4-00c04fd430c8");

/// Namespace UUID for PanelToEventRoom edges
pub const PANEL_TO_EVENT_ROOM_NAMESPACE: Uuid =
    uuid::uuid!("6ba7b812-9dad-11d1-80b4-00c04fd430c8");

/// Namespace UUID for PanelToPanelType edges
pub const PANEL_TO_PANEL_TYPE_NAMESPACE: Uuid =
    uuid::uuid!("6ba7b813-9dad-11d1-80b4-00c04fd430c8");

/// Namespace UUID for EventRoomToHotelRoom edges
pub const EVENT_ROOM_TO_HOTEL_ROOM_NAMESPACE: Uuid =
    uuid::uuid!("6ba7b814-9dad-11d1-80b4-00c04fd430c8");

/// Generate a V5 UUID from a namespace and two endpoint UUIDs
///
/// The name is constructed as "from_uuid:to_uuid" to ensure uniqueness
/// and determinism for the relationship.
pub fn edge_uuid(namespace: Uuid, from_uuid: NonNilUuid, to_uuid: NonNilUuid) -> NonNilUuid {
    let name = format!("{}:{}", from_uuid, to_uuid);
    let uuid = Uuid::new_v5(&namespace, name.as_bytes());
    // SAFETY: V5 UUIDs are never nil (all zeros would require SHA-1 to produce all zeros)
    unsafe { NonNilUuid::new_unchecked(uuid) }
}

/// Generate a V5 UUID for PanelToPresenter edges
pub fn panel_to_presenter_uuid(panel_uuid: NonNilUuid, presenter_uuid: NonNilUuid) -> NonNilUuid {
    edge_uuid(PANEL_TO_PRESENTER_NAMESPACE, panel_uuid, presenter_uuid)
}

/// Generate a V5 UUID for PresenterToGroup edges
pub fn presenter_to_group_uuid(member_uuid: NonNilUuid, group_uuid: NonNilUuid) -> NonNilUuid {
    edge_uuid(PRESENTER_TO_GROUP_NAMESPACE, member_uuid, group_uuid)
}

/// Generate a V5 UUID for PanelToEventRoom edges
pub fn panel_to_event_room_uuid(panel_uuid: NonNilUuid, room_uuid: NonNilUuid) -> NonNilUuid {
    edge_uuid(PANEL_TO_EVENT_ROOM_NAMESPACE, panel_uuid, room_uuid)
}

/// Generate a V5 UUID for PanelToPanelType edges
pub fn panel_to_panel_type_uuid(panel_uuid: NonNilUuid, type_uuid: NonNilUuid) -> NonNilUuid {
    edge_uuid(PANEL_TO_PANEL_TYPE_NAMESPACE, panel_uuid, type_uuid)
}

/// Generate a V5 UUID for EventRoomToHotelRoom edges
pub fn event_room_to_hotel_room_uuid(
    event_room_uuid: NonNilUuid,
    hotel_room_uuid: NonNilUuid,
) -> NonNilUuid {
    edge_uuid(EVENT_ROOM_TO_HOTEL_ROOM_NAMESPACE, event_room_uuid, hotel_room_uuid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uuid(byte: u8) -> NonNilUuid {
        let mut bytes = [0u8; 16];
        bytes[15] = byte;
        unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes(bytes)) }
    }

    #[test]
    fn test_edge_uuid_deterministic() {
        let from = test_uuid(1);
        let to = test_uuid(2);

        let uuid1 = edge_uuid(PANEL_TO_PRESENTER_NAMESPACE, from, to);
        let uuid2 = edge_uuid(PANEL_TO_PRESENTER_NAMESPACE, from, to);

        assert_eq!(uuid1, uuid2, "V5 UUIDs should be deterministic");
    }

    #[test]
    fn test_edge_uuid_different_order_different_result() {
        let a = test_uuid(1);
        let b = test_uuid(2);

        let uuid_ab = edge_uuid(PANEL_TO_PRESENTER_NAMESPACE, a, b);
        let uuid_ba = edge_uuid(PANEL_TO_PRESENTER_NAMESPACE, b, a);

        assert_ne!(uuid_ab, uuid_ba, "Different order should produce different UUIDs");
    }

    #[test]
    fn test_edge_uuid_different_namespace_different_result() {
        let from = test_uuid(1);
        let to = test_uuid(2);

        let uuid1 = edge_uuid(PANEL_TO_PRESENTER_NAMESPACE, from, to);
        let uuid2 = edge_uuid(PRESENTER_TO_GROUP_NAMESPACE, from, to);

        assert_ne!(uuid1, uuid2, "Different namespaces should produce different UUIDs");
    }

    #[test]
    fn test_panel_to_presenter_uuid() {
        let panel = test_uuid(1);
        let presenter = test_uuid(2);

        let uuid1 = panel_to_presenter_uuid(panel, presenter);
        let uuid2 = panel_to_presenter_uuid(panel, presenter);

        assert_eq!(uuid1, uuid2);
    }

    #[test]
    fn test_presenter_to_group_uuid() {
        let member = test_uuid(1);
        let group = test_uuid(2);

        let uuid1 = presenter_to_group_uuid(member, group);
        let uuid2 = presenter_to_group_uuid(member, group);

        assert_eq!(uuid1, uuid2);
    }
}
