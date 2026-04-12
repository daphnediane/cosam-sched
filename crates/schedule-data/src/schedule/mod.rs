/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule container with entity storage, edge indexing, and UUID registry.

pub mod edge_index;
mod metadata;
mod storage;

pub use edge_index::EdgeIndex;
pub use metadata::{GeneratorInfo, ScheduleMetadata};
pub use storage::{BuildError, EntityStorage, EntityStore, InsertError, TypedStorage};

use crate::entity::{
    EntityKind, EntityType, EntityUUID, EventRoomId, HotelRoomId, PanelId, PanelTypeId,
    PresenterEntityType, PresenterId, TypedId,
};
use uuid::NonNilUuid;

/// Error returned by [`Schedule::lookup_tagged_presenter`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    /// Input string was empty or whitespace.
    Empty,
    /// A `presenter-<uuid>` or bare UUID reference was supplied but no
    /// presenter with that UUID exists in the schedule.
    UuidNotFound(uuid::Uuid),
    /// The UUID string was syntactically invalid.
    InvalidUuid(String),
    /// The input was a bare name (no tag prefix) and no exact
    /// case-insensitive match was found. Auto-create is not performed
    /// at this layer; use a tagged string to create new presenters.
    NameNotFound(String),
    /// The tag prefix character was not a recognised rank flag.
    UnknownTag(char),
    /// The rest after the tag was "Other", a column-header sentinel.
    OtherSentinel,
}

impl std::fmt::Display for LookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LookupError::Empty => write!(f, "presenter string is empty"),
            LookupError::UuidNotFound(u) => write!(f, "no presenter with UUID {u}"),
            LookupError::InvalidUuid(s) => write!(f, "invalid UUID string: {s}"),
            LookupError::NameNotFound(n) => write!(f, "no presenter named {n:?}"),
            LookupError::UnknownTag(c) => write!(f, "unknown rank tag {c:?}"),
            LookupError::OtherSentinel => write!(f, "input is the 'Other' column sentinel"),
        }
    }
}

impl std::error::Error for LookupError {}

/// Central schedule container.
///
/// Holds all entities, relationships, metadata, and provides a unified API
/// for schedule operations.
#[derive(Debug)]
pub struct Schedule {
    /// Entity storage for all entity types.
    pub entities: EntityStorage,

    /// Schedule metadata.
    metadata: ScheduleMetadata,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    /// Create a new empty schedule.
    pub fn new() -> Self {
        Self {
            entities: EntityStorage::new(),
            metadata: ScheduleMetadata::default(),
        }
    }

    /// Get the schedule metadata.
    pub fn metadata(&self) -> &ScheduleMetadata {
        &self.metadata
    }

    /// Get mutable schedule metadata.
    pub fn metadata_mut(&mut self) -> &mut ScheduleMetadata {
        &mut self.metadata
    }

    // -----------------------------------------------------------------------
    // UUID registry and identification
    // -----------------------------------------------------------------------

    /// Identify which entity kind a UUID belongs to.
    pub fn identify(&self, uuid: NonNilUuid) -> Option<EntityUUID> {
        let kind = self.entities.uuid_registry.get(&uuid)?;
        match kind {
            EntityKind::Panel => Some(EntityUUID::Panel(PanelId::from_uuid(uuid))),
            EntityKind::Presenter => Some(EntityUUID::Presenter(PresenterId::from_uuid(uuid))),
            EntityKind::EventRoom => Some(EntityUUID::EventRoom(EventRoomId::from_uuid(uuid))),
            EntityKind::HotelRoom => Some(EntityUUID::HotelRoom(HotelRoomId::from_uuid(uuid))),
            EntityKind::PanelType => Some(EntityUUID::PanelType(PanelTypeId::from_uuid(uuid))),
        }
    }

    // -----------------------------------------------------------------------
    // Generic entity CRUD (works for all node and edge entity types)
    // -----------------------------------------------------------------------

