/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Actor identity and device configuration for CRDT operations.
//!
//! ## File layout
//!
//! All cosam apps share one config directory.  Within it, shared user identity
//! is in `identity.toml` and each app binary gets its own actor UUID file:
//!
//! ```text
//! ~/Library/Application Support/com.CosplayAmerica.cosam_sched/   (macOS)
//! ├── identity.toml            ← display_name; shared across all apps
//! ├── actor-cosam-editor.toml  ← actor UUID for the GUI editor
//! └── actor-cosam-modify.toml  ← actor UUID for the CLI tool
//! ```
//!
//! Per-app actor UUIDs let the CRDT layer distinguish which app made which
//! change.  The shared `display_name` means all apps attribute changes to the
//! same human name ("Daphne").
//!
//! ## OS config directory
//!
//! | Platform | Config directory |
//! |---|---|
//! | macOS   | `~/Library/Application Support/com.CosplayAmerica.cosam_sched/` |
//! | Windows | `C:\Users\<user>\AppData\Roaming\CosplayAmerica\cosam_sched\` |
//! | Linux   | `~/.config/cosam_sched/` |
//!
//! ## Usage (app startup)
//!
//! ```rust,ignore
//! let config = DeviceConfig::load_or_create("cosam-editor", "Daphne")?;
//! let doc = AutomergeDocument::new(&config.actor_id())?;
//! ```

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// ActorId
// ---------------------------------------------------------------------------

/// Unique identifier for an editing peer (one per app per device).
///
/// Wraps a UUID v4.  The automerge backend embeds this in every operation so
/// that causal ordering and LWW tiebreaking work correctly when merging with
/// other replicas.
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

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from config loading and saving.
#[derive(Debug, Error)]
pub enum ActorConfigError {
    /// Could not determine the OS config directory (no home dir).
    #[error("could not determine OS config directory")]
    NoDirs,
    /// Filesystem I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// TOML deserialize error.
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    /// TOML serialize error.
    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

// ---------------------------------------------------------------------------
// On-disk file formats (private)
// ---------------------------------------------------------------------------

/// identity.toml — shared across all apps.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IdentityFile {
    display_name: String,
}

/// actor-<app>.toml — one per app binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActorFile {
    actor_id: uuid::Uuid,
}

// ---------------------------------------------------------------------------
// DeviceConfig — the combined public type
// ---------------------------------------------------------------------------

/// Combined device configuration for CRDT use.
///
/// Assembled at startup from two files in the shared config directory:
///
/// - `identity.toml` — the user's display name (shared across all apps)
/// - `actor-<app>.toml` — the actor UUID for this specific app binary
///
/// Use [`load_or_create`][Self::load_or_create] at startup, passing the app
/// binary name (e.g. `"cosam-editor"` or `"cosam-modify"`).  Both files are
/// created on first launch if absent.
///
/// ## TOML file formats
///
/// `identity.toml`:
/// ```toml
/// # Shared user identity — edit display_name in app preferences.
/// display_name = "Daphne"
/// ```
///
/// `actor-cosam-editor.toml`:
/// ```toml
/// # Generated on first launch. Do not edit manually.
/// actor_id = "550e8400-e29b-41d4-a716-446655440000"
/// ```
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    /// Actor UUID for this app on this device.  Distinct per app binary so the
    /// CRDT layer can attribute changes to "cosam-editor" vs "cosam-modify".
    pub actor_id: uuid::Uuid,
    /// Human-readable name shown in change attribution (from `identity.toml`).
    pub display_name: String,
}

impl DeviceConfig {
    /// Load or create device config for the given app binary name.
    ///
    /// - `app` — the binary name used to select the actor file, e.g.
    ///   `"cosam-editor"` or `"cosam-modify"`.
    /// - `display_name` — used only when creating `identity.toml` for the
    ///   first time; existing files are read as-is.
    ///
    /// Creates the config directory and any absent files on first call.
    pub fn load_or_create(
        app: &str,
        display_name: impl Into<String>,
    ) -> Result<Self, ActorConfigError> {
        let dir = Self::config_dir()?;
        std::fs::create_dir_all(&dir)?;

        let identity = load_or_create_identity(&dir.join("identity.toml"), display_name)?;
        let actor = load_or_create_actor(&dir.join(actor_filename(app)))?;

        Ok(Self {
            actor_id: actor.actor_id,
            display_name: identity.display_name,
        })
    }

