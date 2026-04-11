/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelToPanelType edge-entity implementation.
//!
//! Connects a panel to its panel-type category.  A panel has at most one panel
//! type; replacing the edge replaces the assignment.

use crate::EntityFields;
use uuid::NonNilUuid;

/// PanelToPanelType edge-entity.
///
/// The macro generates `PanelToPanelTypeId`, `PanelToPanelTypeData`, and
/// `PanelToPanelTypeEntityType`.
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PanelToPanelType)]
pub struct PanelToPanelType {
    /// UUID of the panel (left side).
    #[field(display = "Panel UUID", description = "UUID of the panel")]
    #[required]
    #[edge_from(Panel)]
    pub panel_uuid: NonNilUuid,

    /// UUID of the panel type (right side).
    #[field(display = "Panel Type UUID", description = "UUID of the panel type")]
    #[required]
    #[edge_to(PanelType)]
    pub panel_type_uuid: NonNilUuid,
}

// ---------------------------------------------------------------------------
// Convenience queries on PanelToPanelTypeEntityType
// ---------------------------------------------------------------------------

impl PanelToPanelTypeEntityType {
    /// The panel type assigned to a panel (at most one; takes first outgoing).
    pub fn panel_type_of(
        storage: &crate::schedule::EntityStorage,
        panel: NonNilUuid,
    ) -> Option<crate::entity::PanelTypeId> {
        use crate::entity::DirectedEdge;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let index = Self::edge_index(storage);
        let map = Self::typed_map(storage);
        index
            .outgoing(panel)
            .first()
            .and_then(|edge_uuid| map.get(edge_uuid))
            .map(|edge| crate::entity::PanelTypeId::from(edge.right_uuid()))
    }

    /// Panels assigned to a panel type (incoming edges).
    pub fn panels_of_type(
        storage: &crate::schedule::EntityStorage,
        panel_type: NonNilUuid,
    ) -> Vec<crate::entity::PanelId> {
        use crate::entity::DirectedEdge;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let index = Self::edge_index(storage);
        let map = Self::typed_map(storage);
        index
            .incoming(panel_type)
            .iter()
            .filter_map(|edge_uuid| map.get(edge_uuid))
            .map(|edge| crate::entity::PanelId::from(edge.left_uuid()))
            .collect()
    }

    /// Set (replace) the panel type for a panel.
    ///
    /// Removes any existing type assignment before adding the new one.
    pub fn set_panel_type(
        storage: &mut crate::schedule::EntityStorage,
        panel_uuid: NonNilUuid,
        panel_type_uuid: NonNilUuid,
    ) -> Result<(), crate::schedule::InsertError> {
        use crate::schedule::TypedEdgeStorage;
        let old_edge_uuids: Vec<NonNilUuid> =
            Self::edge_index(storage).outgoing(panel_uuid).to_vec();
        for edge_uuid in old_edge_uuids {
            storage.remove_edge::<Self>(PanelToPanelTypeId::from(edge_uuid));
        }
        let edge_uuid = unsafe { NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) };
        storage
            .add_edge::<Self>(PanelToPanelTypeData {
                entity_uuid: edge_uuid,
                panel_uuid,
                panel_type_uuid,
            })
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn nn(b: u8) -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b,
            ]))
        }
    }

    #[test]
    fn panel_to_panel_type_id_round_trip() {
        let id = PanelToPanelTypeId::from(nn(1));
        assert_eq!(NonNilUuid::from(id), nn(1));
    }

    #[test]
    fn panel_to_panel_type_id_try_from_nil_returns_none() {
        assert!(PanelToPanelTypeId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn panel_to_panel_type_id_display() {
        let id = PanelToPanelTypeId::from(nn(1));
        assert_eq!(
            id.to_string(),
            "panel-to-panel-type-00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn panel_to_panel_type_data_accessors() {
        let data = PanelToPanelTypeData {
            entity_uuid: nn(3),
            panel_uuid: nn(1),
            panel_type_uuid: nn(2),
        };
        assert_eq!(data.panel_id().non_nil_uuid(), nn(1));
        assert_eq!(data.panel_type_id().non_nil_uuid(), nn(2));
    }
}
