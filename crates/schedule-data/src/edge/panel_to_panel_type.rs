/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelToPanelType edge implementation

use crate::edge::{Edge, EdgeType, SimpleEdge};
use crate::entity::EntityId;

/// PanelToPanelType edge implementation
#[derive(Debug, Clone)]
pub struct PanelToPanelTypeEdge {
    pub from_id: EntityId, // Panel
    pub to_id: EntityId,   // PanelType
    pub data: PanelToPanelTypeData,
}

#[derive(Debug, Clone)]
pub struct PanelToPanelTypeData {
    // No additional data needed for this simple relationship
}

impl PanelToPanelTypeEdge {
    pub fn new(panel_id: EntityId, panel_type_id: EntityId) -> Self {
        Self {
            from_id: panel_id,
            to_id: panel_type_id,
            data: PanelToPanelTypeData {},
        }
    }
}

impl Edge for PanelToPanelTypeEdge {
    type FromEntity = crate::entity::PanelEntityType;
    type ToEntity = crate::entity::PanelTypeEntityType;
    type Data = PanelToPanelTypeData;

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
        EdgeType::PanelToPanelType
    }
}

impl SimpleEdge for PanelToPanelTypeEdge {
    fn is_bidirectional(&self) -> bool {
        false
    }
}
