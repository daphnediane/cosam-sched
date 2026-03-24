/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Per-table XLSX column header definitions.
//!
//! Each table has a module containing [`FieldDef`] constants for every
//! recognized column.  A `FieldDef` carries:
//! - `export`    – the canonical header string written when creating a file.
//! - `canonical` – the lookup key produced by `canonical_header(export)`.
//! - `aliases`   – additional canonical lookup keys accepted during import
//!                 (already in canonical / underscore form).
//!
//! Use `FieldDef::keys()` to iterate over all accepted canonical keys (primary
//! + aliases) when building a lookup map, and `FieldDef::export` when writing
//! the header row.

/// A single column definition: one export name plus zero or more import aliases.
#[derive(Debug, Clone, Copy)]
pub struct FieldDef {
    /// Header string written into the spreadsheet when creating a new file.
    pub export: &'static str,
    /// Primary canonical lookup key; equals `canonical_header(self.export)`.
    pub canonical: &'static str,
    /// Extra canonical keys accepted during import in addition to `canonical`.
    pub aliases: &'static [&'static str],
}

impl FieldDef {
    /// Iterate over all accepted canonical lookup keys (primary + aliases).
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

    pub const NAME: FieldDef = FieldDef {
        export: "Name",
        canonical: "Name",
        aliases: &["Panel_Name", "PanelName", "Title"],
    };

    pub const DESCRIPTION: FieldDef = FieldDef {
        export: "Description",
        canonical: "Description",
        aliases: &["Desc"],
    };

    pub const START_TIME: FieldDef = FieldDef {
        export: "Start Time",
        canonical: "Start_Time",
        aliases: &["StartTime", "Start", "Begin"],
    };

    pub const END_TIME: FieldDef = FieldDef {
        export: "End Time",
        canonical: "End_Time",
        aliases: &["EndTime", "End"],
    };

    pub const DURATION: FieldDef = FieldDef {
        export: "Duration",
        canonical: "Duration",
        aliases: &["Length"],
    };

    pub const ROOM: FieldDef = FieldDef {
        export: "Room",
        canonical: "Room",
        aliases: &["Room_Name", "Location"],
    };

    pub const KIND: FieldDef = FieldDef {
        export: "Kind",
        canonical: "Kind",
        aliases: &["Type", "Panel_Type", "PanelType", "Prefix"],
    };

    pub const COST: FieldDef = FieldDef {
        export: "Cost",
        canonical: "Cost",
        aliases: &["Price", "Fee"],
    };

    pub const CAPACITY: FieldDef = FieldDef {
        export: "Capacity",
        canonical: "Capacity",
        aliases: &["Cap", "Max"],
    };

    pub const PRE_REG_MAX: FieldDef = FieldDef {
        export: "Pre-Reg Max",
        canonical: "Pre-Reg_Max",
        aliases: &["PreRegMax", "Pre_Reg_Max"],
    };

    pub const DIFFICULTY: FieldDef = FieldDef {
        export: "Difficulty",
        canonical: "Difficulty",
        aliases: &["Level"],
    };

    pub const NOTE: FieldDef = FieldDef {
        export: "Note",
        canonical: "Note",
        aliases: &["Notes", "AV_Note", "AV_Notes", "AV"],
    };

    pub const PREREQ: FieldDef = FieldDef {
        export: "Prereq",
        canonical: "Prereq",
        aliases: &["Prerequisite", "Prerequisites", "Pre_Req"],
    };

    pub const TICKET_SALE: FieldDef = FieldDef {
        export: "Ticket Sale",
        canonical: "Ticket_Sale",
        aliases: &["TicketSale", "Tickets", "Sale"],
    };

    pub const FULL: FieldDef = FieldDef {
        export: "Full",
        canonical: "Full",
        aliases: &["IsFull", "Is_Full", "Sold_Out"],
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

    pub const OLD_UNIQ_ID: FieldDef = FieldDef {
        export: "Old Uniq Id",
        canonical: "Old_Uniq_Id",
        aliases: &["OldUniqId", "Old_ID", "OldID"],
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

    /// All fixed (non-presenter) column definitions in export order.
    pub const ALL: &[FieldDef] = &[
        UNIQ_ID,
        NAME,
        DESCRIPTION,
        START_TIME,
        END_TIME,
        DURATION,
        ROOM,
        KIND,
        COST,
        CAPACITY,
        DIFFICULTY,
        NOTE,
        PREREQ,
        TICKET_SALE,
        FULL,
        HIDE_PANELIST,
        ALT_PANELIST,
        OLD_UNIQ_ID,
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

    pub const SORT_KEY: FieldDef = FieldDef {
        export: "Sort Key",
        canonical: "Sort_Key",
        aliases: &["SortKey", "Sort", "Order"],
    };

    /// All column definitions in export order.
    pub const ALL: &[FieldDef] = &[ROOM_NAME, LONG_NAME, HOTEL_ROOM, SORT_KEY];
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

    pub const IS_CAFE: FieldDef = FieldDef {
        export: "Is Café",
        canonical: "Is_Café",
        aliases: &["Is_Cafe", "IsCafe", "IsCafé", "Cafe", "Café"],
    };

    pub const IS_ROOM_HOURS: FieldDef = FieldDef {
        export: "Is Room Hours",
        canonical: "Is_Room_Hours",
        aliases: &["IsRoomHours", "Room_Hours", "RoomHours"],
    };

    pub const HIDDEN: FieldDef = FieldDef {
        export: "Hidden",
        canonical: "Hidden",
        aliases: &["IsHidden", "Is_Hidden", "Hide"],
    };

    pub const IS_TIMELINE: FieldDef = FieldDef {
        export: "Is TimeLine",
        canonical: "Is_TimeLine",
        aliases: &["IsTimeLine", "Is_Timeline", "IsTimeline", "Timeline"],
    };

    pub const IS_PRIVATE: FieldDef = FieldDef {
        export: "Is Private",
        canonical: "Is_Private",
        aliases: &["IsPrivate", "Private"],
    };

    /// All column definitions in export order.
    pub const ALL: &[FieldDef] = &[
        PREFIX,
        PANEL_KIND,
        COLOR,
        BW_COLOR,
        IS_BREAK,
        IS_WORKSHOP,
        IS_CAFE,
        IS_ROOM_HOURS,
        HIDDEN,
        IS_TIMELINE,
        IS_PRIVATE,
    ];
}

// ─── People table ─────────────────────────────────────────────────────────────

/// Column definitions for the **People** / **Presenters** sheet.
pub mod people {
    use super::FieldDef;

    pub const NAME: FieldDef = FieldDef {
        export: "Name",
        canonical: "Name",
        aliases: &["Presenter", "Speaker"],
    };

    pub const RANK: FieldDef = FieldDef {
        export: "Rank",
        canonical: "Rank",
        aliases: &["Type", "Role", "Level"],
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
        aliases: &["AlwaysGrouped", "Always_In_Group"],
    };

    /// All column definitions in export order.
    pub const ALL: &[FieldDef] = &[NAME, RANK, IS_GROUP, MEMBERS, GROUPS, ALWAYS_GROUPED];
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
    fn test_people_all_count() {
        assert_eq!(people::ALL.len(), 6);
    }
}