    /// Add any entity to the schedule, registering its UUID.
    pub fn add_entity<T>(&mut self, data: T::Data) -> Result<T::Id, InsertError>
    where
        T: EntityType + TypedStorage,
    {
        self.entities.add_entity::<T>(data)
    }

    /// Get entity data by typed ID.
    pub fn get_entity<T>(&self, id: T::Id) -> Option<&T::Data>
    where
        T: EntityType + TypedStorage,
    {
        EntityStore::<T>::get_entity(&self.entities, id.non_nil_uuid())
    }

    /// Get a mutable reference to entity data by typed ID.
    pub fn get_entity_mut<T>(&mut self, id: T::Id) -> Option<&mut T::Data>
    where
        T: EntityType + TypedStorage,
    {
        EntityStore::<T>::get_entity_mut(&mut self.entities, id.non_nil_uuid())
    }

    /// Remove an entity by typed ID, returning the data if it existed.
    pub fn remove_entity<T>(&mut self, id: T::Id) -> Option<T::Data>
    where
        T: EntityType + TypedStorage,
    {
        EntityStore::<T>::remove_entity(&mut self.entities, id.non_nil_uuid())
    }

    /// Check if an entity with the given typed ID exists.
    pub fn contains_entity<T>(&self, id: T::Id) -> bool
    where
        T: EntityType + TypedStorage,
    {
        EntityStore::<T>::contains_entity(&self.entities, id.non_nil_uuid())
    }

    /// Get entity data by raw UUID (requires knowing the entity type).
    pub fn get_entity_by_uuid<T>(&self, uuid: NonNilUuid) -> Option<&T::Data>
    where
        T: EntityType + TypedStorage,
    {
        EntityStore::<T>::get_entity(&self.entities, uuid)
    }

    // -----------------------------------------------------------------------
    // Relationship convenience methods
    // -----------------------------------------------------------------------

    /// Presenters assigned to a panel (from the `presenter_ids` backing field).
    pub fn get_panel_presenters(&self, panel_id: PanelId) -> Vec<PresenterId> {
        use crate::entity::PanelEntityType;
        PanelEntityType::presenters_of(&self.entities, panel_id)
    }

    /// Panels a presenter is assigned to (from the `panels_by_presenter` reverse index).
    pub fn get_presenter_panels(&self, presenter_id: PresenterId) -> Vec<PanelId> {
        use crate::entity::PanelEntityType;
        PanelEntityType::panels_of_presenter(&self.entities, presenter_id)
    }

    /// Event room assigned to a panel (from the `event_room_id` backing field).
    pub fn get_panel_event_room(&self, panel_id: PanelId) -> Option<EventRoomId> {
        use crate::entity::PanelEntityType;
        PanelEntityType::event_room_of(&self.entities, panel_id)
    }

    /// Panels assigned to an event room (from the `panels_by_event_room` reverse index).
    pub fn get_event_room_panels(&self, event_room_id: EventRoomId) -> Vec<PanelId> {
        use crate::entity::EventRoomEntityType;
        EventRoomEntityType::panels_of(&self.entities, event_room_id)
    }

    /// Panel type assigned to a panel (from the `panel_type_id` backing field).
    pub fn get_panel_type(&self, panel_id: PanelId) -> Option<PanelTypeId> {
        use crate::entity::PanelEntityType;
        PanelEntityType::panel_type_of(&self.entities, panel_id)
    }

    /// Panels of a given panel type (from the `panels_by_panel_type` reverse index).
    pub fn get_panels_by_type(&self, panel_type_id: PanelTypeId) -> Vec<PanelId> {
        use crate::entity::PanelTypeEntityType;
        PanelTypeEntityType::panels_of(&self.entities, panel_type_id)
    }

    /// Hotel rooms mapped to an event room (from the `hotel_room_ids` backing field).
    pub fn get_event_room_hotel_rooms(&self, event_room_id: EventRoomId) -> Vec<HotelRoomId> {
        use crate::entity::EventRoomEntityType;
        EventRoomEntityType::hotel_rooms_of(&self.entities, event_room_id)
    }

