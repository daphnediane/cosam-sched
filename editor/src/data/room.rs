use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub uid: u32,
    pub short_name: String,
    pub long_name: String,
    pub hotel_room: String,
    pub sort_key: u32,
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
        };
        let json = serde_json::to_string(&room).unwrap();
        let room2: Room = serde_json::from_str(&json).unwrap();
        assert_eq!(room, room2);
    }
}
