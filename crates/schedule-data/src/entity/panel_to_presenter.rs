/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelToPresenter edge-entity implementation.
//!
//! Connects a panel to one of its presenters.  Each edge has its own UUID
//! so it can be stored in `EntityStorage` and tracked by the UUID registry.

use crate::EntityFields;
use uuid::NonNilUuid;

/// PanelToPresenter edge-entity.
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PanelToPresenter)]
pub struct PanelToPresenter {
    /// UUID of the panel (from side).
    #[field(display = "Panel UUID", description = "UUID of the panel")]
    #[required]
    #[edge_from(Panel)]
    pub panel_uuid: NonNilUuid,

    /// UUID of the presenter (to side).
    #[field(display = "Presenter UUID", description = "UUID of the presenter")]
    #[required]
    #[edge_to(Presenter)]
    pub presenter_uuid: NonNilUuid,
}

// ---------------------------------------------------------------------------
// Convenience queries on PanelToPresenterEntityType
// ---------------------------------------------------------------------------

impl PanelToPresenterEntityType {
    /// Direct presenters assigned to a panel (outgoing edges).
    pub fn presenters_of(
        storage: &crate::schedule::EntityStorage,
        panel: NonNilUuid,
    ) -> Vec<crate::entity::PresenterId> {
        use crate::entity::DirectedEdge;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let index = Self::edge_index(storage);
        let map = Self::typed_map(storage);
        index
            .outgoing(panel)
            .iter()
            .filter_map(|edge_uuid| map.get(edge_uuid))
            .map(|edge| crate::entity::PresenterId::from(edge.to_uuid()))
            .collect()
    }

    /// Panels that a presenter is assigned to (incoming edges).
    pub fn panels_of(
        storage: &crate::schedule::EntityStorage,
        presenter: NonNilUuid,
    ) -> Vec<crate::entity::PanelId> {
        use crate::entity::DirectedEdge;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let index = Self::edge_index(storage);
        let map = Self::typed_map(storage);
        index
            .incoming(presenter)
            .iter()
            .filter_map(|edge_uuid| map.get(edge_uuid))
            .map(|edge| crate::entity::PanelId::from(edge.from_uuid()))
            .collect()
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
    fn panel_to_presenter_id_round_trip() {
        let id = PanelToPresenterId::from(nn(1));
        assert_eq!(NonNilUuid::from(id), nn(1));
    }

    #[test]
    fn panel_to_presenter_id_try_from_nil_returns_none() {
        assert!(PanelToPresenterId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn panel_to_presenter_id_display() {
        let id = PanelToPresenterId::from(nn(1));
        assert_eq!(
            id.to_string(),
            "panel-to-presenter-00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn panel_to_presenter_data_accessors() {
        let data = PanelToPresenterData {
            entity_uuid: nn(3),
            panel_uuid: nn(1),
            presenter_uuid: nn(2),
        };
        assert_eq!(data.panel_id().non_nil_uuid(), nn(1));
        assert_eq!(data.presenter_id().non_nil_uuid(), nn(2));
    }
}
