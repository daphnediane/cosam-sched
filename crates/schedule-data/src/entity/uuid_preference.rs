/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! UUID generation preference for entity builders.

use uuid::{NonNilUuid, Uuid};

/// Controls how an entity UUID is determined when [`build()`] is called.
///
/// The `namespace` parameter to [`resolve()`] is supplied by the entity builder
/// and identifies the entity type; `GenerateNew` and `Exact` ignore it.
///
/// [`build()`]: crate docs
/// [`resolve()`]: UuidPreference::resolve
#[derive(Debug, Clone, Default)]
pub enum UuidPreference {
    /// Generate a new v7 (time-ordered, monotonically increasing) UUID.
    ///
    /// This is the default and the right choice when creating a brand-new
    /// entity that has no external natural key.
    #[default]
    GenerateNew,

    /// Derive a deterministic v5 UUID from the entity-type namespace and a
    /// natural-key string.
    ///
    /// Use this when importing from an external source (e.g. a spreadsheet
    /// "Uniq ID" column) and you want the same natural key to always produce
    /// the same UUID across repeated imports.
    ///
    /// The namespace is supplied by the builder; the caller does not need to
    /// know it.
    FromV5 {
        /// Natural key string (e.g. `"GP001"`).
        name: String,
    },

    /// Derive a deterministic v5 UUID for an edge entity from its two
    /// endpoint UUIDs and the entity-type namespace.
    ///
    /// The `from`→`to` ordering is significant: swapping them produces a
    /// different UUID.  The namespace is supplied by the builder.
    Edge {
        /// UUID of the "from" endpoint.
        from: NonNilUuid,
        /// UUID of the "to" endpoint.
        to: NonNilUuid,
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
    /// - `GenerateNew` produces a fresh v7 UUID (`namespace` is ignored).
    /// - `FromV5 { name }` produces `UUIDv5(namespace, name)`.
    /// - `Edge { from, to }` produces `UUIDv5(namespace, "{from}:{to}")`.
    /// - `Exact(uuid)` returns the UUID as-is (`namespace` is ignored).
    pub fn resolve(self, namespace: Uuid) -> NonNilUuid {
        match self {
            Self::GenerateNew => {
                // SAFETY: Uuid::now_v7() sets version bits to 7; result is
                // never the nil UUID.
                unsafe { NonNilUuid::new_unchecked(Uuid::now_v7()) }
            }
            Self::FromV5 { name } => {
                let raw = Uuid::new_v5(&namespace, name.as_bytes());
                // SAFETY: v5 UUIDs have version bits 0x5X; never nil.
                unsafe { NonNilUuid::new_unchecked(raw) }
            }
            Self::Edge { from, to } => {
                let name = format!("{}:{}", from, to);
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

    fn test_ns() -> Uuid {
        Uuid::from_bytes([0xde, 0xad, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    }

    fn test_nn(byte: u8) -> NonNilUuid {
        let mut b = [0u8; 16];
        b[15] = byte;
        unsafe { NonNilUuid::new_unchecked(Uuid::from_bytes(b)) }
    }

    #[test]
    fn generate_new_produces_non_nil() {
        let uuid = UuidPreference::GenerateNew.resolve(Uuid::nil());
        assert_ne!(Uuid::from(uuid), Uuid::nil());
    }

    #[test]
    fn exact_round_trips() {
        let raw = test_nn(1);
        assert_eq!(UuidPreference::Exact(raw).resolve(Uuid::nil()), raw);
    }

    #[test]
    fn from_v5_is_deterministic() {
        let ns = test_ns();
        let a = UuidPreference::FromV5 {
            name: "GP001".into(),
        }
        .resolve(ns);
        let b = UuidPreference::FromV5 {
            name: "GP001".into(),
        }
        .resolve(ns);
        assert_eq!(a, b);
    }

    #[test]
    fn from_v5_differs_on_different_names() {
        let ns = test_ns();
        let a = UuidPreference::FromV5 {
            name: "GP001".into(),
        }
        .resolve(ns);
        let b = UuidPreference::FromV5 {
            name: "GP002".into(),
        }
        .resolve(ns);
        assert_ne!(a, b);
    }

    #[test]
    fn edge_is_deterministic() {
        let ns = test_ns();
        let a = UuidPreference::Edge {
            from: test_nn(1),
            to: test_nn(2),
        }
        .resolve(ns);
        let b = UuidPreference::Edge {
            from: test_nn(1),
            to: test_nn(2),
        }
        .resolve(ns);
        assert_eq!(a, b);
    }

    #[test]
    fn edge_order_matters() {
        let ns = test_ns();
        let ab = UuidPreference::Edge {
            from: test_nn(1),
            to: test_nn(2),
        }
        .resolve(ns);
        let ba = UuidPreference::Edge {
            from: test_nn(2),
            to: test_nn(1),
        }
        .resolve(ns);
        assert_ne!(ab, ba);
    }

    #[test]
    fn edge_namespace_affects_result() {
        let ns2 = Uuid::from_bytes([0xca, 0xfe, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let a = UuidPreference::Edge {
            from: test_nn(1),
            to: test_nn(2),
        }
        .resolve(test_ns());
        let b = UuidPreference::Edge {
            from: test_nn(1),
            to: test_nn(2),
        }
        .resolve(ns2);
        assert_ne!(a, b);
    }
}
