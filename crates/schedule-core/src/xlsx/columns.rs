/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Per-table XLSX column header definitions.
//!
//! Each sub-module (`schedule`, `room_map`, `panel_types`, `people`) contains
//! [`FieldDef`] constants for every recognized column, plus an `ALL` slice in
//! 2026 spreadsheet order.  Use [`FieldDef::keys`] when building a lookup map
//! and [`FieldDef::export`] when writing the header row.

/// A formula-only column in an XLSX sheet.
///
/// Formula columns are written as spreadsheet formulas on export and stored in
/// the sidecar (not the CRDT) on import. They carry no user-editable data.
///
/// `regenerate: Some(formula)` — always write this formula string on export,
/// regardless of what was in the sidecar.
/// `regenerate: None` — preserve the imported formula string; skip if absent.
#[derive(Debug, Clone, Copy)]
pub struct FormulaColumnDef {
    /// The primary column header string (export name).
    pub export: &'static str,
    /// Canonical import key (result of `canonical_header(export)`).
    pub canonical: &'static str,
    /// Alternative import names.
    pub aliases: &'static [&'static str],
    /// Formula template to regenerate on export, or `None` to preserve only.
    pub regenerate: Option<&'static str>,
}

impl FormulaColumnDef {
    /// Iterate over all accepted lookup keys (primary canonical + aliases).
    pub fn keys(&self) -> impl Iterator<Item = &'static str> {
        std::iter::once(self.canonical).chain(self.aliases.iter().copied())
    }
}

/// A single column definition: one export name plus zero or more import aliases.
#[derive(Debug, Clone, Copy)]
pub struct FieldDef {
    /// Header string written into the spreadsheet when creating a new file.
    pub export: &'static str,
    /// Primary canonical lookup key; equals `canonical_header(self.export)`.
    pub canonical: &'static str,
    /// Extra lookup keys accepted during import in addition to `canonical`.
    pub aliases: &'static [&'static str],
}

impl FieldDef {
    /// Iterate over all accepted lookup keys (primary canonical + aliases).
    pub fn keys(&self) -> impl Iterator<Item = &'static str> {
        std::iter::once(self.canonical).chain(self.aliases.iter().copied())
    }
}

// ─── Schedule table ──────────────────────────────────────────────────────────

/// Column definitions for the main **Schedule** sheet.
pub mod schedule {
    use super::{FieldDef, FormulaColumnDef};

    pub const UNIQ_ID: FieldDef = FieldDef {
        export: "Uniq ID",
        canonical: "Uniq_ID",
        aliases: &["UniqID", "ID", "Id", "Code"],
    };

    pub const OLD_UNIQ_ID: FieldDef = FieldDef {
        export: "Old Uniq Id",
        canonical: "Old_Uniq_Id",
        aliases: &["OldUniqId", "Old_ID", "OldID"],
    };

    pub const NAME: FieldDef = FieldDef {
        export: "Name",
        canonical: "Name",
        aliases: &["Panel_Name", "PanelName", "Title"],
    };

    pub const ROOM: FieldDef = FieldDef {
        export: "Room",
        canonical: "Room",
        aliases: &["Room_Name", "Location"],
    };

    pub const START_TIME: FieldDef = FieldDef {
        export: "Start Time",
        canonical: "Start_Time",
        aliases: &["StartTime", "Start", "Begin"],
    };

    pub const DURATION: FieldDef = FieldDef {
        export: "Duration",
        canonical: "Duration",
        aliases: &["Length"],
    };

    pub const END_TIME: FieldDef = FieldDef {
        export: "End Time",
        canonical: "End_Time",
        aliases: &["EndTime", "End"],
    };

    pub const DESCRIPTION: FieldDef = FieldDef {
        export: "Description",
        canonical: "Description",
        aliases: &["Desc"],
    };

    pub const PREREQ: FieldDef = FieldDef {
        export: "Prereq",
        canonical: "Prereq",
        aliases: &["Prerequisite", "Prerequisites", "Pre_Req"],
    };

    pub const NOTE: FieldDef = FieldDef {
        export: "Note",
        canonical: "Note",
        aliases: &["Notes"],
    };

    pub const NOTES_NON_PRINTING: FieldDef = FieldDef {
        export: "Notes (Non Printing)",
        canonical: "Notes_Non_Printing",
        aliases: &["NotesNonPrinting"],
    };