    /// Groups a presenter belongs to (from the `group_ids` backing field).
    pub fn get_presenter_groups(&self, presenter_id: PresenterId) -> Vec<PresenterId> {
        use crate::entity::PresenterEntityType;
        PresenterEntityType::groups_of(&self.entities, presenter_id)
    }

    /// Members of a presenter group (from the `presenters_by_group` reverse index).
    pub fn get_presenter_members(&self, group_id: PresenterId) -> Vec<PresenterId> {
        use crate::entity::PresenterEntityType;
        PresenterEntityType::members_of(&self.entities, group_id)
    }

    /// Whether a presenter is a group (has the explicit flag set or has members).
    pub fn is_presenter_group(&self, presenter_id: PresenterId) -> bool {
        PresenterEntityType::is_group(&self.entities, presenter_id.non_nil_uuid())
    }

    // -----------------------------------------------------------------------
    // Presenter-group membership mutation helpers
    // -----------------------------------------------------------------------

    /// Mark a presenter as a group by setting `is_explicit_group = true`.
    pub fn mark_presenter_group(&mut self, presenter_id: PresenterId) -> Result<(), InsertError> {
        PresenterEntityType::set_explicit_group(&mut self.entities, presenter_id, true);
        Ok(())
    }

    /// Set the group status of a presenter to `value`.
    ///
    /// - `true` → sets `is_explicit_group = true`.
    /// - `false` → clears `is_explicit_group` AND removes all members so the
    ///   computed read stays coherent.
    pub fn set_is_group(&mut self, presenter_id: PresenterId, value: bool) {
        use crate::entity::PresenterEntityType;
        PresenterEntityType::set_explicit_group(&mut self.entities, presenter_id, value);
    }

    /// Remove the explicit group marker from a presenter.
    ///
    /// Sets `is_explicit_group = false`.
    /// Does **not** remove members — use [`set_is_group`](Self::set_is_group)`(id, false)` for that.
    ///
    /// Returns `true` if the entity was previously marked as an explicit group.
    pub fn unmark_presenter_group(&mut self, presenter_id: PresenterId) -> bool {
        use crate::entity::PresenterEntityType;
        PresenterEntityType::unmark_explicit_group(&mut self.entities, presenter_id)
    }

    /// Add `member` to `group` with default flags (`always_shown_in_group = false`,
    /// `always_grouped = false`).
    ///
    /// No-op if already a member (flags are not changed).
    /// Updates `member.group_ids` backing field and `presenters_by_group` reverse index.
    /// Use [`add_grouped_member`](Self::add_grouped_member) or
    /// [`add_shown_member`](Self::add_shown_member) to set flags.
    pub fn add_member(
        &mut self,
        member: PresenterId,
        group: PresenterId,
    ) -> Result<(), InsertError> {
        use crate::entity::PresenterEntityType;
        PresenterEntityType::add_member(&mut self.entities, member, group)
    }

    /// Add `member` to `group` and set `always_grouped = true`.
    ///
    /// If already a member, updates the flag without duplicating the entry.
    /// Updates `member.always_grouped` and `member.group_ids` backing fields.
    pub fn add_grouped_member(
        &mut self,
        member: PresenterId,
        group: PresenterId,
    ) -> Result<(), InsertError> {
        use crate::entity::PresenterEntityType;
        PresenterEntityType::add_grouped_member(&mut self.entities, member, group)
    }

    /// Add `member` to `group` and set `always_shown_in_group = true`.
    ///
    /// If already a member, updates the flag without duplicating the entry.
    /// Updates `member.always_shown_in_group` and `member.group_ids` backing fields.
    pub fn add_shown_member(
        &mut self,
        member: PresenterId,
        group: PresenterId,
    ) -> Result<(), InsertError> {
        use crate::entity::PresenterEntityType;
        PresenterEntityType::add_shown_member(&mut self.entities, member, group)
    }

