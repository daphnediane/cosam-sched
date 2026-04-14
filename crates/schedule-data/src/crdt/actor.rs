/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Actor identity for CRDT operations.
//!
//! Each device generates a UUID v4 on first launch and persists it to the
//! OS-conventional config directory.  The actor ID is embedded in every
//! automerge operation so that concurrent writes can be attributed and ordered.
//!
//! ## Config paths
//!
//! | Platform | Path |
//! |---|---|
//! | macOS | `~/Library/Application Support/com.CosplayAmerica.cosam-sched/device.toml` |
//! | Windows | `C:\Users\<user>\AppData\Roaming\CosplayAmerica\cosam-sched\device.toml` |
//! | Linux | `~/.config/cosam-sched/device.toml` |

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unique identifier for an editing peer (device / installation).
///
/// Wraps a UUID v4.  Used by the automerge backend to tag every operation so
/// that causal ordering and LWW tiebreaking work correctly across replicas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId(pub uuid::Uuid);

impl ActorId {
    /// Generate a fresh random actor ID (UUID v4).
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Wrap an existing UUID as an actor ID.
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    /// Return the underlying UUID.
    pub fn uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for ActorId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ActorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Errors from actor config loading and saving.
#[derive(Debug, Error)]
pub enum ActorConfigError {
    /// Could not determine the OS config directory (unusual; means no home dir).
    #[error("could not determine OS config directory")]
    NoDirs,
    /// Filesystem I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// TOML parse error when reading device.toml.
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    /// TOML serialization error when writing device.toml.
    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

/// Device configuration persisted to the OS config directory.
///
/// The `actor_id` is a UUID v4 generated once per device/installation.  The
/// `display_name` is written into the automerge document's `actors/` map on
/// first merge and propagated to all replicas, so any device can resolve an
/// actor UUID to a human name for change attribution.
///
/// ## File format (TOML)
///
/// ```toml
/// # Generated on first launch. Do not edit manually.
/// actor_id = "550e8400-e29b-41d4-a716-446655440000"
/// display_name = "Daphne"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// UUID v4 identifying this device/installation.
    pub actor_id: uuid::Uuid,
    /// Human-readable name for this device (set in app preferences).
    pub display_name: String,
}

impl DeviceConfig {
    /// Load device config from the OS config path, creating a new one if absent.
    ///
    /// On first call, generates a fresh UUID v4 and saves it to disk.
    /// `display_name` is used only when creating a new config; existing files
    /// are read as-is.
    pub fn load_or_create(display_name: impl Into<String>) -> Result<Self, ActorConfigError> {
        let path = Self::config_path()?;
        if path.exists() {
            Self::load_from_path(&path)
        } else {
            let config = DeviceConfig {
                actor_id: uuid::Uuid::new_v4(),
                display_name: display_name.into(),
            };
            config.save_to_path(&path)?;
            Ok(config)
        }
    }

    /// Load device config from the OS config path.
    ///
    /// Returns `None` if the config file does not yet exist (i.e., first launch
    /// before `load_or_create` has run).
    pub fn load() -> Result<Option<Self>, ActorConfigError> {
        let path = Self::config_path()?;
        if path.exists() {
            Ok(Some(Self::load_from_path(&path)?))
        } else {
            Ok(None)
        }
    }

    /// Save this config to the OS config path.
    pub fn save(&self) -> Result<(), ActorConfigError> {
        let path = Self::config_path()?;
        self.save_to_path(&path)
    }

    /// Return the [`ActorId`] for use with CRDT operations.
    pub fn actor_id(&self) -> ActorId {
        ActorId(self.actor_id)
    }

    /// Return the OS-conventional config file path.
    pub fn config_path() -> Result<PathBuf, ActorConfigError> {
        let dirs = directories::ProjectDirs::from("com", "CosplayAmerica", "cosam-sched")
            .ok_or(ActorConfigError::NoDirs)?;
        Ok(dirs.config_dir().join("device.toml"))
    }

    fn load_from_path(path: &PathBuf) -> Result<Self, ActorConfigError> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    fn save_to_path(&self, path: &PathBuf) -> Result<(), ActorConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_id_new_is_nonzero() {
        let a = ActorId::new();
        let b = ActorId::new();
        assert_ne!(a, b, "two fresh actor IDs must differ");
        assert_ne!(a.uuid(), uuid::Uuid::nil());
    }

    #[test]
    fn actor_id_display() {
        let uuid = uuid::Uuid::from_bytes([
            0x55, 0x0e, 0x84, 0x00, 0xe2, 0x9b, 0x41, 0xd4, 0xa7, 0x16, 0x44, 0x66, 0x55, 0x44,
            0x00, 0x00,
        ]);
        let actor = ActorId::from_uuid(uuid);
        assert_eq!(actor.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn device_config_serde_roundtrip() {
        let config = DeviceConfig {
            actor_id: uuid::Uuid::new_v4(),
            display_name: "TestDevice".to_string(),
        };
        let toml_str = toml::to_string(&config).expect("serialize");
        let restored: DeviceConfig = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(config.actor_id, restored.actor_id);
        assert_eq!(config.display_name, restored.display_name);
    }

    #[test]
    fn device_config_actor_id_roundtrip() {
        let original = ActorId::new();
        let config = DeviceConfig {
            actor_id: original.uuid(),
            display_name: "Test".to_string(),
        };
        assert_eq!(config.actor_id(), original);
    }
}