    /// Load device config for the given app binary name.
    ///
    /// Returns `None` if either config file does not yet exist.
    pub fn load(app: &str) -> Result<Option<Self>, ActorConfigError> {
        let dir = Self::config_dir()?;
        let identity_path = dir.join("identity.toml");
        let actor_path = dir.join(actor_filename(app));

        if !identity_path.exists() || !actor_path.exists() {
            return Ok(None);
        }

        let identity: IdentityFile = read_toml(&identity_path)?;
        let actor: ActorFile = read_toml(&actor_path)?;

        Ok(Some(Self {
            actor_id: actor.actor_id,
            display_name: identity.display_name,
        }))
    }

    /// Save the display name back to `identity.toml`.
    ///
    /// Call this when the user changes their display name in preferences.
    /// The actor UUID file is never updated after creation.
    pub fn save_identity(&self) -> Result<(), ActorConfigError> {
        let path = Self::config_dir()?.join("identity.toml");
        write_toml(&path, &IdentityFile {
            display_name: self.display_name.clone(),
        })
    }

    /// Return the [`ActorId`] for use with CRDT operations.
    pub fn actor_id(&self) -> ActorId {
        ActorId(self.actor_id)
    }

    /// Return the shared config directory path.
    ///
    /// All actor and identity files live here.
    pub fn config_dir() -> Result<PathBuf, ActorConfigError> {
        let dirs =
            directories::ProjectDirs::from("com", "CosplayAmerica", "cosam_sched")
                .ok_or(ActorConfigError::NoDirs)?;
        Ok(dirs.config_dir().to_path_buf())
    }

    /// Return the path to `identity.toml`.
    pub fn identity_path() -> Result<PathBuf, ActorConfigError> {
        Ok(Self::config_dir()?.join("identity.toml"))
    }

    /// Return the path to `actor-<app>.toml` for the given app binary name.
    pub fn actor_path(app: &str) -> Result<PathBuf, ActorConfigError> {
        Ok(Self::config_dir()?.join(actor_filename(app)))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build the actor filename from the app name: `actor-<app>.toml`.
fn actor_filename(app: &str) -> String {
    format!("actor-{app}.toml")
}

fn read_toml<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T, ActorConfigError> {
    let content = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

fn write_toml<T: Serialize>(path: &PathBuf, value: &T) -> Result<(), ActorConfigError> {
    let content = toml::to_string(value)?;
    std::fs::write(path, content)?;
    Ok(())
}

fn load_or_create_identity(
    path: &PathBuf,
    display_name: impl Into<String>,
) -> Result<IdentityFile, ActorConfigError> {
    if path.exists() {
        read_toml(path)
    } else {
        let identity = IdentityFile {
            display_name: display_name.into(),
        };
        write_toml(path, &identity)?;
        Ok(identity)
    }
}

fn load_or_create_actor(path: &PathBuf) -> Result<ActorFile, ActorConfigError> {
    if path.exists() {
        read_toml(path)
    } else {
        let actor = ActorFile {
            actor_id: uuid::Uuid::new_v4(),
        };
        write_toml(path, &actor)?;
        Ok(actor)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
    fn actor_filename_format() {
        assert_eq!(actor_filename("cosam-editor"), "actor-cosam-editor.toml");
        assert_eq!(actor_filename("cosam-modify"), "actor-cosam-modify.toml");
    }

    #[test]
    fn identity_file_serde_roundtrip() {
        let identity = IdentityFile {
            display_name: "Daphne".to_string(),
        };
        let toml_str = toml::to_string(&identity).expect("serialize");
        let restored: IdentityFile = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(identity.display_name, restored.display_name);
    }

    #[test]
    fn actor_file_serde_roundtrip() {
        let actor = ActorFile {
            actor_id: uuid::Uuid::new_v4(),
        };
        let toml_str = toml::to_string(&actor).expect("serialize");
        let restored: ActorFile = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(actor.actor_id, restored.actor_id);
    }

    #[test]
    fn device_config_actor_id_roundtrip() {
        let uuid = uuid::Uuid::new_v4();
        let config = DeviceConfig {
            actor_id: uuid,
            display_name: "Test".to_string(),
        };
        assert_eq!(config.actor_id(), ActorId::from_uuid(uuid));
    }

    #[test]
    fn two_apps_get_distinct_filenames() {
        let editor = actor_filename("cosam-editor");
        let modify = actor_filename("cosam-modify");
        assert_ne!(editor, modify);
    }
}