    /// Remove `member` from `group`.
    ///
    /// Updates `presenters_by_group` reverse index and `member.group_ids` backing field.
    /// Returns `true` if the membership existed and was removed.
    pub fn remove_member(&mut self, member: PresenterId, group: PresenterId) -> bool {
        use crate::entity::PresenterEntityType;
        PresenterEntityType::remove_member(&mut self.entities, member, group)
    }

    // -----------------------------------------------------------------------
    // Presenter tag-string lookup / find-or-create
    // -----------------------------------------------------------------------

    /// Look up a presenter by a tagged credit string, or find-or-create one.
    ///
    /// Delegates to [`PresenterEntityType::lookup_tagged`] which owns the
    /// implementation. See that method for the full format documentation.
    #[must_use = "returns the presenter/group ID; check for errors"]
    pub fn lookup_tagged_presenter(&mut self, input: &str) -> Result<PresenterId, LookupError> {
        PresenterEntityType::lookup_tagged(&mut self.entities, input)
    }

    /// Add presenters to a panel by parsing tag strings.
    ///
    /// Each tag string is resolved via [`lookup_tagged_presenter`](Self::lookup_tagged_presenter),
    /// which handles UUID references, tagged credit strings with rank/group syntax,
    /// and bare name lookups. Successfully resolved presenters are connected to the
    /// panel via `PanelToPresenter` edges.
    ///
    /// Returns the number of presenters successfully added. Errors for individual
    /// tags are silently ignored (the tag is skipped); callers that need error
    /// details should use `lookup_tagged_presenter` directly.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let panel_id = schedule.add_entity::<PanelEntityType>(panel_data)?;
    /// let count = schedule.add_presenters(panel_id, &["G:Alice", "P:Bob", "G:Carol=TeamA"]);
    /// ```
    pub fn add_presenters(&mut self, panel_id: PanelId, tags: &[&str]) -> usize {
        use crate::entity::PanelEntityType;
        PanelEntityType::add_presenters_tagged(&mut self.entities, panel_id.non_nil_uuid(), tags)
    }

    // -----------------------------------------------------------------------
    // Generic name lookup helper used by computed-field closures
    // -----------------------------------------------------------------------

    /// Return display names for a slice of UUIDs belonging to entity type `T`.
    /// TODO: Implement field-based name lookup when field system is fully integrated.
    pub fn get_entity_names<T: EntityType>(&self, _uuids: &[NonNilUuid]) -> Vec<String> {
        vec![]
    }
}

/// `Schedule` delegates `EntityStore<T>` to its inner `EntityStorage`.
impl<T: TypedStorage> EntityStore<T> for Schedule {
    fn get_entity(&self, uuid: NonNilUuid) -> Option<&T::Data> {
        EntityStore::<T>::get_entity(&self.entities, uuid)
    }

    fn get_entity_mut(&mut self, uuid: NonNilUuid) -> Option<&mut T::Data> {
        EntityStore::<T>::get_entity_mut(&mut self.entities, uuid)
    }

    fn insert_entity(&mut self, uuid: NonNilUuid, data: T::Data) -> Result<(), InsertError> {
        EntityStore::<T>::insert_entity(&mut self.entities, uuid, data)
    }

    fn remove_entity(&mut self, uuid: NonNilUuid) -> Option<T::Data> {
        EntityStore::<T>::remove_entity(&mut self.entities, uuid)
    }

