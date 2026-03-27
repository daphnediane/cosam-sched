/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Per-table XLSX column header definitions.
//!
//! Each table has a module containing [`FieldDef`] constants for every
//! recognized column.  A `FieldDef` carries:
//! - `export`    – the header string written when creating a file (matches the
//!                 real 2026 spreadsheet column names).
//! - `canonical` – the lookup key produced by `canonical_header(export)`.
//! - `aliases`   – additional lookup keys accepted during import; may be raw
//!                 spreadsheet strings or canonical forms from older files.
//!
//! Use `FieldDef::keys()` to iterate over all accepted lookup keys (primary
//! + aliases) when building a lookup map, and `FieldDef::export` when writing
//! the header row.

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
    use super::FieldDef;

    pub const UNIQ_ID: FieldDef = FieldDef {
        export: "Uniq ID",
        canonical: "Uniq_ID",
        aliases: &["UniqID", "ID", "Id"],
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
        aliases: &["NotesNonPrinting", "Notes_Non_Printing"],
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

    pub const TICKET_URL: FieldDef = FieldDef {
        export: "Ticket URL",
        canonical: "Ticket_URL",
        aliases: &["TicketURL", "Ticket_Url", "TicketUrl"],
    };

    pub const IS_FREE: FieldDef = FieldDef {
        export: "Is Free",
        canonical: "Is_Free",
        aliases: &["IsFree", "Free"],
    };

    pub const IS_KIDS: FieldDef = FieldDef {
        export: "Is Kids",
        canonical: "Is_Kids",
        aliases: &["IsKids", "Kids", "For_Kids"],
    };

    pub const LSTART: FieldDef = FieldDef {
        export: "Lstart",
        canonical: "Lstart",
        aliases: &[],
    };

    pub const LEND: FieldDef = FieldDef {
        export: "Lend",
        canonical: "Lend",
        aliases: &[],
    };

    /// All fixed (non-presenter) column definitions in 2026 spreadsheet order.
    pub const ALL: &[FieldDef] = &[
        UNIQ_ID,
        OLD_UNIQ_ID,
        NAME,
        ROOM,
        START_TIME,
        DURATION,
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
        HIDE_PANELIST,
        ALT_PANELIST,
        END_TIME,
        KIND,
        FULL,
        TICKET_URL,
        IS_FREE,
        IS_KIDS,
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

    /// All primary column definitions in 2026 spreadsheet order.
    pub const ALL: &[FieldDef] = &[ROOM_NAME, SORT_KEY, LONG_NAME, HOTEL_ROOM];

    // Extra columns present in 2026 spreadsheets — recognized but stored as
    // room metadata rather than first-class struct fields.
    pub const NAME_ALT: FieldDef = FieldDef {
        export: "Name Alt",
        canonical: "Name_Alt",
        aliases: &["NameAlt"],
    };

    pub const SUFFIX: FieldDef = FieldDef {
        export: "Suffix",
        canonical: "Suffix",
        aliases: &[],
    };

    pub const ORIG_SORT: FieldDef = FieldDef {
        export: "Orig Sort",
        canonical: "Orig_Sort",
        aliases: &["OrigSort"],
    };

    pub const ORIG_SUFFIX: FieldDef = FieldDef {
        export: "Orig Suffix",
        canonical: "Orig_Suffix",
        aliases: &["OrigSuffix"],
    };

    pub const NOTES: FieldDef = FieldDef {
        export: "Notes",
        canonical: "Notes",
        aliases: &[],
    };

    /// Extra metadata columns (outside ALL — not first-class struct fields).
    pub const EXTRA: &[FieldDef] = &[NAME_ALT, SUFFIX, ORIG_SORT, ORIG_SUFFIX, NOTES];
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
        // "Is_Time_Line" is what canonical_header produces for old "IsTimeLine" / "Is_TimeLine"
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

// ─── People table ─────────────────────────────────────────────────────────────

/// Column definitions for the **People** / **Presenters** sheet.
pub mod people {
    use super::FieldDef;

    /// Person name column — "Person" is the 2026 header; "Name", "Panelist",
    /// "Presenter" are accepted aliases from older or alternate formats.
    pub const NAME: FieldDef = FieldDef {
        export: "Person",
        canonical: "Person",
        aliases: &["Name", "Panelist", "Presenter", "Speaker"],
    };

    /// Classification / rank column — "Classification" is the 2026 header.
    /// Note: Classification values (e.g. "Sponsor") don't always match
    /// PresenterRank::as_str() — use PresenterRank::from_classification().
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

    pub const MEMBERS: FieldDef = FieldDef {
        export: "Members",
        canonical: "Members",
        aliases: &["Group_Members", "GroupMembers"],
    };

    pub const GROUPS: FieldDef = FieldDef {
        export: "Groups",
        canonical: "Groups",
        aliases: &["Member_Of", "MemberOf"],
    };

    pub const ALWAYS_GROUPED: FieldDef = FieldDef {
        export: "Always Grouped",
        canonical: "Always_Grouped",
        aliases: &[
            "AlwaysGrouped",
            "Always_In_Group",
            "Always_Show_In_Group",
            "AlwaysShowInGroup",
        ],
    };

    pub const ALWAYS_SHOWN: FieldDef = FieldDef {
        export: "Always Shown",
        canonical: "Always_Shown",
        aliases: &["AlwaysShown", "Always_Visible", "Group_Shown", "GroupShown"],
    };

    /// All column definitions in export order.
    pub const ALL: &[FieldDef] = &[
        NAME,
        CLASSIFICATION,
        IS_GROUP,
        MEMBERS,
        GROUPS,
        ALWAYS_GROUPED,
        ALWAYS_SHOWN,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_def_keys_includes_canonical() {
        let keys: Vec<_> = schedule::UNIQ_ID.keys().collect();
        assert!(keys.contains(&"Uniq_ID"));
    }

    #[test]
    fn test_field_def_keys_includes_aliases() {
        let keys: Vec<_> = schedule::UNIQ_ID.keys().collect();
        assert!(keys.contains(&"ID"));
    }

    #[test]
    fn test_room_map_export_names() {
        assert_eq!(room_map::ROOM_NAME.export, "Room Name");
        assert_eq!(room_map::SORT_KEY.export, "Sort Key");
    }

    #[test]
    fn test_panel_types_all_count() {
        assert_eq!(panel_types::ALL.len(), 11);
    }

    #[test]
    fn test_people_export_names() {
        assert_eq!(people::NAME.export, "Person");
        assert_eq!(people::CLASSIFICATION.export, "Classification");
    }

    #[test]
    fn test_people_name_aliases() {
        let keys: Vec<_> = people::NAME.keys().collect();
        assert!(keys.contains(&"Person"));
        assert!(keys.contains(&"Name"));
        assert!(keys.contains(&"Panelist"));
        assert!(keys.contains(&"Presenter"));
    }

    #[test]
    fn test_note_columns_are_separate() {
        let note_keys: Vec<_> = schedule::NOTE.keys().collect();
        let av_keys: Vec<_> = schedule::AV_NOTES.keys().collect();
        // NOTE must not contain AV aliases
        assert!(!note_keys.contains(&"AV_Notes"));
        assert!(!note_keys.contains(&"AV"));
        // AV_NOTES must not contain plain Note
        assert!(!av_keys.contains(&"Note"));
        assert!(!av_keys.contains(&"Notes"));
        // They must be distinct canonical keys
        assert_ne!(schedule::NOTE.canonical, schedule::AV_NOTES.canonical);
    }

    #[test]
    fn test_is_timeline_canonical() {
        assert_eq!(panel_types::IS_TIMELINE.export, "Is Timeline");
        assert_eq!(panel_types::IS_TIMELINE.canonical, "Is_Timeline");
        let keys: Vec<_> = panel_types::IS_TIMELINE.keys().collect();
        assert!(keys.contains(&"Is_Time_Line"));
    }

    #[test]
    fn test_pre_reg_max_canonical() {
        assert_eq!(schedule::PRE_REG_MAX.export, "Prereg Max");
        assert_eq!(schedule::PRE_REG_MAX.canonical, "Prereg_Max");
    }

    #[test]
    fn test_schedule_all_count() {
        assert_eq!(schedule::ALL.len(), 30);
    }

    #[test]
    fn test_room_map_extra_not_in_all() {
        let all_canonicals: Vec<_> = room_map::ALL.iter().map(|f| f.canonical).collect();
        for extra in room_map::EXTRA {
            assert!(
                !all_canonicals.contains(&extra.canonical),
                "{} should not be in room_map::ALL",
                extra.canonical
            );
        }
    }
}
