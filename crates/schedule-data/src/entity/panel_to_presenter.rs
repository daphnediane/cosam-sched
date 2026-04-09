/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelToPresenter edge-entity implementation
//!
//! This edge type connects panels to their presenters.
//! As an edge-entity, it has its own UUID and can store metadata
//! like whether this is the primary presenter.

use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::{NonNilUuid, Uuid};

/// PanelToPresenter edge ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PanelToPresenterId(NonNilUuid);

impl PanelToPresenterId {
    /// Get the NonNilUuid from this ID
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }

    /// Get the raw UUID from this ID
    pub fn uuid(&self) -> Uuid {
        self.0.into()
    }

    /// Create a PanelToPresenterId from a NonNilUuid (infallible)
    pub fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }

    /// Try to create a PanelToPresenterId from a raw UUID (boundary use only)
    pub fn try_from_raw_uuid(uuid: Uuid) -> Option<Self> {
        NonNilUuid::new(uuid).map(Self)
    }
}

impl fmt::Display for PanelToPresenterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panel-to-presenter-{}", self.0)
    }
}

impl From<NonNilUuid> for PanelToPresenterId {
    fn from(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

impl From<PanelToPresenterId> for NonNilUuid {
    fn from(id: PanelToPresenterId) -> NonNilUuid {
        id.0
    }
}

impl From<PanelToPresenterId> for Uuid {
    fn from(id: PanelToPresenterId) -> Uuid {
        id.0.into()
    }
}

impl crate::entity::TypedId for PanelToPresenterId {
    type EntityType = PanelToPresenterEntityType;
    fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }
    fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

/// PanelToPresenter edge-entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PanelToPresenter)]
pub struct PanelToPresenter {
    /// UUID of the panel (from side)
    #[field(display = "Panel UUID", description = "UUID of the panel")]
    #[required]
    pub panel_uuid: NonNilUuid,

    /// UUID of the presenter (to side)
    #[field(display = "Presenter UUID", description = "UUID of the presenter")]
    #[required]
    pub presenter_uuid: NonNilUuid,
}

impl PanelToPresenterData {
    /// Get the panel ID from this edge
    pub fn panel_id(&self) -> crate::entity::PanelId {
        crate::entity::PanelId::from_uuid(self.panel_uuid)
    }

    /// Get the presenter ID from this edge
    pub fn presenter_id(&self) -> crate::entity::PresenterId {
        crate::entity::PresenterId::from_uuid(self.presenter_uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_nn() -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]))
        }
    }

    #[test]
    fn panel_to_presenter_id_from_uuid() {
        let nn = test_nn();
        let id = PanelToPresenterId::from(nn);
        assert_eq!(NonNilUuid::from(id), nn);
    }

    #[test]
    fn panel_to_presenter_id_try_from_nil_uuid_returns_none() {
        assert!(PanelToPresenterId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn panel_to_presenter_id_display() {
        let id = PanelToPresenterId::from(test_nn());
        assert_eq!(
            id.to_string(),
            "panel-to-presenter-00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn panel_to_presenter_data_ids() {
        let panel_uuid = unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]))
        };
        let presenter_uuid = unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
            ]))
        };

        let data = PanelToPresenterData {
            entity_uuid: test_nn(),
            panel_uuid,
            presenter_uuid,
        };

        assert_eq!(data.panel_id().non_nil_uuid(), panel_uuid);
        assert_eq!(data.presenter_id().non_nil_uuid(), presenter_uuid);
    }
}
