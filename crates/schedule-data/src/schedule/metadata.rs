/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule metadata and versioning information.

use serde::{Deserialize, Serialize};
use uuid::{NonNilUuid, Uuid};

/// Metadata for a schedule document.
///
/// Tracks version, timestamps, generator information, and provides a unique
/// schedule identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleMetadata {
    /// Unique identifier for this schedule document.
    pub schedule_id: NonNilUuid,
    /// Version of the schedule data format.
    pub version: String,
    /// Timestamp when the schedule was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp when the schedule was last modified.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Information about what generated this schedule.
    pub generator: GeneratorInfo,
}

/// Information about the tool that generated the schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorInfo {
    /// Name of the generator (e.g., "cosam-convert", "cosam-editor").
    pub name: String,
    /// Version of the generator.
    pub version: String,
    /// Optional additional information about the generator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<String>,
}

impl ScheduleMetadata {
    /// Create new metadata with a random schedule ID and current timestamps.
    pub fn new(generator_name: String, generator_version: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            schedule_id: unsafe { NonNilUuid::new_unchecked(Uuid::now_v7()) },
            version: "1.0.0".to_string(),
            created_at: now,
            updated_at: now,
            generator: GeneratorInfo {
                name: generator_name,
                version: generator_version,
                extra: None,
            },
        }
    }

    /// Update the updated_at timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now();
    }

    /// Check if timestamps are time-ordered (created <= updated).
    pub fn is_time_ordered(&self) -> bool {
        self.created_at <= self.updated_at
    }
}

impl Default for ScheduleMetadata {
    fn default() -> Self {
        Self::new("unknown".to_string(), "0.0.0".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_new() {
        let meta = ScheduleMetadata::new("test".to_string(), "1.0.0".to_string());
        assert_eq!(meta.generator.name, "test");
        assert_eq!(meta.generator.version, "1.0.0");
        assert!(meta.is_time_ordered());
    }

    #[test]
    fn test_metadata_touch() {
        let mut meta = ScheduleMetadata::new("test".to_string(), "1.0.0".to_string());
        let original = meta.updated_at;
        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));
        meta.touch();
        assert!(meta.updated_at > original);
    }
}
