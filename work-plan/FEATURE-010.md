# Enhance presenter group display and filtering in widget

## Summary

Update the schedule widget to properly display presenter groups and allow filtering by both individual presenters and groups, following the group handling logic from the original implementation.

## Status

Open

## Priority

Medium

## Description

The current widget displays presenters as a simple list of names, but doesn't handle the sophisticated group logic from the original schedule-to-html system. Users need to see groups properly formatted and be able to filter by groups like "UNC Staff" in addition to individual presenters.

## Expected behavior

- Groups should be displayed according to original logic (group name when all members attend, individual names when partial attendance)
- Filter dropdown should include both individual presenters and presenter groups
- Group filtering should show all events where any group member is presenting
- Display should handle always-grouped presenters (==Group) correctly

## Implementation Details

### Presenter Display Enhancement

1. **Update widget presenter rendering**:

   ```javascript
   function formatPresenters(presenters) {
       // Group presenters by their group relationships
       // Apply original logic: group name if all members, individual if partial
       // Handle always-grouped presenters (show as group)
       // Handle mixed attendance: "Group Name (Individual Name)"
   }
   ```

2. **Group relationship tracking**:
   - Parse presenter data to identify group memberships
   - Track which presenters belong to which groups
   - Identify always-grouped presenters
   - Determine when to show group vs individual names

### Filter Enhancement

3. **Update presenter filter dropdown**:

   ```javascript
   function buildPresenterFilter(presenters) {
       // Extract all individual presenters
       // Extract all presenter groups
       // Sort groups separately from individuals
       // Display with visual distinction (e.g., "UNC Staff [Group]")
   }
   ```

4. **Group filtering logic**:

   - When group selected, show events where any group member presents
   - When individual selected, show only their specific events
   - Handle cases where presenter belongs to multiple groups
   - Maintain filter state and URL parameters

### Data Structure Updates

5. **Enhanced presenter data in JSON**:

   ```json
   {
     "presenters": [
       {
         "name": "John Doe",
         "rank": "guest",
         "groups": ["UNC Staff"],
         "always_grouped": false,
         "is_group": false
       },
       {
         "name": "UNC Staff", 
         "rank": "staff",
         "members": ["John Doe", "Jane Smith"],
         "always_shown": false,
         "is_group": true
       }
     ]
   }
   ```

6. **Event presenter relationships**:

   ```json
   {
     "events": [
       {
         "presenters": [
           {
             "presenter_id": "john_doe",
             "display_name": "John Doe",
             "shown_as_group": false
           },
           {
             "presenter_id": "unc_staff",
             "display_name": "UNC Staff", 
             "shown_as_group": true,
             "member_count": 3
           }
         ]
       }
     ]
   }
   ```

### Widget UI Updates

7. **Presenter display formatting**:

   - Group names shown when all members attend
   - Individual names shown for partial attendance
   - Always-grouped presenters always shown as group
   - Mixed attendance: "Group Name (Individual Name)"

8. **Filter dropdown styling**:

   - Groups shown with different styling (bold, icon, etc.)
   - Clear visual distinction between individuals and groups
   - Alphabetical sorting within each category

### Testing

9. **Test scenarios**:

   - Single presenter events
   - Group events with all members attending
   - Group events with partial attendance
   - Always-grouped presenters
   - Presenters in multiple groups
   - Mixed individual and group presenters in same event

## Acceptance Criteria

- Presenter groups display correctly according to original logic
- Filter includes both individuals and groups with clear distinction
- Group filtering shows all relevant events
- Always-grouped presenters always shown as groups
- Mixed attendance displays as "Group Name (Individual Name)"
- Backward compatibility with existing presenter data
