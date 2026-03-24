/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::panel::ExtraFields;
use super::source_info::{ChangeState, SourceInfo};

#[derive(Debug, Clone, PartialEq)]
pub enum PresenterRank {
    Guest,
    Judge,
    Staff,
    InvitedGuest,
    FanPanelist,
    // Custom ranks like "Sponsor", "Industry", etc.
    Custom(String),
}

impl PresenterRank {
    pub fn as_str(&self) -> &str {
        match self {
            PresenterRank::Guest => "guest",
            PresenterRank::Judge => "judge",
            PresenterRank::Staff => "staff",
            PresenterRank::InvitedGuest => "invited_guest",
            PresenterRank::FanPanelist => "fan_panelist",
            PresenterRank::Custom(s) => s,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "guest" => PresenterRank::Guest,
            "judge" => PresenterRank::Judge,
            "staff" => PresenterRank::Staff,
            "invited_guest" => PresenterRank::InvitedGuest,
            "fan_panelist" => PresenterRank::FanPanelist,
            _ => PresenterRank::Custom(s.to_string()),
        }
    }

    pub fn prefix_char(&self) -> char {
        match self {
            PresenterRank::Guest => 'G',
            PresenterRank::Judge => 'J',
            PresenterRank::Staff => 'S',
            PresenterRank::InvitedGuest => 'I',
            PresenterRank::FanPanelist => 'P',
            PresenterRank::Custom(_) => 'I', // Default custom ranks to 'I' prefix
        }
    }
}

impl Serialize for PresenterRank {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PresenterRank {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(PresenterRank::from_str(&s))
    }
}

impl Default for PresenterRank {
    fn default() -> Self {
        PresenterRank::FanPanelist
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum PresenterMember {
    #[default]
    NotMember,
    IsMember(std::collections::BTreeSet<String>, bool), // Groups and always_grouped
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum PresenterGroup {
    #[default]
    NotGroup,
    IsGroup(std::collections::BTreeSet<String>, bool), // Members and always_shown
}

#[derive(Debug, Clone, PartialEq)]
pub struct Presenter {
    pub id: Option<u32>,
    pub name: String,
    pub rank: PresenterRank,
    pub is_member: PresenterMember,
    pub is_grouped: PresenterGroup,
    pub metadata: Option<ExtraFields>,
    pub source: Option<SourceInfo>,
    pub change_state: ChangeState,
}

impl Presenter {
    /// Helper method to check if presenter is a group (for backward compatibility)
    pub fn is_group(&self) -> bool {
        matches!(self.is_grouped, PresenterGroup::IsGroup(_, _))
    }

    /// Helper method to get members (for backward compatibility)
    pub fn members(&self) -> &std::collections::BTreeSet<String> {
        match &self.is_grouped {
            PresenterGroup::IsGroup(members, _) => members,
            PresenterGroup::NotGroup => {
                // Return a reference to an empty set
                static EMPTY_SET: std::sync::OnceLock<std::collections::BTreeSet<String>> =
                    std::sync::OnceLock::new();
                EMPTY_SET.get_or_init(std::collections::BTreeSet::new)
            }
        }
    }

    /// Helper method to get groups (for backward compatibility)
    pub fn groups(&self) -> &std::collections::BTreeSet<String> {
        match &self.is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => {
                // Return a reference to an empty set
                static EMPTY_SET: std::sync::OnceLock<std::collections::BTreeSet<String>> =
                    std::sync::OnceLock::new();
                EMPTY_SET.get_or_init(std::collections::BTreeSet::new)
            }
        }
    }

    /// Helper method to check if always_grouped (for backward compatibility)
    pub fn always_grouped(&self) -> bool {
        match &self.is_member {
            PresenterMember::IsMember(_, grouped) => *grouped,
            PresenterMember::NotMember => false,
        }
    }

    /// Helper method to check if always_shown (for backward compatibility)
    pub fn always_shown(&self) -> bool {
        match &self.is_grouped {
            PresenterGroup::IsGroup(_, shown) => *shown,
            PresenterGroup::NotGroup => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presenter_deserialize_minimal() {
        let json = r#"{"name": "Yaya Han", "rank": "guest"}"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        assert_eq!(p.name, "Yaya Han");
        assert_eq!(p.rank, PresenterRank::Guest);
        assert!(!p.is_group());
        assert_eq!(p.members(), &std::collections::BTreeSet::new());
        assert_eq!(p.groups(), &std::collections::BTreeSet::new());
        assert!(!p.always_grouped());
        assert!(!p.always_shown());
        assert_eq!(p.id, None);
    }

    #[test]
    fn test_presenter_deserialize_full() {
        let json = r#"{
            "name": "Pros and Cons Cosplay",
            "rank": "guest",
            "is_group": true,
            "members": ["Pro", "Con"],
            "groups": [],
            "always_grouped": false
        }"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        assert!(p.is_group());
        let mut expected_members = std::collections::BTreeSet::new();
        expected_members.insert("Pro".to_string());
        expected_members.insert("Con".to_string());
        assert_eq!(p.members(), &expected_members);
        assert_eq!(p.rank, PresenterRank::Guest);
    }

    #[test]
    fn test_presenter_custom_rank() {
        let json = r#"{"name": "CUT/SEW", "rank": "Sponsor"}"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        assert_eq!(p.name, "CUT/SEW");

        // Check rank string and prefix before moving
        assert_eq!(p.rank.as_str(), "Sponsor");
        assert_eq!(p.rank.prefix_char(), 'I'); // Custom ranks default to 'I' prefix

        // Now check the enum variant
        match p.rank {
            PresenterRank::Custom(s) => assert_eq!(s, "Sponsor"),
            _ => panic!("Expected Custom rank"),
        }
    }

    #[test]
    fn test_presenter_rank_from_str() {
        assert_eq!(PresenterRank::from_str("guest"), PresenterRank::Guest);
        assert_eq!(
            PresenterRank::from_str("SPONSOR"),
            PresenterRank::Custom("SPONSOR".to_string())
        );
        assert_eq!(
            PresenterRank::from_str("industry"),
            PresenterRank::Custom("industry".to_string())
        );
    }

    #[test]
    fn test_presenter_with_groups() {
        let json = r#"{
            "name": "Con",
            "rank": "guest",
            "is_group": false,
            "members": [],
            "groups": ["Pros and Cons Cosplay"],
            "always_grouped": false
        }"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        let mut expected_groups = std::collections::BTreeSet::new();
        expected_groups.insert("Pros and Cons Cosplay".to_string());
        assert_eq!(p.groups(), &expected_groups);
        assert!(!p.is_group());
    }

    #[test]
    fn test_presenter_roundtrip() {
        let p = Presenter {
            id: Some(5),
            name: "Sayakat Cosplay".into(),
            rank: PresenterRank::from_str("fan_panelist"),
            is_member: PresenterMember::NotMember,
            is_grouped: PresenterGroup::NotGroup,
            metadata: None,
            source: None,
            change_state: ChangeState::Unchanged,
        };
        let json = serde_json::to_string(&p).unwrap();
        let p2: Presenter = serde_json::from_str(&json).unwrap();
        assert_eq!(p, p2);
    }
}

// Custom serialization for Presenter to output v7 format
impl Serialize for Presenter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("Presenter", 8)?;

        // Only serialize id if it's Some (matches previous v7 behavior)
        if let Some(ref id) = self.id {
            state.serialize_field("id", id)?;
        }

        state.serialize_field("name", &self.name)?;
        state.serialize_field("rank", &self.rank)?;
        state.serialize_field("is_group", &self.is_group())?;
        state.serialize_field("members", &self.members().iter().collect::<Vec<_>>())?;
        state.serialize_field("groups", &self.groups().iter().collect::<Vec<_>>())?;
        state.serialize_field("always_grouped", &self.always_grouped())?;
        state.serialize_field("always_shown", &self.always_shown())?;
        if let Some(ref metadata) = self.metadata {
            state.serialize_field("metadata", metadata)?;
        }
        state.end()
    }
}