    pub const WORKSHOP_NOTES: FieldDef = FieldDef {
        export: "Workshop Notes",
        canonical: "Workshop_Notes",
        aliases: &["WorkshopNotes"],
    };

    pub const POWER_NEEDS: FieldDef = FieldDef {
        export: "Power Needs",
        canonical: "Power_Needs",
        aliases: &["PowerNeeds"],
    };

    pub const SEWING_MACHINES: FieldDef = FieldDef {
        export: "Sewing Machines",
        canonical: "Sewing_Machines",
        aliases: &["SewingMachines"],
    };

    pub const AV_NOTES: FieldDef = FieldDef {
        export: "AV Notes",
        canonical: "AV_Notes",
        aliases: &["AVNotes", "AV_Note", "AVNote", "AV"],
    };

    pub const DIFFICULTY: FieldDef = FieldDef {
        export: "Difficulty",
        canonical: "Difficulty",
        aliases: &["Level"],
    };

    pub const COST: FieldDef = FieldDef {
        export: "Cost",
        canonical: "Cost",
        aliases: &["Price", "Fee"],
    };

    pub const SEATS_SOLD: FieldDef = FieldDef {
        export: "Seats Sold",
        canonical: "Seats_Sold",
        aliases: &["SeatsSold"],
    };

    pub const PRE_REG_MAX: FieldDef = FieldDef {
        export: "Prereg Max",
        canonical: "Prereg_Max",
        aliases: &["Pre_Reg_Max", "PreRegMax", "Pre-Reg_Max", "Pre-Reg Max"],
    };

    pub const CAPACITY: FieldDef = FieldDef {
        export: "Capacity",
        canonical: "Capacity",
        aliases: &["Cap", "Max"],
    };

    pub const HAVE_TICKET_IMAGE: FieldDef = FieldDef {
        export: "Have Ticket Image",
        canonical: "Have_Ticket_Image",
        aliases: &["HaveTicketImage"],
    };

    pub const SIMPLE_TIX_EVENT: FieldDef = FieldDef {
        export: "SimpleTix Event",
        canonical: "Simple_Tix_Event",
        aliases: &["SimpleTixEvent"],
    };

    pub const TICKET_SALE: FieldDef = FieldDef {
        export: "Ticket Sale",
        canonical: "Ticket_Sale",
        aliases: &["TicketSale", "Tickets", "Sale"],
    };

    pub const TICKET_URL: FieldDef = FieldDef {
        export: "Ticket URL",
        canonical: "Ticket_URL",
        aliases: &["TicketURL", "Ticket_Url", "TicketUrl"],
    };

    pub const HIDE_PANELIST: FieldDef = FieldDef {
        export: "Hide Panelist",
        canonical: "Hide_Panelist",
        aliases: &["HidePanelist", "Hide_Presenter"],
    };

    pub const ALT_PANELIST: FieldDef = FieldDef {
        export: "Alt Panelist",
        canonical: "Alt_Panelist",
        aliases: &["AltPanelist", "Alt_Presenter", "Alt"],
    };

    pub const KIND: FieldDef = FieldDef {
        export: "Kind",
        canonical: "Kind",
        aliases: &["Type", "Panel_Type", "PanelType", "Prefix"],
    };

    pub const FULL: FieldDef = FieldDef {
        export: "Full",
        canonical: "Full",
        aliases: &["IsFull", "Is_Full", "Sold_Out"],
    };

    /// All fixed (non-presenter) column definitions in 2026 spreadsheet order.
    ///
    /// `LSTART` and `LEND` are intentionally excluded — they are formula columns
    /// handled by [`FORMULA_COLUMNS`] and stored in the sidecar, not mapped to
    /// entity fields.
    pub const ALL: &[FieldDef] = &[
        UNIQ_ID,
        OLD_UNIQ_ID,
        NAME,
        ROOM,
        START_TIME,
        DURATION,
        // END_TIME is intentionally excluded: it is *accepted* on import (an
        // author may give an end instead of a duration) but never written —
        // start + duration is the only canonical timing pair exported. Writing
        // a redundant end risks `start + duration != end` after edits.
        DESCRIPTION,
        PREREQ,
        NOTE,
        NOTES_NON_PRINTING,
        WORKSHOP_NOTES,
        POWER_NEEDS,
        SEWING_MACHINES,
        AV_NOTES,
        DIFFICULTY,
        COST,
        SEATS_SOLD,
        PRE_REG_MAX,
        CAPACITY,
        HAVE_TICKET_IMAGE,
        SIMPLE_TIX_EVENT,
        TICKET_SALE,
        TICKET_URL,
        HIDE_PANELIST,
        ALT_PANELIST,
        KIND,
        FULL,
    ];

