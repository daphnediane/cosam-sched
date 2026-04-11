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
        storage: &mut crate::schedule::EntityStorage,
        panel_uuid: NonNilUuid,
        presenter_ids: &[crate::entity::PresenterId],
    ) -> usize {
        use crate::entity::PanelToPresenterData;
        use uuid::Uuid;

        let mut added = 0;
        for presenter_id in presenter_ids {
            let presenter_uuid = presenter_id.non_nil_uuid();

            // Skip if already connected
            if Self::presenters_of(storage, panel_uuid)
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

            if storage.add_edge::<PanelToPresenterEntityType>(edge).is_ok() {
                added += 1;
            }
        }
        added
    }

    /// Remove presenters from a panel, deleting their edges.
    ///
    /// Returns the number of presenters actually removed (edges deleted).
    pub fn remove_presenters(
        storage: &mut crate::schedule::EntityStorage,
        panel_uuid: NonNilUuid,
        presenter_ids: &[crate::entity::PresenterId],
    ) -> usize {
        use crate::entity::PanelToPresenterId;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};

        let mut removed = 0;
        for presenter_id in presenter_ids {
            let presenter_uuid = presenter_id.non_nil_uuid();

            let edge_uuids: Vec<NonNilUuid> = {
                let map = Self::typed_map(storage);
                Self::edge_index(storage)
                    .outgoing(panel_uuid)
                    .iter()
                    .copied()
                    .filter(|&edge_uuid| {
                        map.get(&edge_uuid)
                            .is_some_and(|e| e.presenter_uuid == presenter_uuid)
                    })
                    .collect()
            };

            for edge_uuid in edge_uuids {
                storage
                    .remove_edge::<PanelToPresenterEntityType>(PanelToPresenterId::from(edge_uuid));
                removed += 1;
            }
        }
        removed
    }

    /// Replace all presenters of a panel with the given presenter UUIDs.
    pub fn set_presenters(
        storage: &mut crate::schedule::EntityStorage,
        panel_uuid: NonNilUuid,
        presenter_uuids: &[NonNilUuid],
    ) -> Result<(), crate::schedule::InsertError> {
        use crate::entity::PanelToPresenterId;
        use crate::schedule::TypedEdgeStorage;
        let old_edge_uuids: Vec<NonNilUuid> =
            Self::edge_index(storage).outgoing(panel_uuid).to_vec();
        for edge_uuid in old_edge_uuids {
            storage.remove_edge::<Self>(PanelToPresenterId::from(edge_uuid));
        }
        for &presenter_uuid in presenter_uuids {
            let edge_uuid = unsafe { NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) };
            storage.add_edge_with_policy::<Self>(
                crate::entity::PanelToPresenterData {
                    entity_uuid: edge_uuid,
                    panel_uuid,
                    presenter_uuid,
                },
                crate::schedule::EdgePolicy::Ignore,
            )?;
        }
        Ok(())
    }

    /// Replace all panels of a presenter with the given panel UUIDs.
    pub fn set_panels_for_presenter(
        storage: &mut crate::schedule::EntityStorage,
        presenter_uuid: NonNilUuid,
        panel_uuids: &[NonNilUuid],
    ) -> Result<(), crate::schedule::InsertError> {
        use crate::entity::PanelToPresenterId;
        use crate::schedule::TypedEdgeStorage;
        let old_edge_uuids: Vec<NonNilUuid> =
            Self::edge_index(storage).incoming(presenter_uuid).to_vec();
        for edge_uuid in old_edge_uuids {
            storage.remove_edge::<Self>(PanelToPresenterId::from(edge_uuid));
        }
        for &panel_uuid in panel_uuids {
            let edge_uuid = unsafe { NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) };
            storage.add_edge_with_policy::<Self>(
                crate::entity::PanelToPresenterData {
                    entity_uuid: edge_uuid,
                    panel_uuid,
                    presenter_uuid,
                },
                crate::schedule::EdgePolicy::Ignore,
            )?;
        }
        Ok(())
    }

    /// Remove specific panels from a presenter's panel list.
    pub fn remove_panels_for_presenter(
        storage: &mut crate::schedule::EntityStorage,
        presenter_uuid: NonNilUuid,
        panel_uuids: &[NonNilUuid],
    ) {
        use crate::entity::PanelToPresenterId;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        for &panel_uuid in panel_uuids {
            let edge_uuids: Vec<NonNilUuid> = {
                let map = Self::typed_map(storage);
                Self::edge_index(storage)
                    .incoming(presenter_uuid)
                    .iter()
                    .copied()
                    .filter(|&edge_uuid| {
                        map.get(&edge_uuid)
                            .is_some_and(|e| e.panel_uuid == panel_uuid)
                    })
                    .collect()
            };
            for edge_uuid in edge_uuids {
                storage.remove_edge::<Self>(PanelToPresenterId::from(edge_uuid));
            }
        }
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
