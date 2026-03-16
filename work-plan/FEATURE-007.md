# Reference panel types by UID instead of hardcoding colors

## Summary

Replace hardcoded panel type colors with CSS-based UID reference system for theming.

## Status

Completed

## Priority

High

## Description

Currently panel type colors are hardcoded in the event data, making it difficult to implement themes and maintain consistent styling. This change will make panel types reference UIDs and use CSS classes for colors, enabling proper theming support.

## Implementation Details

- Create UID-based panel type system (e.g., panel-type-au, panel-type-gp, etc.)
- Update converter to output panel type UIDs instead of color codes
- Define CSS color classes for each panel type in the stylesheet
- Update JavaScript to apply CSS classes based on panel type UIDs
- Ensure this change is a prerequisite for UI-003 (dark/light mode switching)
- Update any existing hardcoded color references in the codebase

## Acceptance Criteria

- Panel types reference UIDs instead of hardcoded colors
- CSS classes define all panel type colors
- Converter outputs clean UID-based panel type data
- Theming system can easily override panel type colors
- No hardcoded colors remain in event data
- All existing functionality is preserved
- This enables UI-003 dark/light mode implementation