    /// Columns accepted on import but **never written** on export.
    ///
    /// These are recognized so they aren't routed to the sidecar as unknown
    /// extra fields, but they carry no canonical data: `END_TIME` is folded into
    /// the start + duration pair (see [`ALL`]).
    pub const READ_ONLY: &[FieldDef] = &[END_TIME];

    /// Formula-only columns appended after all data columns on export.
    ///
    /// These are not backed by entity fields; they are regenerated from the
    /// formula template on every export and preserved in the sidecar on import.
    pub const FORMULA_COLUMNS: &[FormulaColumnDef] = &[
        FormulaColumnDef {
            export: "Lstart",
            canonical: "Lstart",
            aliases: &[],
            regenerate: Some(
                "IF(ISBLANK([@[Start Time]]),MAX([Start Time])+TIME(80,0,0),[@[Start Time]])",
            ),
        },
        FormulaColumnDef {
            export: "Lend",
            canonical: "Lend",
            aliases: &[],
            regenerate: Some("[@Lstart]+IF(ISBLANK([@Duration]),0,[@Duration])"),
        },
    ];
}

// ─── Room map table ───────────────────────────────────────────────────────────

/// Column definitions for the **Rooms** / **RoomMap** sheet.
pub mod room_map {
    use super::FieldDef;

    pub const ROOM_NAME: FieldDef = FieldDef {
        export: "Room Name",
        canonical: "Room_Name",
        aliases: &["Room", "Name", "Short_Name", "ShortName"],
    };

    pub const SORT_KEY: FieldDef = FieldDef {
        export: "Sort Key",
        canonical: "Sort_Key",
        aliases: &["SortKey", "Sort", "Order"],
    };

    pub const LONG_NAME: FieldDef = FieldDef {
        export: "Long Name",
        canonical: "Long_Name",
        aliases: &["LongName", "Full_Name", "FullName"],
    };

    pub const HOTEL_ROOM: FieldDef = FieldDef {
        export: "Hotel Room",
        canonical: "Hotel_Room",
        aliases: &["HotelRoom", "Hotel", "Building"],
    };

    pub const IS_PSEUDO: FieldDef = FieldDef {
        export: "Is Pseudo",
        canonical: "Is_Pseudo",
        aliases: &["IsPseudo", "Pseudo", "Fake"],
    };

    /// All primary column definitions in 2026 spreadsheet order.
    pub const ALL: &[FieldDef] = &[ROOM_NAME, SORT_KEY, LONG_NAME, HOTEL_ROOM, IS_PSEUDO];
}

// ─── Panel types table ────────────────────────────────────────────────────────

/// Column definitions for the **PanelTypes** / **Prefix** sheet.
pub mod panel_types {
    use super::FieldDef;

    pub const PREFIX: FieldDef = FieldDef {
        export: "Prefix",
        canonical: "Prefix",
        aliases: &["ID", "Id", "Tag"],
    };

    pub const PANEL_KIND: FieldDef = FieldDef {
        export: "Panel Kind",
        canonical: "Panel_Kind",
        aliases: &["PanelKind", "Kind", "Name", "Type"],
    };

    pub const COLOR: FieldDef = FieldDef {
        export: "Color",
        canonical: "Color",
        aliases: &["Colour", "BgColor", "Bg_Color"],
    };

    pub const BW_COLOR: FieldDef = FieldDef {
        export: "BW",
        canonical: "BW",
        aliases: &["Bw", "BwColor", "Bw_Color", "Grayscale"],
    };

    pub const HIDDEN: FieldDef = FieldDef {
        export: "Hidden",
        canonical: "Hidden",
        aliases: &["IsHidden", "Is_Hidden", "Hide"],
    };

    pub const IS_TIMELINE: FieldDef = FieldDef {
        export: "Is Timeline",
        canonical: "Is_Timeline",
        // "Is_Time_Line" is what canonical_header produces for old "IsTimeLine"
        aliases: &["Is_Time_Line", "IsTimeLine", "IsTimeline", "Timeline"],
    };

