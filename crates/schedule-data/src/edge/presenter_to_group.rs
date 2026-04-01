/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PresenterToGroup edge implementation

use crate::edge::{Edge, EdgeType, SimpleEdge};
use crate::entity::EntityId;

/// PresenterToGroup edge implementation (based on GroupEdge from schedule-core)
#[derive(Debug, Clone)]
pub struct PresenterToGroupEdge {
    pub from_id: EntityId, // Member presenter
    pub to_id: EntityId,   // Group presenter
    pub data: PresenterToGroupData,
}

#[derive(Debug, Clone)]
pub struct PresenterToGroupData {
    pub always_grouped: bool,        // Member should always appear with group
    pub always_shown_in_group: bool, // Group should always be shown as group
}

impl PresenterToGroupEdge {
    /// Create a new presenter-group edge
    pub fn new(
        from_id: EntityId,
        to_id: EntityId,
        always_grouped: bool,
        always_shown_in_group: bool,
    ) -> Self {
        Self {
            from_id,
            to_id,
            data: PresenterToGroupData {
                always_grouped,
                always_shown_in_group,
            },
        }
    }

    /// Create an edge for a group with unknown members (G:==Group syntax)
    pub fn group_only(group_id: EntityId, always_shown_in_group: bool) -> Self {
        Self::new(0, group_id, false, always_shown_in_group)
    }

    /// Check if this edge represents a group with unknown members
    pub fn is_group_only(&self) -> bool {
        self.from_id == 0
    }
}

impl Edge for PresenterToGroupEdge {
    type FromEntity = crate::entity::PresenterEntityType;
    type ToEntity = crate::entity::PresenterEntityType;
    type Data = PresenterToGroupData;

    fn from_id(&self) -> EntityId {
        self.from_id
    }

    fn to_id(&self) -> EntityId {
        self.to_id
    }

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.data
    }

    fn edge_type(&self) -> EdgeType {
        EdgeType::PresenterToGroup
    }
}

impl SimpleEdge for PresenterToGroupEdge {
    fn is_bidirectional(&self) -> bool {
        false
    }
}
