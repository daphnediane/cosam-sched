/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelToPresenter edge implementation

use crate::edge::{Edge, EdgeType, SimpleEdge};
use crate::entity::EntityId;

/// PanelToPresenter edge implementation
#[derive(Debug, Clone)]
pub struct PanelToPresenterEdge {
    pub from_id: EntityId, // Panel
    pub to_id: EntityId,   // Presenter
    pub data: PanelToPresenterData,
}

#[derive(Debug, Clone)]
pub struct PanelToPresenterData {
    // No additional data needed for this simple relationship
}

impl PanelToPresenterEdge {
    pub fn new(panel_id: EntityId, presenter_id: EntityId) -> Self {
        Self {
            from_id: panel_id,
            to_id: presenter_id,
            data: PanelToPresenterData {},
        }
    }
}

impl Edge for PanelToPresenterEdge {
    type FromEntity = crate::entity::PanelEntityType;
    type ToEntity = crate::entity::PresenterEntityType;
    type Data = PanelToPresenterData;

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
        EdgeType::PanelToPresenter
    }
}

impl SimpleEdge for PanelToPresenterEdge {
    fn is_bidirectional(&self) -> bool {
        false
    }
}
