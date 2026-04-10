/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! UUID generation preference for entity builders.

use uuid::{NonNilUuid, Uuid};

/// Controls how an entity UUID is determined when [`build()`] is called.
///
/// [`build()`]: crate docs
#[derive(Debug, Clone, Default)]
pub enum UuidPreference {
    /// Generate a new v7 (time-ordered, monotonically increasing) UUID.
    ///
    /// This is the default and the right choice when creating a brand-new
    /// entity that has no external natural key.
    #[default]
    GenerateNew,

    /// Derive a deterministic v5 UUID from a namespace UUID and a natural-key
    /// string.
    ///
    /// Use this when importing from an external source (e.g. a spreadsheet
    /// "Uniq ID" column) and you want the same natural key to always produce
    /// the same UUID across repeated imports.
    FromV5 {
        /// Namespace UUID (e.g. a per-schedule or per-entity-type constant).
        namespace: Uuid,
        /// Natural key string (e.g. `"GP001"`).
        name: String,
    },

    /// Use this exact UUID, bypassing all generation logic.
    ///
    /// Useful when restoring entities from serialized state or when the caller
    /// has already resolved the UUID outside the builder.
    Exact(NonNilUuid),
}

impl UuidPreference {
    /// Resolve the preference into a concrete [`NonNilUuid`].
    ///
    /// - `GenerateNew` produces a fresh v7 UUID.
    /// - `FromV5 { namespace, name }` produces a deterministic v5 UUID.
    /// - `Exact(uuid)` returns the UUID as-is.
    pub fn resolve(self) -> NonNilUuid {
        match self {
            Self::GenerateNew => {
                // SAFETY: Uuid::now_v7() sets version bits to 7; result is
                // never the nil UUID.
                unsafe { NonNilUuid::new_unchecked(Uuid::now_v7()) }
            }
            Self::FromV5 { namespace, name } => {
                let raw = Uuid::new_v5(&namespace, name.as_bytes());
                // SAFETY: v5 UUIDs have version bits 0x5X; never nil.
                unsafe { NonNilUuid::new_unchecked(raw) }
            }
            Self::Exact(uuid) => uuid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_new_produces_non_nil() {
        let pref = UuidPreference::GenerateNew;
        let uuid = pref.resolve();
        assert_ne!(Uuid::from(uuid), Uuid::nil());
    }

    #[test]
    fn exact_round_trips() {
        let raw = unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]))
        };
        let pref = UuidPreference::Exact(raw);
        assert_eq!(pref.resolve(), raw);
    }

    #[test]
    fn from_v5_is_deterministic() {
        let ns = Uuid::from_bytes([0xde, 0xad, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let a = UuidPreference::FromV5 {
            namespace: ns,
            name: "GP001".into(),
        }
        .resolve();
        let b = UuidPreference::FromV5 {
            namespace: ns,
            name: "GP001".into(),
        }
        .resolve();
        assert_eq!(a, b);
    }

    #[test]
    fn from_v5_differs_on_different_names() {
        let ns = Uuid::from_bytes([0xde, 0xad, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let a = UuidPreference::FromV5 {
            namespace: ns,
            name: "GP001".into(),
        }
        .resolve();
        let b = UuidPreference::FromV5 {
            namespace: ns,
            name: "GP002".into(),
        }
        .resolve();
        assert_ne!(a, b);
    }
}
