# Handle group presenter conflicts intelligently

## Summary

Enable presenter conflict detection to distinguish between individual presenters and groups, allowing groups like "UNC Staff" to be scheduled in multiple panels simultaneously.

## Status

Completed

## Priority

High

## Description

Currently the converter flags conflicts when the same presenter name appears in overlapping events, but this creates false positives for presenter groups. Groups like "UNC Staff", "Pros and Cons", or "Guest Panelists" represent multiple people who can be in different panels at the same time.

The 2025 schedule shows this issue:

- UNC Staff scheduled for both "Parasol History and Construction" (10:00-11:00, room 4)
- UNC Staff also scheduled for "Reshaping the Body" (10:00-11:00, room 5)

This is not a real conflict since UNC Staff represents multiple staff members who can be split across different panels.

## Expected behavior

- Individual presenters should still trigger conflict warnings for double-booking
- Group presenters should be allowed to be in multiple overlapping panels
- Need a way to distinguish individual presenters from groups
- Should handle both named groups and generic group labels

## Implementation Details

### Group Detection Strategy

Based on the spreadsheet format and original implementation, groups can be identified in several ways:

1. **Header format with =Group suffix**:

   - `G:Name=Group` indicates presenter is member of Group, eg:
     - `G:Pro==Pros and Cons`
   - `S:UNC Staff=Staff` indicates UNC Staff member
   - Group name comes from header, cell is just a flag

2. **Group names in "Other" columns**:

   - `G:Other`, `S:Other` columns contain comma-separated names
   - Names like "UNC Staff", "Pros and Cons" indicate groups
   - Parse with separator regex: `\s*(?:,\s*(?:and\s+)?|\band\s+)`

3. **Group patterns and configuration**:

   - Names ending with "Staff", "Team", "Crew"
   - Organization names like "UNC Staff", "MIT Team"
   - Generic labels like "Others", "Multiple Presenters"

4. **Presenter group relationships** (from original implementation):

   - Presenters can be members of groups
   - Groups can have multiple members
   - Groups can be "always shown" vs individual members

### Conflict Detection Enhancement

Update `Convert::Events::detect_conflicts()`:

1. **Add group detection function**:

   ```perl
   sub _is_group_presenter ($presenter_name, $presenter_info) {
       # Check if presenter comes from =Group header
       return 1 if $presenter_info->{is_group_member};
       
       # Check against known group patterns
       return 1 if $presenter_name =~ m{\b (Staff|Team|Crew|Panelists|Guests) \b}xmsi;
       return 1 if $presenter_name =~ m{\A (UNC|MIT|NYU) \s+ \S+}xmsi;
       
       # Check against configuration list
       return 1 if exists $group_config{$presenter_name};
       
       return 0;
   }
   ```

2. **Enhanced presenter parsing**:

   - Parse `=Group` suffix from presenter headers
   - Track group membership relationships
   - Distinguish between individual presenters and group names

3. **Skip group conflicts**:

   - If either presenter in a conflict is a group, skip the warning
   - Still track the conflict in JSON data for widget reference
   - Mark conflict type as "group_presenter" vs "individual_presenter"

4. **Enhanced warning format**:

   - Individual conflicts: "Presenter X is double-booked"
   - Group conflicts: No warning (groups can be in multiple places)
   - Mixed conflicts: Optional info message about group vs individual

### Configuration Options

1. **YAML configuration file**:

   ```yaml
   group_presenters:
     - "UNC Staff"
     - "Pros and Cons"
     - "Guest Panelists"
   group_patterns:
     - ".* Staff"
     - ".* Team"
     - "Others"
   ```

2. **Command line flag**:
   - `--no-group-conflicts` to disable group conflict checking
   - `--group-config` to specify group configuration file

### Testing

- Verify UNC Staff no longer triggers conflict warnings
- Verify individual presenters still trigger conflict warnings
- Test with various group name patterns
- Test configuration file loading
- Test mixed scenarios (group + individual in same event)

## Acceptance Criteria

- No warnings for UNC Staff overlapping with itself
- Still warnings for individual presenter double-booking
- Configurable group detection patterns
- Backward compatibility with existing schedules
- Clear documentation of group vs individual presenter logic
