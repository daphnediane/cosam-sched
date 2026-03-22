/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use serde::{Deserialize, Serialize};

use super::panel::ExtraFields;
use super::source_info::{ChangeState, SourceInfo};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub uid: u32,
    pub short_name: String,
    pub long_name: String,
    pub hotel_room: String,
    pub sort_key: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_break: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ExtraFields>,
    #[serde(default, skip_serializing)]
    pub source: Option<SourceInfo>,
    #[serde(default, skip_serializing)]
    pub change_state: ChangeState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_deserialize() {
        let json = r#"{
            "uid": 10,
            "short_name": "Main",
            "long_name": "Main",
            "hotel_room": "Salon F/G",
            "sort_key": 1
        }"#;
        let room: Room = serde_json::from_str(json).unwrap();
        assert_eq!(room.uid, 10);
        assert_eq!(room.short_name, "Main");
        assert_eq!(room.hotel_room, "Salon F/G");
        assert_eq!(room.sort_key, 1);
    }

    #[test]
    fn test_room_roundtrip() {
        let room = Room {
            uid: 5,
            short_name: "Workshop 1".into(),
            long_name: "Workshop 1".into(),
            hotel_room: "Salon A".into(),
            sort_key: 4,
            is_break: false,
            metadata: None,
            source: None,
            change_state: ChangeState::Unchanged,
        };
        let json = serde_json::to_string(&room).unwrap();
        let room2: Room = serde_json::from_str(&json).unwrap();
        assert_eq!(room, room2);
    }
}
