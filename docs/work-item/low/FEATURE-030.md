# Document Schedule-Data Custom Field Extensions

## Summary

Document custom fields in schedule-data that are not present in schedule-core.

## Status

Not Started

## Priority

Low

## Description

The schedule-data crate includes custom fields that are useful for the editor but are not present in schedule-core's XLSX/JSON processing. These fields should be documented to distinguish them from canonical fields.

## Custom Fields to Document

### Presenter Entity

- `bio: Option<String>` - Presenter's biography
- `pronouns: Option<String>` - Presenter's preferred pronouns
- `website: Option<String>` - Presenter's website

### Edge-Entity Metadata (Future Considerations)

The following metadata fields were considered during REFACTOR-052 (edge-to-entity migration) but removed pending future needs:

**PanelToPresenter Edge:**

- `is_primary: bool` - Designate primary presenter for panels with multiple presenters
- `confirmed: bool` - Track presenter participation confirmation status

**PanelToEventRoom Edge:**

- `is_primary_room: bool` - Designate primary room for panels spanning multiple rooms
- Note: Also consider if this belongs with HotelRoom rather than EventRoom

## Rationale

These fields provide additional presenter information useful for the editor interface and potential future features like presenter profiles. They are not needed for schedule-core's primary responsibilities of XLSX import/export and JSON generation for the schedule widget.

## Future Considerations

Consider whether these custom fields should be moved to a separate presenter-profiles entity if the number of editor-specific presenter fields grows significantly.

## Acceptance Criteria

- Custom fields documented in this work plan
- Documentation explains rationale for each custom field
- Future considerations noted for potential refactoring
