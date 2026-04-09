/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PresenterToGroup edge-entity implementation
//!
//! This edge type connects presenters to groups (self-referential).
//! As an edge-entity, it has its own UUID and stores membership flags.
//!
//! # Group Semantics
//!
//! - **Group marker edge**: A presenter marks itself as a group via self-loop
//!   - `member_uuid == group_uuid` (self-loop)
//!   - `is_group_marker = true`
//!   - `is_group_member = false`
//!
//! - **Group member edge**: A presenter joins a group
//!   - `member_uuid != group_uuid`
//!   - `is_group_marker = false`
//!   - `is_group_member = true`

use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::{NonNilUuid, Uuid};

/// PresenterToGroup edge ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PresenterToGroupId(NonNilUuid);

impl PresenterToGroupId {
    /// Get the NonNilUuid from this ID
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        self.0
    }

    /// Get the raw UUID from this ID
    pub fn uuid(&self) -> Uuid {
        self.0.into()
    }

    /// Create a PresenterToGroupId from a NonNilUuid (infallible)
    pub fn from_uuid(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }

    /// Try to create a PresenterToGroupId from a raw UUID (boundary use only)
    pub fn try_from_raw_uuid(uuid: Uuid) -> Option<Self> {
        NonNilUuid::new(uuid).map(Self)
    }
}

impl fmt::Display for PresenterToGroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "presenter-to-group-{}", self.0)
    }
}

impl From<NonNilUuid> for PresenterToGroupId {
    fn from(uuid: NonNilUuid) -> Self {
        Self(uuid)
    }
}

impl From<PresenterToGroupId> for NonNilUuid {
    fn from(id: PresenterToGroupId) -> NonNilUuid {
        id.0
    }
}

impl From<PresenterToGroupId> for Uuid {
    fn from(id: PresenterToGroupId) -> Uuid {
        id.0.into()
    }
}

impl crate::entity::TypedId for PresenterToGroupId {
    type EntityType = PresenterToGroupEntityType;
    fn non_nil_uuid(&self) -> NonNilUuid { self.0 }
    fn from_uuid(uuid: NonNilUuid) -> Self { Self(uuid) }
}

/// PresenterToGroup edge-entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PresenterToGroup)]
pub struct PresenterToGroup {
    /// UUID of the member presenter (from side)
    #[field(display = "Member UUID", description = "UUID of the member presenter")]
    #[required]
    pub member_uuid: NonNilUuid,

    /// UUID of the group presenter (to side)
    #[field(display = "Group UUID", description = "UUID of the group presenter")]
    #[required]
    pub group_uuid: NonNilUuid,

    /// @todo - these are not needed

    /// Whether this edge marks the member as a group itself
    #[field(display = "Is Group Marker", description = "Whether this marks a group")]
    pub is_group_marker: bool,

    /// Whether this edge indicates group membership
    #[field(display = "Is Group Member", description = "Whether this indicates membership")]
    pub is_group_member: bool,

    // @todo - Need always shown in group and always grouped
}

impl PresenterToGroupData {
    /// Get the member presenter ID from this edge
    pub fn member_id(&self) -> crate::entity::PresenterId {
        crate::entity::PresenterId::from_uuid(self.member_uuid)
    }

    /// Get the group presenter ID from this edge
    pub fn group_id(&self) -> crate::entity::PresenterId {
        crate::entity::PresenterId::from_uuid(self.group_uuid)
    }

    /// Check if this is a self-loop (group marker edge)
    pub fn is_self_loop(&self) -> bool {
        self.member_uuid == self.group_uuid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_nn() -> NonNilUuid {
        unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])) }
    }

    #[test]
    fn presenter_to_group_id_from_uuid() {
        let nn = test_nn();
        let id = PresenterToGroupId::from(nn);
        assert_eq!(NonNilUuid::from(id), nn);
    }

    #[test]
    fn presenter_to_group_id_try_from_nil_uuid_returns_none() {
        assert!(PresenterToGroupId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn presenter_to_group_id_display() {
        let id = PresenterToGroupId::from(test_nn());
        assert_eq!(id.to_string(), "presenter-to-group-00000000-0000-0000-0000-000000000001");
    }

    #[test]
    fn presenter_to_group_data_ids() {
        let member_uuid = unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])) };
        let group_uuid = unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2])) };

        let data = PresenterToGroupData {
            entity_uuid: test_nn(),
            member_uuid,
            group_uuid,
            is_group_marker: false,
            is_group_member: true,
        };

        assert_eq!(data.member_id().non_nil_uuid(), member_uuid);
        assert_eq!(data.group_id().non_nil_uuid(), group_uuid);
        assert!(!data.is_self_loop());
    }

    #[test]
    fn presenter_to_group_self_loop_detection() {
        let uuid = unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])) };

        let data = PresenterToGroupData {
            entity_uuid: test_nn(),
            member_uuid: uuid,
            group_uuid: uuid,
            is_group_marker: true,
            is_group_member: false,
        };

        assert!(data.is_self_loop());
    }
}