    fn contains_entity(&self, uuid: NonNilUuid) -> bool {
        EntityStore::<T>::contains_entity(&self.entities, uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::UuidPreference;
    use uuid::Uuid;

    fn nn(b: u8) -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b,
            ]))
        }
    }

    /// Insert a Panel with the given UUID, uid, and name. Returns PanelId.
    fn add_panel(schedule: &mut Schedule, uuid: NonNilUuid, uid: &str, name: &str) -> PanelId {
        crate::entity::panel::PanelBuilder::new()
            .with_uuid_preference(UuidPreference::Exact(uuid))
            .with_uid(uid.to_string())
            .with_name(name.to_string())
            .build(schedule)
            .unwrap()
    }

    /// Insert a Presenter with the given UUID and name. Returns PresenterId.
    fn add_presenter(schedule: &mut Schedule, uuid: NonNilUuid, name: &str) -> PresenterId {
        crate::entity::presenter::PresenterBuilder::new()
            .with_uuid_preference(UuidPreference::Exact(uuid))
            .with_name(name.to_string())
            .build(schedule)
            .unwrap()
    }

    /// Insert an EventRoom with the given UUID and name. Returns EventRoomId.
    fn add_event_room(schedule: &mut Schedule, uuid: NonNilUuid, name: &str) -> EventRoomId {
        crate::entity::event_room::EventRoomBuilder::new()
            .with_uuid_preference(UuidPreference::Exact(uuid))
            .with_room_name(name.to_string())
            .build(schedule)
            .unwrap()
    }

    /// Insert a PanelType with the given UUID, prefix, and kind. Returns PanelTypeId.
    fn add_panel_type(
        schedule: &mut Schedule,
        uuid: NonNilUuid,
        prefix: &str,
        kind: &str,
    ) -> PanelTypeId {
        crate::entity::panel_type::PanelTypeBuilder::new()
            .with_uuid_preference(UuidPreference::Exact(uuid))
            .with_prefix(prefix.to_string())
            .with_panel_kind(kind.to_string())
            .build(schedule)
            .unwrap()
    }

    /// Insert a HotelRoom with the given UUID and name. Returns HotelRoomId.
    fn add_hotel_room(schedule: &mut Schedule, uuid: NonNilUuid, name: &str) -> HotelRoomId {
        crate::entity::hotel_room::HotelRoomBuilder::new()
            .with_uuid_preference(UuidPreference::Exact(uuid))
            .with_hotel_room_name(name.to_string())
            .build(schedule)
            .unwrap()
    }

    #[test]
    fn test_panel_presenter_relationship() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");
        let panel_id = PanelId::from(panel_uuid);

        let count = schedule.add_presenters(panel_id, &["P:Alice"]);
        assert_eq!(count, 1);

        let presenters = schedule.get_panel_presenters(panel_id);
        assert_eq!(presenters.len(), 1);

        let presenter_id = presenters[0];
        let panels = schedule.get_presenter_panels(presenter_id);
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].non_nil_uuid(), panel_uuid);
    }

    #[test]
    fn test_presenter_removed_from_panel() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");
        let panel_id = PanelId::from(panel_uuid);

        schedule.add_presenters(panel_id, &["P:Bob"]);
        let presenter_id = schedule.get_panel_presenters(panel_id)[0];
        let presenter_uuid = presenter_id.non_nil_uuid();

        if let Some(panel_data) = schedule.entities.panels.get_mut(&panel_uuid) {
            panel_data
                .presenter_ids
                .retain(|id| id.non_nil_uuid() != presenter_uuid);
        }
        if let Some(panels) = schedule
            .entities
            .panels_by_presenter
            .get_mut(&presenter_uuid)
        {
            panels.retain(|&u| u != panel_uuid);
        }

        assert!(schedule.get_panel_presenters(panel_id).is_empty());
        assert!(schedule.get_presenter_panels(presenter_id).is_empty());
    }

    #[test]
    fn test_multiple_presenters_per_panel() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");
        let panel_id = PanelId::from(panel_uuid);

        schedule.add_presenters(panel_id, &["P:Alice", "P:Bob"]);

        let presenters = schedule.get_panel_presenters(panel_id);
        assert_eq!(presenters.len(), 2);
    }

    #[test]
    fn test_panel_to_event_room_single() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        let room_uuid = nn(2);

        let panel_id = add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");
        let room_id = add_event_room(&mut schedule, room_uuid, "Room A");

        if let Some(panel_data) = schedule.entities.panels.get_mut(&panel_uuid) {
            panel_data.event_room_id = Some(EventRoomId::from_uuid(room_uuid));
        }
        schedule
            .entities
            .panels_by_event_room
            .entry(room_uuid)
            .or_default()
            .push(panel_uuid);

        let room = schedule.get_panel_event_room(panel_id);
        assert!(room.is_some());
        assert_eq!(room.unwrap().non_nil_uuid(), room_uuid);

        let panels = schedule.get_event_room_panels(room_id);
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].non_nil_uuid(), panel_uuid);
    }

    #[test]
    fn test_panel_to_panel_type() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        let type_uuid = nn(2);

        let panel_id = add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");
        let type_id = add_panel_type(&mut schedule, type_uuid, "WS", "Workshop");

        if let Some(panel_data) = schedule.entities.panels.get_mut(&panel_uuid) {
            panel_data.panel_type_id = Some(PanelTypeId::from_uuid(type_uuid));
        }
        schedule
            .entities
            .panels_by_panel_type
            .entry(type_uuid)
            .or_default()
            .push(panel_uuid);

        assert_eq!(
            schedule.get_panel_type(panel_id).unwrap().non_nil_uuid(),
            type_uuid
        );

        let panels = schedule.get_panels_by_type(type_id);
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].non_nil_uuid(), panel_uuid);
    }

    #[test]
    fn test_event_room_to_hotel_room() {
        let mut schedule = Schedule::new();
        let er_uuid = nn(1);
        let hr_uuid = nn(2);

        let er_id = add_event_room(&mut schedule, er_uuid, "ER1");
        add_hotel_room(&mut schedule, hr_uuid, "HR1");

        if let Some(er_data) = schedule.entities.event_rooms.get_mut(&er_uuid) {
            er_data.hotel_room_ids.push(HotelRoomId::from_uuid(hr_uuid));
        }
        schedule
            .entities
            .event_rooms_by_hotel_room
            .entry(hr_uuid)
            .or_default()
            .push(er_uuid);

        let rooms = schedule.get_event_room_hotel_rooms(er_id);
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].non_nil_uuid(), hr_uuid);
    }

    #[test]
    fn test_presenter_to_group() {
        let mut schedule = Schedule::new();
        let member_uuid = nn(1);
        let group_uuid = nn(2);

        add_presenter(&mut schedule, member_uuid, "Alice");
        add_presenter(&mut schedule, group_uuid, "Group A");

        let member_id = PresenterId::from(member_uuid);
        let group_id = PresenterId::from(group_uuid);

        schedule.mark_presenter_group(group_id).unwrap();
        schedule.add_member(member_id, group_id).unwrap();

        let groups = schedule.get_presenter_groups(member_id);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].non_nil_uuid(), group_uuid);

        let members = schedule.get_presenter_members(group_id);
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].non_nil_uuid(), member_uuid);

        assert!(PresenterEntityType::is_group(
            &schedule.entities,
            group_uuid
        ));
        assert!(!PresenterEntityType::is_group(
            &schedule.entities,
            member_uuid
        ));
    }

    #[test]
    fn test_add_presenters_basic() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);

        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");

        let panel_id = PanelId::from(panel_uuid);
        let count = schedule.add_presenters(panel_id, &["P:Alice", "P:Bob"]);

        assert_eq!(count, 2);
        assert_eq!(schedule.get_panel_presenters(panel_id).len(), 2);
    }

    #[test]
    fn test_add_presenters_with_groups() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);

        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");

        let panel_id = PanelId::from(panel_uuid);
        // Alice is a member of TeamA
        let count = schedule.add_presenters(panel_id, &["P:Alice=TeamA"]);

        assert_eq!(count, 1);
        assert_eq!(schedule.get_panel_presenters(panel_id).len(), 1);

        // Verify the group was created
        let presenter = schedule.get_panel_presenters(panel_id).pop().unwrap();
        let groups = schedule.get_presenter_groups(presenter);
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn test_add_presenters_skips_invalid_tags() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);

        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");

        let panel_id = PanelId::from(panel_uuid);
        // Mix of valid and invalid tags - P: prefix auto-creates presenters
        let count = schedule.add_presenters(panel_id, &["P:Alice", "", "invalid", "P:Bob"]);

        // Should add Alice and Bob (auto-created), skip empty and bare "invalid" (not a known name)
        assert_eq!(count, 2);
    }

    #[test]
    fn test_add_presenters_duplicate_ignored() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);

        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");

        let panel_id = PanelId::from(panel_uuid);
        // Add Alice twice
        let count1 = schedule.add_presenters(panel_id, &["P:Alice"]);
        let count2 = schedule.add_presenters(panel_id, &["P:Alice"]);

        // First add succeeds, second returns 0 (presenter already assigned, no duplicate)
        assert_eq!(count1, 1);
        assert_eq!(count2, 0);
        assert_eq!(schedule.get_panel_presenters(panel_id).len(), 1);
    }

    // ------------------------------------------------------------------
    // Backing-field and reverse-index regression tests
    // ------------------------------------------------------------------

    #[test]
    fn test_presenter_ids_backing_field_updated_by_add_presenters() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");

        let panel_id = PanelId::from(panel_uuid);
        schedule.add_presenters(panel_id, &["P:Alice"]);

        let data = schedule.entities.panels.get(&panel_uuid).unwrap();
        assert_eq!(data.presenter_ids.len(), 1);
    }

    #[test]
    fn test_presenter_ids_consistent_with_reverse_index() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");

        let panel_id = PanelId::from(panel_uuid);
        schedule.add_presenters(panel_id, &["P:Alice", "P:Bob"]);

        let data = schedule.entities.panels.get(&panel_uuid).unwrap();
        let backing_ids: Vec<_> = data
            .presenter_ids
            .iter()
            .map(|id| id.non_nil_uuid())
            .collect();

        for &presenter_uuid in &backing_ids {
            let panels = schedule.entities.panels_by_presenter.get(&presenter_uuid);
            assert!(
                panels.is_some_and(|v| v.contains(&panel_uuid)),
                "presenter not in panels_by_presenter reverse index"
            );
        }
    }

    #[test]
    fn test_mark_presenter_group_sets_is_explicit_group() {
        let mut schedule = Schedule::new();
        let group_uuid = nn(1);
        add_presenter(&mut schedule, group_uuid, "Panel A");

        let group_id = PresenterId::from(group_uuid);
        schedule.mark_presenter_group(group_id).unwrap();

        let data = schedule.entities.presenters.get(&group_uuid).unwrap();
        assert!(
            data.is_explicit_group,
            "is_explicit_group should be true after mark"
        );
        assert!(
            PresenterEntityType::is_group(&schedule.entities, group_uuid),
            "is_group should return true"
        );
    }

    #[test]
    fn test_unmark_presenter_group_clears_is_explicit_group() {
        let mut schedule = Schedule::new();
        let group_uuid = nn(1);
        add_presenter(&mut schedule, group_uuid, "Panel A");

        let group_id = PresenterId::from(group_uuid);
        schedule.mark_presenter_group(group_id).unwrap();
        let was_explicit = schedule.unmark_presenter_group(group_id);

        assert!(was_explicit, "should return true when previously explicit");
        let data = schedule.entities.presenters.get(&group_uuid).unwrap();
        assert!(
            !data.is_explicit_group,
            "is_explicit_group should be false after unmark"
        );
    }

    #[test]
    fn test_add_member_updates_group_ids_backing_field() {
        let mut schedule = Schedule::new();
        let member_uuid = nn(1);
        let group_uuid = nn(2);
        add_presenter(&mut schedule, member_uuid, "Alice");
        add_presenter(&mut schedule, group_uuid, "TeamA");

        let member_id = PresenterId::from(member_uuid);
        let group_id = PresenterId::from(group_uuid);
        schedule.add_member(member_id, group_id).unwrap();

        let data = schedule.entities.presenters.get(&member_uuid).unwrap();
        assert_eq!(data.group_ids.len(), 1);
        assert_eq!(data.group_ids[0].non_nil_uuid(), group_uuid);
    }

    #[test]
    fn test_remove_member_clears_group_ids_backing_field() {
        let mut schedule = Schedule::new();
        let member_uuid = nn(1);
        let group_uuid = nn(2);
        add_presenter(&mut schedule, member_uuid, "Alice");
        add_presenter(&mut schedule, group_uuid, "TeamA");

        let member_id = PresenterId::from(member_uuid);
        let group_id = PresenterId::from(group_uuid);
        schedule.add_member(member_id, group_id).unwrap();
        schedule.remove_member(member_id, group_id);

        let data = schedule.entities.presenters.get(&member_uuid).unwrap();
        assert!(
            data.group_ids.is_empty(),
            "group_ids should be empty after remove_member"
        );
    }

    #[test]
    fn test_is_group_computed_via_members() {
        let mut schedule = Schedule::new();
        let member_uuid = nn(1);
        let group_uuid = nn(2);
        add_presenter(&mut schedule, member_uuid, "Alice");
        add_presenter(&mut schedule, group_uuid, "TeamA");

        let member_id = PresenterId::from(member_uuid);
        let group_id = PresenterId::from(group_uuid);

        // Before adding any member: group_uuid is not a group
        assert!(!PresenterEntityType::is_group(
            &schedule.entities,
            group_uuid
        ));

        // After adding a member: group_uuid becomes a group (has members)
        schedule.add_member(member_id, group_id).unwrap();
        assert!(PresenterEntityType::is_group(
            &schedule.entities,
            group_uuid
        ));
        assert!(!PresenterEntityType::is_group(
            &schedule.entities,
            member_uuid
        ));
    }

    #[test]
    fn test_hotel_room_ids_backing_field_initially_empty() {
        let mut schedule = Schedule::new();
        let er_uuid = nn(1);
        let hr_uuid = nn(2);

        add_event_room(&mut schedule, er_uuid, "ER1");
        add_hotel_room(&mut schedule, hr_uuid, "HR-A");

        let data = schedule.entities.event_rooms.get(&er_uuid).unwrap();
        assert!(
            data.hotel_room_ids.is_empty(),
            "hotel_room_ids should be empty before write-closure path"
        );
    }

    #[test]
    fn test_set_is_group_false_clears_members() {
        let mut schedule = Schedule::new();
        let member_uuid = nn(1);
        let group_uuid = nn(2);
        add_presenter(&mut schedule, member_uuid, "Alice");
        add_presenter(&mut schedule, group_uuid, "TeamA");

        let member_id = PresenterId::from(member_uuid);
        let group_id = PresenterId::from(group_uuid);

        schedule.mark_presenter_group(group_id).unwrap();
        schedule.add_member(member_id, group_id).unwrap();

        // Confirm group state
        assert!(PresenterEntityType::is_group(
            &schedule.entities,
            group_uuid
        ));
        assert_eq!(schedule.get_presenter_members(group_id).len(), 1);

        // Writing is_group=false must clear both is_explicit_group AND all members
        schedule.set_is_group(group_id, false);

        assert!(!PresenterEntityType::is_group(
            &schedule.entities,
            group_uuid
        ));
        assert!(schedule.get_presenter_members(group_id).is_empty());
        // member's group_ids should also be cleared
        let data = schedule.entities.presenters.get(&member_uuid).unwrap();
        assert!(data.group_ids.is_empty());
    }

    #[test]
    fn test_event_room_id_backing_field_initially_empty_on_panel() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");

        let data = schedule.entities.panels.get(&panel_uuid).unwrap();
        assert!(
            data.event_room_id.is_none(),
            "event_room_id should be None before write-closure path"
        );
    }
}