    pub const IS_PRIVATE: FieldDef = FieldDef {
        export: "Is Private",
        canonical: "Is_Private",
        aliases: &["IsPrivate", "Private"],
    };

    pub const IS_BREAK: FieldDef = FieldDef {
        export: "Is Break",
        canonical: "Is_Break",
        aliases: &["IsBreak", "Break"],
    };

    pub const IS_WORKSHOP: FieldDef = FieldDef {
        export: "Is Workshop",
        canonical: "Is_Workshop",
        aliases: &["IsWorkshop", "Workshop"],
    };

    pub const IS_ROOM_HOURS: FieldDef = FieldDef {
        export: "Is Room Hours",
        canonical: "Is_Room_Hours",
        aliases: &["IsRoomHours", "Room_Hours", "RoomHours"],
    };

    pub const IS_CAFE: FieldDef = FieldDef {
        export: "Is Café",
        canonical: "Is_Café",
        aliases: &["Is_Cafe", "IsCafe", "IsCafé", "Cafe", "Café"],
    };

    /// All column definitions in 2026 spreadsheet order.
    pub const ALL: &[FieldDef] = &[
        PREFIX,
        PANEL_KIND,
        COLOR,
        BW_COLOR,
        HIDDEN,
        IS_TIMELINE,
        IS_PRIVATE,
        IS_BREAK,
        IS_WORKSHOP,
        IS_ROOM_HOURS,
        IS_CAFE,
    ];
}

// ─── Hotel rooms table ───────────────────────────────────────────────────────

/// Column definitions for the **Hotels** / **Hotel Rooms** sheet.
pub mod hotel_rooms {
    use super::FieldDef;

    pub const HOTEL_ROOM_NAME: FieldDef = FieldDef {
        export: "Hotel Room",
        canonical: "Hotel_Room",
        aliases: &["HotelRoom", "Name", "Room"],
    };

    pub const SORT_KEY: FieldDef = FieldDef {
        export: "Sort Key",
        canonical: "Sort_Key",
        aliases: &["SortKey", "Sort", "Order"],
    };

    pub const LONG_NAME: FieldDef = FieldDef {
        export: "Long Name",
        canonical: "Long_Name",
        aliases: &["LongName", "Full_Name", "FullName"],
    };

    /// All column definitions in export order.
    pub const ALL: &[FieldDef] = &[HOTEL_ROOM_NAME, SORT_KEY, LONG_NAME];
}

// ─── Timeline table ──────────────────────────────────────────────────────────

/// Column definitions for the **Timeline** sheet.
pub mod timeline {
    use super::FieldDef;

    // UNIQ_ID/DESCRIPTION/NOTE are identical to the Schedule sheet's, so reuse
    // them (a `FieldDef` is a plain `const`). Only NAME and TIME differ — a
    // timeline is a single time point titled differently than a panel — and
    // PANEL_TYPES is timeline-specific (header "Panel Types", the timeline
    // analog of the Schedule sheet's "Kind" column).
    pub use super::schedule::{DESCRIPTION, NOTE, UNIQ_ID};

    pub const NAME: FieldDef = FieldDef {
        export: "Name",
        canonical: "Name",
        aliases: &["Title", "Event"],
    };

    pub const TIME: FieldDef = FieldDef {
        export: "Time",
        canonical: "Time",
        aliases: &["Start_Time", "StartTime", "Start"],
    };

    pub const PANEL_TYPES: FieldDef = FieldDef {
        export: "Panel Types",
        canonical: "Panel_Types",
        aliases: &["PanelTypes", "Kind", "Type", "Prefix"],
    };

    /// All column definitions in export order.
    pub const ALL: &[FieldDef] = &[UNIQ_ID, NAME, DESCRIPTION, NOTE, TIME, PANEL_TYPES];
}

// ─── Breaks table ────────────────────────────────────────────────────────────

/// Column definitions for the optional **Breaks** sheet.
///
/// Breaks parallel timelines but carry a duration (start + end/duration), so the
/// sheet has the extra timing columns a single-time-point timeline lacks.
pub mod breaks {
    use super::FieldDef;

