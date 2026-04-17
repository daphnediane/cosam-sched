/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Compile-time static string registration and runtime interning.
//!
//! Modules that own `&'static str` constants (entity type names, field names)
//! submit them via [`inventory::submit!`] at link time. [`intern_static_str`]
//! then resolves deserialized strings to the pre-existing `&'static str`
//! pointer with zero allocation for known strings. Unknown strings (e.g. bad
//! data or future types) are leaked exactly once and cached, bounding total
//! leaked memory to the number of distinct unregistered strings ever seen.
//!
//! ## Usage
//!
//! Register a known static string (typically right after its definition):
//!
//! ```ignore
//! inventory::submit! { KnownStaticStr(MyEntityType::TYPE_NAME) }
//! inventory::submit! { KnownStaticStr("my_field_name") }
//! ```
//!
//! Resolve a deserialized `&str` to `&'static str`:
//!
//! ```ignore
//! let s: &'static str = intern_static_str(&deserialized_string);
//! ```

/// A static string submitted to the global intern registry at link time.
pub struct KnownStaticStr(pub &'static str);
inventory::collect!(KnownStaticStr);

/// Resolve a `&str` to a `&'static str`, seeding from registered strings first.
///
/// Lookup order:
/// 1. The compile-time registry (all [`KnownStaticStr`] submissions) — zero allocation.
/// 2. A runtime cache of previously interned strings — zero allocation on hit.
/// 3. `Box::leak` for genuinely unknown strings — one allocation per unique string, ever.
pub fn intern_static_str(s: &str) -> &'static str {
    use std::collections::HashSet;
    use std::sync::{LazyLock, Mutex};

    static INTERNED: LazyLock<Mutex<HashSet<&'static str>>> = LazyLock::new(|| {
        let mut set = HashSet::new();
        for entry in inventory::iter::<KnownStaticStr>() {
            set.insert(entry.0);
        }
        Mutex::new(set)
    });

    let mut set = INTERNED.lock().unwrap();
    if let Some(&existing) = set.get(s) {
        return existing;
    }
    let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
    set.insert(leaked);
    leaked
}

#[cfg(test)]
mod tests {
    use super::*;

    inventory::submit! { KnownStaticStr("test_known") }

    #[test]
    fn test_known_string_returns_static_pointer() {
        let result = intern_static_str("test_known");
        assert_eq!(result, "test_known");
    }

    #[test]
    fn test_unknown_string_is_interned_once() {
        let a = intern_static_str("unknown_xyz_12345");
        let b = intern_static_str("unknown_xyz_12345");
        assert!(std::ptr::eq(a, b), "second call must return same pointer");
    }

    #[test]
    fn test_different_strings_are_distinct() {
        let a = intern_static_str("alpha_unique_1");
        let b = intern_static_str("beta_unique_2");
        assert_ne!(a, b);
    }
}
