/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelToPanelType edge-entity implementation
//!
//! This edge type connects panels to their panel types.
//! As an edge-entity, it has its own UUID and can store metadata.

use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::{NonNilUuid, Uuid};

/// PanelToPanelType edge ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PanelToPanelTypeId(NonNilUuid);

impl PanelToPanelTypeId {
    /// Get the NonNilUuid from this ID
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }

    /// Get the raw UUID from this ID
    pub fn uuid(&self) -> Uuid {
        self.0.into()
    }

    /// Create a PanelToPanelTypeId from a NonNilUuid (infallible)
    pub fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }

    /// Try to create a PanelToPanelTypeId from a raw UUID (boundary use only)
    pub fn try_from_raw_uuid(uuid: Uuid) -> Option<Self> {
        NonNilUuid::new(uuid).map(Self)
    }
}

impl fmt::Display for PanelToPanelTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "panel-to-panel-type-{}", self.0)
    }
}

impl From<NonNilUuid> for PanelToPanelTypeId {
    fn from(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

impl From<PanelToPanelTypeId> for NonNilUuid {
    fn from(id: PanelToPanelTypeId) -> NonNilUuid {
        id.0
    }
}

impl From<PanelToPanelTypeId> for Uuid {
    fn from(id: PanelToPanelTypeId) -> Uuid {
        id.0.into()
    }
}

impl crate::entity::TypedId for PanelToPanelTypeId {
    type EntityType = PanelToPanelTypeEntityType;
    fn non_nil_uuid(&self) -> NonNilUuid { self.0 }
    fn from_uuid(uuid: NonNilUuid) -> Self { Self(uuid) }
}

/// PanelToPanelType edge-entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PanelToPanelType)]
pub struct PanelToPanelType {
    /// UUID of the panel (from side)
    #[field(display = "Panel UUID", description = "UUID of the panel")]
    #[required]
    pub panel_uuid: NonNilUuid,

    /// UUID of the panel type (to side)
    #[field(display = "Panel Type UUID", description = "UUID of the panel type")]
    #[required]
    pub panel_type_uuid: NonNilUuid,
}

impl PanelToPanelTypeData {
    /// Get the panel ID from this edge
    pub fn panel_id(&self) -> crate::entity::PanelId {
        crate::entity::PanelId::from_uuid(self.panel_uuid)
    }

    /// Get the panel type ID from this edge
    pub fn panel_type_id(&self) -> crate::entity::PanelTypeId {
        crate::entity::PanelTypeId::from_uuid(self.panel_type_uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_nn() -> NonNilUuid {
        unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])) }
    }

    #[test]
    fn panel_to_panel_type_id_from_uuid() {
        let nn = test_nn();
        let id = PanelToPanelTypeId::from(nn);
        assert_eq!(NonNilUuid::from(id), nn);
    }

    #[test]
    fn panel_to_panel_type_id_try_from_nil_uuid_returns_none() {
        assert!(PanelToPanelTypeId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn panel_to_panel_type_id_display() {
        let id = PanelToPanelTypeId::from(test_nn());
        assert_eq!(id.to_string(), "panel-to-panel-type-00000000-0000-0000-0000-000000000001");
    }

    #[test]
    fn panel_to_panel_type_data_ids() {
        let panel_uuid = unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])) };
        let panel_type_uuid = unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2])) };

        let data = PanelToPanelTypeData {
            entity_uuid: test_nn(),
            panel_uuid,
            panel_type_uuid,
        };

        assert_eq!(data.panel_id().non_nil_uuid(), panel_uuid);
        assert_eq!(data.panel_type_id().non_nil_uuid(), panel_type_uuid);
    }
}
