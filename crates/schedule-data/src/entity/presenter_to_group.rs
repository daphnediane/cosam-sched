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
use uuid::NonNilUuid;

/// PresenterToGroup edge-entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PresenterToGroup)]
pub struct PresenterToGroup {
    /// UUID of the member presenter (from side)
    #[field(display = "Member UUID", description = "UUID of the member presenter")]
    #[required]
    #[edge_from(Presenter, accessor = member_id)]
    pub member_uuid: NonNilUuid,

    /// UUID of the group presenter (to side)
    #[field(display = "Group UUID", description = "UUID of the group presenter")]
    #[required]
    #[edge_to(Presenter, accessor = group_id)]
    pub group_uuid: NonNilUuid,

    /// Whether this member should always be shown when the group is displayed
    #[field(
        display = "Always Shown in Group",
        description = "Whether always shown in group"
    )]
    pub always_shown_in_group: bool,

    /// Whether this presenter should always be grouped with this group
    #[field(display = "Always Grouped", description = "Whether always grouped")]
    pub always_grouped: bool,
}

impl PresenterToGroupData {
    /// Check if this is a self-loop (group marker edge)
    pub fn is_self_loop(&self) -> bool {
        self.member_uuid == self.group_uuid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::{NonNilUuid, Uuid};

    fn test_nn() -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]))
        }
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
        assert_eq!(
            id.to_string(),
            "presenter-to-group-00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn presenter_to_group_data_ids() {
        let member_uuid = unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]))
        };
        let group_uuid = unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
            ]))
        };

        let data = PresenterToGroupData {
            entity_uuid: test_nn(),
            member_uuid,
            group_uuid,
            always_shown_in_group: true,
            always_grouped: false,
        };

        assert_eq!(data.member_id().non_nil_uuid(), member_uuid);
        assert_eq!(data.group_id().non_nil_uuid(), group_uuid);
        assert!(!data.is_self_loop());
    }

    #[test]
    fn presenter_to_group_self_loop_detection() {
        let uuid = unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]))
        };

        let data = PresenterToGroupData {
            entity_uuid: test_nn(),
            member_uuid: uuid,
            group_uuid: uuid,
            always_shown_in_group: true,
            always_grouped: true,
        };

        assert!(data.is_self_loop());
    }
}
