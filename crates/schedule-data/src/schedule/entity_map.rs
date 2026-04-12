/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Generic typed entity storage wrapper.
//!
//! Provides a type-safe wrapper around `HashMap<EntityId, EntityData>` with
//! common operations for entity storage. Used by [`EntityStorage`] for all
//! entity types.

use std::collections::HashMap;

use crate::entity::EntityType;

/// Generic typed entity storage wrapper.
///
/// Wraps a `HashMap<EntityId, EntityData>` and provides common operations
/// with proper type safety. The internal HashMap is accessible for advanced
/// operations via `as_map()` and `as_map_mut()`.
///
/// # Type Parameters
///
/// - `T`: The entity type implementing [`EntityType`]
///
/// # Example
///
/// ```rust,ignore
/// use cosam_sched::entity::PanelEntityType;
/// use cosam_sched::schedule::EntityMap;
///
/// let mut panels = EntityMap::<PanelEntityType>::new();
/// // Insert, get, remove panels with type-safe IDs
/// ```
/// Generic typed entity storage wrapper.
///
/// Wraps a `HashMap<EntityId, EntityData>` and provides common operations
/// with proper type safety. The internal HashMap is accessible for advanced
/// operations via `as_map()` and `as_map_mut()`.
///
/// # Type Parameters
///
/// - `T`: The entity type implementing [`EntityType`]
#[derive(Debug)]
pub struct EntityMap<T: EntityType>
where
    T::Id: std::hash::Hash + Eq,
{
    map: HashMap<T::Id, T::Data>,
}

impl<T: EntityType> Clone for EntityMap<T>
where
    T::Id: std::hash::Hash + Eq + Clone,
    T::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
        }
    }
}

impl<T: EntityType> Default for EntityMap<T>
where
    T::Id: std::hash::Hash + Eq,
{
    fn default() -> Self {
        Self {
            map: HashMap::default(),
        }
    }
}

impl<T: EntityType> EntityMap<T>
where
    T::Id: std::hash::Hash + Eq,
{
    /// Creates an empty entity map.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Creates an empty entity map with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
        }
    }

    /// Returns a reference to the entity data for the given ID.
    pub fn get(&self, id: T::Id) -> Option<&T::Data> {
        self.map.get(&id)
    }

    /// Returns a mutable reference to the entity data for the given ID.
    pub fn get_mut(&mut self, id: T::Id) -> Option<&mut T::Data> {
        self.map.get_mut(&id)
    }

    /// Inserts an entity into the map.
    ///
    /// Returns the previous entity data if an entity with the same ID existed.
    pub fn insert(&mut self, id: T::Id, data: T::Data) -> Option<T::Data> {
        self.map.insert(id, data)
    }

    /// Removes an entity from the map.
    ///
    /// Returns the entity data if it existed.
    pub fn remove(&mut self, id: T::Id) -> Option<T::Data> {
        self.map.remove(&id)
    }

    /// Returns `true` if the map contains an entity with the given ID.
    pub fn contains(&self, id: T::Id) -> bool {
        self.map.contains_key(&id)
    }

    /// Returns `true` if the map contains an entity with the given ID.
    /// Alias for [`contains`](Self::contains).
    pub fn contains_key(&self, id: T::Id) -> bool {
        self.map.contains_key(&id)
    }

    /// Returns the number of entities in the map.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the map contains no entities.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns an iterator over the entity IDs in the map.
    pub fn keys(&self) -> impl Iterator<Item = T::Id> + '_ {
        self.map.keys().copied()
    }

    /// Returns an iterator over the entity data references in the map.
    pub fn values(&self) -> impl Iterator<Item = &T::Data> {
        self.map.values()
    }

    /// Returns an iterator over the mutable entity data references in the map.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T::Data> {
        self.map.values_mut()
    }

    /// Returns an iterator over the (ID, data) pairs in the map.
    pub fn iter(&self) -> impl Iterator<Item = (T::Id, &T::Data)> {
        self.map.iter().map(|(k, v)| (*k, v))
    }

    /// Returns an iterator over the (ID, mutable data) pairs in the map.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (T::Id, &mut T::Data)> {
        self.map.iter_mut().map(|(k, v)| (*k, v))
    }

    /// Returns a reference to the underlying HashMap.
    pub fn as_map(&self) -> &HashMap<T::Id, T::Data> {
        &self.map
    }

    /// Returns a mutable reference to the underlying HashMap.
    pub fn as_map_mut(&mut self) -> &mut HashMap<T::Id, T::Data> {
        &mut self.map
    }

    /// Clears the map, removing all entities.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Reserves capacity for at least `additional` more entities.
    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
    }

    /// Shrinks the capacity of the map as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit();
    }
}

impl<T: EntityType> IntoIterator for EntityMap<T>
where
    T::Id: std::hash::Hash + Eq,
{
    type Item = (T::Id, T::Data);
    type IntoIter = std::collections::hash_map::IntoIter<T::Id, T::Data>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a, T: EntityType> IntoIterator for &'a EntityMap<T>
where
    T::Id: std::hash::Hash + Eq,
{
    type Item = (T::Id, &'a T::Data);
    type IntoIter = std::iter::Map<
        std::collections::hash_map::Iter<'a, T::Id, T::Data>,
        fn((&T::Id, &'a T::Data)) -> (T::Id, &'a T::Data),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter().map(|(k, v)| (*k, v))
    }
}

impl<'a, T: EntityType> IntoIterator for &'a mut EntityMap<T>
where
    T::Id: std::hash::Hash + Eq,
{
    type Item = (T::Id, &'a mut T::Data);
    type IntoIter = std::iter::Map<
        std::collections::hash_map::IterMut<'a, T::Id, T::Data>,
        fn((&T::Id, &'a mut T::Data)) -> (T::Id, &'a mut T::Data),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter_mut().map(|(k, v)| (*k, v))
    }
}

#[cfg(test)]
mod tests {
    // Tests temporarily disabled - will be re-enabled once EntityStorage
    // is updated to use EntityMap and proper test fixtures are available
}