// Custom deserialization for Presenter to handle both v7 and legacy formats
impl<'de> Deserialize<'de> for Presenter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct PresenterHelper {
            #[serde(default)]
            id: Option<u32>,
            name: String,
            #[serde(default)]
            rank: PresenterRank,
            // V7 format fields
            #[serde(default)]
            is_group: bool,
            #[serde(default)]
            members: Vec<String>,
            #[serde(default)]
            groups: Vec<String>,
            #[serde(default)]
            always_grouped: bool,
            #[serde(default)]
            always_shown: bool,
            // Legacy enum fields (for backward compatibility)
            #[serde(default)]
            is_member: Option<PresenterMember>,
            #[serde(default)]
            is_grouped: Option<PresenterGroup>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            metadata: Option<ExtraFields>,
        }

        let helper = PresenterHelper::deserialize(deserializer)?;

        // Determine which format we're dealing with
        let (is_member, is_grouped) = if helper.is_member.is_some() || helper.is_grouped.is_some() {
            // Legacy format with enums
            (
                helper.is_member.unwrap_or_default(),
                helper.is_grouped.unwrap_or_default(),
            )
        } else {
            // V7 format with separate boolean/array fields
            let is_member = if helper.groups.is_empty() {
                PresenterMember::NotMember
            } else {
                PresenterMember::IsMember(
                    helper.groups.into_iter().collect(),
                    helper.always_grouped,
                )
            };

            let is_grouped = if helper.is_group || !helper.members.is_empty() {
                PresenterGroup::IsGroup(helper.members.into_iter().collect(), helper.always_shown)
            } else {
                PresenterGroup::NotGroup
            };

            (is_member, is_grouped)
        };

        Ok(Presenter {
            id: helper.id,
            name: helper.name,
            rank: helper.rank,
            is_member,
            is_grouped,
            metadata: helper.metadata,
            source: None,
            change_state: Default::default(),
        })
    }
}
