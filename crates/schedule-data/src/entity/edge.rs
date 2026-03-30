/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge entity implementation

// @TODO: Make edges their own things they are not entities, and there should be different
// implementations of edges for different relationship types, base on relationships.rs
// etc. Probably should move/split to schedule-data/src/edges/...

use crate::entity::EntityId;
use crate::EntityFields;

/// Relationship direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipDirection {
    Outgoing, // Entity -> Related
    Incoming, // Related -> Entity
}

/// Edge types for relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeType {
    PresenterToGroup,
    PanelToPresenter,
    PanelToRoom,
    PanelToPanelType,
}

impl EdgeType {
    /// Get a human-readable name for the edge type
    /// @todo replace with ability to left left and right entity types
    pub fn name(&self) -> &'static str {
        match self {
            EdgeType::PresenterToGroup => "presenter_to_group",
            EdgeType::PanelToPresenter => "panel_to_presenter",
            EdgeType::PanelToRoom => "panel_to_room",
            EdgeType::PanelToPanelType => "panel_to_panel_type",
        }
    }
}

/// Edge entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
pub struct Edge {
    #[field(display = "From UID", description = "Source entity UID")]
    #[alias("from", "from_uid", "fromUID", "member")]
    pub from_uid: EntityId,

    #[field(display = "To UID", description = "Target entity UID")]
    #[alias("to", "to_uid", "toUID", "group")]
    #[required]
    pub to_uid: EntityId,

    #[computed_field(display = "Edge Type", description = "Type of relationship")]
    #[alias("type", "edge_type", "edgeType")]
    #[read(|entity: &Edge| {
                Some(crate::field::FieldValue::String(
                    match entity.edge_type {
                        EdgeType::PresenterToGroup => "presenter_to_group",
                        EdgeType::PanelToPresenter => "panel_to_presenter",
                        EdgeType::PanelToRoom => "panel_to_room",
                        EdgeType::PanelToPanelType => "panel_to_panel_type",
                    }
                    .to_string()
                ))
            })]
    #[write(|entity: &mut Edge, value: crate::field::FieldValue| {
                if let crate::field::FieldValue::String(type_str) = value {
                    match type_str.as_str() {
                        "presenter_to_group" => {
                            entity.edge_type = EdgeType::PresenterToGroup;
                            Ok(())
                        }
                        "panel_to_presenter" => {
                            entity.edge_type = EdgeType::PanelToPresenter;
                            Ok(())
                        }
                        "panel_to_room" => {
                            entity.edge_type = EdgeType::PanelToRoom;
                            Ok(())
                        }
                        "panel_to_panel_type" => {
                            entity.edge_type = EdgeType::PanelToPanelType;
                            Ok(())
                        }
                        _ => Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
                    }
                } else {
                    Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
                }
            })]
    pub edge_type: EdgeType,

    #[field(display = "Metadata", description = "Additional metadata")]
    #[alias("meta", "metadata")]
    pub metadata: std::collections::HashMap<String, crate::field::FieldValue>,
}