    // A break's columns are a blend of two existing sheets, and a `FieldDef` is
    // a plain `const`, so reuse them rather than redeclaring:
    //   - non-timing columns are identical to the Timeline sheet's;
    //   - the timing trio (start/end/duration) is identical to the Schedule
    //     sheet's (e.g. `Begin` is a start alias; duration is just `Length`).
    //
    // There is intentionally no `Panel Types`/`Kind` column: a break's type is
    // always derived from its Uniq ID prefix (e.g. `BREAK001` → `BR`).
    //
    // `END_TIME` is *accepted* on import (authors may give an end instead of a
    // duration) but is **not** in `ALL`, so it is never written: start + duration
    // is the only canonical pair exported. Writing a redundant end time risks
    // `start + duration != end` after edits, with no clean way to reconcile.
    pub use super::schedule::{DURATION, END_TIME, START_TIME};
    pub use super::timeline::{DESCRIPTION, NAME, NOTE, UNIQ_ID};

    /// Column definitions written on export, in order.
    pub const ALL: &[FieldDef] = &[UNIQ_ID, NAME, DESCRIPTION, NOTE, START_TIME, DURATION];

    /// Columns accepted on import but never written (see [`super::schedule::READ_ONLY`]).
    pub const READ_ONLY: &[FieldDef] = &[END_TIME];
}

// ─── People / Presenters table ───────────────────────────────────────────────

/// Column definitions for the **People** / **Presenters** sheet.
pub mod people {
    use super::FieldDef;

    /// Presenter name — "Person" is the 2026 header; older sheets use "Name".
    pub const NAME: FieldDef = FieldDef {
        export: "Person",
        canonical: "Person",
        aliases: &["Name", "Presenter", "Panelist", "Speaker"],
    };

    /// Classification / rank column.
    pub const CLASSIFICATION: FieldDef = FieldDef {
        export: "Classification",
        canonical: "Classification",
        aliases: &["Rank", "Type", "Role", "Level"],
    };

    pub const IS_GROUP: FieldDef = FieldDef {
        export: "Is Group",
        canonical: "Is_Group",
        aliases: &["IsGroup", "Group"],
    };

    /// Maps to `subsumes_members` on group — group appears in credits and subsumes its members.
    pub const SUBSUMES_MEMBERS: FieldDef = FieldDef {
        export: "Subsumes Members",
        canonical: "Subsumes_Members",
        aliases: &[
            "SubsumesMembers",
            "Always_Shown",
            "AlwaysShown",
            "Group_Shown",
        ],
    };

    /// Maps to `show_individually` on member — member appears individually, not subsumed by group.
    pub const SHOW_INDIVIDUALLY: FieldDef = FieldDef {
        export: "Show Individually",
        canonical: "Show_Individually",
        aliases: &[
            "ShowIndividually",
            "Always_Grouped",
            "AlwaysGrouped",
            "Always_Show_in_Group",
        ],
    };

    /// Members of this group (on a group row): comma-separated list of member names.
    /// Member names might be tagged, including rank prefixes and show individually flags.
    pub const MEMBERS: FieldDef = FieldDef {
        export: "Members",
        canonical: "Members",
        aliases: &["Member"],
    };

    /// Groups this presenter belongs to (on a member row): comma-separated list of
    /// group names.  Member names might be tagged with rank prefixes and
    /// subsumed member flags.
    pub const GROUPS: FieldDef = FieldDef {
        export: "Groups",
        canonical: "Groups",
        aliases: &["Group"],
    };

    /// All primary column definitions in export order.
    pub const ALL: &[FieldDef] = &[
        NAME,
        CLASSIFICATION,
        IS_GROUP,
        SUBSUMES_MEMBERS,
        SHOW_INDIVIDUALLY,
        MEMBERS,
        GROUPS,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_def_keys_includes_canonical_and_aliases() {
        let keys: Vec<_> = schedule::UNIQ_ID.keys().collect();
        assert!(keys.contains(&"Uniq_ID"));
        assert!(keys.contains(&"ID"));
    }

    #[test]
    fn test_room_map_export_names() {
        assert_eq!(room_map::ROOM_NAME.export, "Room Name");
        assert_eq!(room_map::SORT_KEY.export, "Sort Key");
    }

    #[test]
    fn test_is_timeline_aliases_include_old_form() {
        let keys: Vec<_> = panel_types::IS_TIMELINE.keys().collect();
        assert!(keys.contains(&"Is_Timeline"));
        assert!(keys.contains(&"Is_Time_Line"));
    }
}
