/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use serde::{Deserialize, Serialize};

use super::panel::Panel;
use super::source_info::ChangeState;

/// Groups a set of related panels under a common base ID.
///
/// In the flat model every panel is fully self-contained; `PanelSet` is only
/// a thin grouping container keyed by `base_id` (e.g. `"GP002"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelSet {
    pub base_id: String,
    pub panels: Vec<Panel>,
    #[serde(skip)]
    pub change_state: ChangeState,
}

impl PanelSet {
    /// Create a new empty PanelSet.
    pub fn new(base_id: impl Into<String>) -> Self {
        Self {
            base_id: base_id.into(),
            panels: Vec::new(),
            change_state: ChangeState::Unchanged,
        }
    }

    /// Find a panel by its full Uniq ID.
    pub fn get_panel(&self, id: &str) -> Option<&Panel> {
        self.panels.iter().find(|p| p.id == id)
    }

    /// Find a panel by its full Uniq ID (mutable).
    pub fn get_panel_mut(&mut self, id: &str) -> Option<&mut Panel> {
        self.panels.iter_mut().find(|p| p.id == id)
    }
}
