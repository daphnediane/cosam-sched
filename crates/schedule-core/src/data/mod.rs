/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

pub mod display_export;
pub mod event;
pub mod panel;
pub mod panel_id;
pub mod panel_set;
pub mod panel_type;
pub mod post_process;
pub mod presenter;
pub mod room;
pub mod schedule;
pub mod source_info;
pub mod time;
pub mod timeline;
pub mod widget_embed;

pub use event::Event;
pub use panel::{ExtraFields, ExtraValue, FormulaValue, Panel};
pub use panel_id::PanelId;
pub use panel_set::PanelSet;
pub use panel_type::PanelType;
pub use post_process::apply_schedule_parity;
pub use presenter::{Presenter, PresenterRank};
pub use room::Room;
pub use schedule::{Meta, Schedule, SessionDisplayInfo};
pub use source_info::{ChangeState, ImportedSheetPresence, SourceInfo};
pub use timeline::TimelineEntry;
pub use widget_embed::{
    WidgetSources, generate_embed_html, generate_preview_html, generate_test_html,
    write_embed_html, write_test_html,
};
