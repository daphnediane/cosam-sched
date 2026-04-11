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
    /// UUID of the panel (left side).
    #[field(display = "Panel UUID", description = "UUID of the panel")]
    #[required]
    #[edge_from(Panel)]
    pub panel_uuid: NonNilUuid,

    /// UUID of the presenter (right side).
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
            .map(|edge| crate::entity::PresenterId::from(edge.right_uuid()))
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
            .map(|edge| crate::entity::PanelId::from(edge.left_uuid()))
            .collect()
    }

    /// Add presenters to a panel, creating edges for each.
    ///
    /// Returns the number of presenters actually added (new edges created).
    /// Skips presenters that are already connected to the panel.
    pub fn add_presenters(
        schedule: &mut crate::schedule::Schedule,
        panel_uuid: NonNilUuid,
        presenter_ids: &[crate::entity::PresenterId],
    ) -> usize {
        use crate::entity::PanelToPresenterData;
        use uuid::Uuid;

        let mut added = 0;
        for presenter_id in presenter_ids {
            let presenter_uuid = presenter_id.non_nil_uuid();

            // Skip if already connected
            if Self::presenters_of(&schedule.entities, panel_uuid)
                .iter()
                .any(|id| id.non_nil_uuid() == presenter_uuid)
            {
                continue;
            }

            let edge_uuid = unsafe { NonNilUuid::new_unchecked(Uuid::now_v7()) };
            let edge = PanelToPresenterData {
                entity_uuid: edge_uuid,
                panel_uuid,
                presenter_uuid,
            };

            // Add edge; ignore errors (e.g., duplicate with Reject policy)
            if schedule
                .add_edge::<crate::entity::PanelToPresenterEntityType>(edge)
                .is_ok()
            {
                added += 1;
            }
        }
        added
    }

    /// Remove presenters from a panel, deleting their edges.
    ///
    /// Returns the number of presenters actually removed (edges deleted).
    pub fn remove_presenters(
        schedule: &mut crate::schedule::Schedule,
        panel_uuid: NonNilUuid,
        presenter_ids: &[crate::entity::PresenterId],
    ) -> usize {
        use crate::entity::PanelToPresenterId;
        use crate::schedule::TypedEdgeStorage;

        let mut removed = 0;
        for presenter_id in presenter_ids {
            let presenter_uuid = presenter_id.non_nil_uuid();

            // Find the edge UUID for this presenter
            let edge_uuids: Vec<NonNilUuid> = Self::edge_index(&schedule.entities)
                .outgoing(panel_uuid)
                .iter()
                .copied()
                .filter(|&edge_uuid| {
                    schedule
                        .entities
                        .panel_to_presenters
                        .get(&edge_uuid)
                        .is_some_and(|edge| edge.presenter_uuid == presenter_uuid)
                })
                .collect();

            for edge_uuid in edge_uuids {
                schedule.remove_edge::<crate::entity::PanelToPresenterEntityType>(
                    PanelToPresenterId::from_uuid(edge_uuid),
                );
                removed += 1;
            }
        }
        removed
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
