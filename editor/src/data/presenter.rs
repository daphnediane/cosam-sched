use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Presenter {
    pub name: String,
    pub rank: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presenter_deserialize() {
        let json = r#"{"name": "Yaya Han", "rank": "guest"}"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        assert_eq!(p.name, "Yaya Han");
        assert_eq!(p.rank, "guest");
    }

    #[test]
    fn test_presenter_roundtrip() {
        let p = Presenter {
            name: "Sayakat Cosplay".into(),
            rank: "fan_panelist".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let p2: Presenter = serde_json::from_str(&json).unwrap();
        assert_eq!(p, p2);
    }
}
