/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge entity implementation

use crate::entity::EntityType;
use crate::field::field_set::FieldSet;
use crate::field::traits::*;
use crate::field::{FieldValue, ValidationError};
use std::fmt;

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
    /// Get the reverse edge type
    pub fn reverse(&self) -> Self {
        match self {
            EdgeType::PresenterToGroup => EdgeType::PresenterToGroup, // Symmetric
            EdgeType::PanelToPresenter => EdgeType::PanelToPresenter, // Symmetric
            EdgeType::PanelToRoom => EdgeType::PanelToRoom,           // Symmetric
            EdgeType::PanelToPanelType => EdgeType::PanelToPanelType, // Symmetric
        }
    }

    /// Get a human-readable name for the edge type
    pub fn name(&self) -> &'static str {
        match self {
            EdgeType::PresenterToGroup => "presenter_to_group",
            EdgeType::PanelToPresenter => "panel_to_presenter",
            EdgeType::PanelToRoom => "panel_to_room",
            EdgeType::PanelToPanelType => "panel_to_panel_type",
        }
    }
}

/// Edge ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(uuid::Uuid);

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "edge-{}", self.0)
    }
}

/// Edge entity
#[derive(Debug, Clone)]
pub struct Edge {
    pub uid: EdgeId,
    pub from_uid: String,
    pub to_uid: String,
    pub edge_type: EdgeType,
    pub metadata: std::collections::HashMap<String, FieldValue>,
}

/// Field constants for Edge
pub mod edge_fields {
    use super::{Edge, EdgeType};
    use crate::entity::EntityType;
    use crate::field::traits::*;
    use crate::field::{FieldError, FieldValue, ValidationError};

    // Import macros from the dedicated macros module
    use crate::entity::macros::{computed_field, direct_field};

    // FROM_UID field - optional (valid to be empty for PresenterToGroup)
    direct_field!(
        FromUidField,
        "From UID",
        "Source entity UID",
        Edge,
        from_uid,
        String
    );

    // TO_UID field - required
    direct_field!(
        ToUidField,
        "To UID",
        "Target entity UID",
        Edge,
        to_uid,
        String
    );

    // EDGE_TYPE field - required - uses enhanced macro for enum conversion
    computed_field!(
        EdgeTypeField,
        "Edge Type",
        "Type of relationship edge",
        Edge,
        {
            read: |self, entity| {
                Some(FieldValue::String(
                    match entity.edge_type {
                        EdgeType::PresenterToGroup => "presenter_to_group",
                        EdgeType::PanelToPresenter => "panel_to_presenter",
                        EdgeType::PanelToRoom => "panel_to_room",
                        EdgeType::PanelToPanelType => "panel_to_panel_type",
                    }
                    .to_string(),
                ))
            },
            write: |self, entity, value| {
                if let FieldValue::String(v) = value {
                    entity.edge_type = match v.as_str() {
                        "presenter_to_group" => EdgeType::PresenterToGroup,
                        "panel_to_presenter" => EdgeType::PanelToPresenter,
                        "panel_to_room" => EdgeType::PanelToRoom,
                        "panel_to_panel_type" => EdgeType::PanelToPanelType,
                        _ => {
                            return Err(FieldError::ValidationError(
                                ValidationError::ValidationFailed {
                                    field: "edge_type".to_string(),
                                    reason: format!("unknown edge type '{}'", v),
                                },
                            ))
                        }
                    };
                    Ok(())
                } else {
                    Err(FieldError::CannotStoreComputedField)
                }
            }
        }
    );

    pub static FROM_UID: FromUidField = FromUidField;
    pub static TO_UID: ToUidField = ToUidField;
    pub static EDGE_TYPE: EdgeTypeField = EdgeTypeField;
}

impl Edge {
    pub fn field_set() -> &'static FieldSet<Edge> {
        use std::sync::LazyLock;

        // Import macros from the dedicated macros module
        use crate::entity::macros::field_set;

        static FIELD_SET: LazyLock<FieldSet<Edge>> = field_set!(Edge, {
            fields: [
                &edge_fields::FROM_UID => ["member", "fromUID", "from_uid", "from"],
                &edge_fields::TO_UID => ["group", "toUID", "to_uid", "to"],
                &edge_fields::EDGE_TYPE => ["type", "edge_type", "edgeType"]
            ],
            required: ["to_uid", "edge_type"]
        });

        &FIELD_SET
    }
}

impl EntityType for Edge {
    type Data = Edge;

    const TYPE_NAME: &'static str = "edge";

    fn field_set() -> &'static FieldSet<Self> {
        Edge::field_set()
    }

    fn validate(data: &Self::Data) -> Result<(), ValidationError> {
        // TO_UID is always required
        if data.to_uid.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "to_uid".to_string(),
            });
        }

        // EDGE_TYPE is always required (implicit validation through enum)

        // FROM_UID is only required for non-PresenterToGroup edges
        if data.edge_type != EdgeType::PresenterToGroup && data.from_uid.is_empty() {
            return Err(ValidationError::RequiredFieldMissing {
                field: "from_uid".to_string(),
            });
        }

        Ok(())
    }
}
