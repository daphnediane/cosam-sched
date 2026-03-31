---
description: Accessibility requirements for schedule display
trigger: glob
globs: apps/cosam-editor/**/*.rs,widget/**/*.js,widget/**/*.css,widget/**/*.html,widget/**/*.htm
---

# Accessibility Requirements

## Standards

- **WCAG 2.1 AA** minimum compliance
- **WAI-ARIA** for dynamic content and screen readers
- Reference: <https://www.w3.org/WAI/standards-guidelines/>

## Screen Readers

### Semantic Structure

- Use proper HTML5 semantic elements (`<header>`, `<main>`, `<nav>`, `<section>`, `<article>`, `<footer>`)
- Implement proper heading hierarchy (h1 → h2 → h3)
- Use `<table>` with proper headers OR accessible grid layouts

### Grid Layout Accessibility

- Maintain logical source order regardless of CSS grid visual layout
- Avoid CSS `order` property for screen readers
- Use `grid-auto-flow: row` for reading order consistency
- Provide proper grid labeling with `aria-label` or `aria-labelledby`
- Test grid layouts with screen readers to verify content sequence

### ARIA Implementation

- Add `aria-label` or `aria-labelledby` to interactive elements
- Use `role` attributes where semantic HTML is insufficient
- Implement `aria-live` regions for dynamic content updates
- Use `aria-expanded` for collapsible sections

### Alternative Text

- Provide descriptive `alt` text for meaningful images
- Use `aria-describedby` for complex table relationships
- Include skip navigation links for keyboard users

## Color Blindness

### Color Independence

- Never use color as the only means of conveying information
- Ensure sufficient contrast ratio (4.5:1 normal text, 3:1 large text)
- Use patterns, textures, or icons in addition to color coding

### Visual Indicators

- Use icons or symbols alongside color coding
- Implement hover/focus states that don't rely solely on color
- Provide text labels for color-coded categories

### Testing

- Test with color blindness simulators
- Verify readability in grayscale
- Ensure sufficient contrast for interactive elements

## Keyboard Navigation

- All interactive elements must be keyboard accessible
- Implement logical tab order through schedule elements
- Provide visible focus indicators
- Support keyboard shortcuts for common actions

## Testing and Validation

- Test with actual screen readers (NVDA, JAWS, VoiceOver)
- Use automated accessibility testing tools
- Conduct manual keyboard-only navigation testing
- Validate with WAI-ARIA best practices

## Checklist

- [ ] Semantic HTML structure
- [ ] Proper ARIA attributes
- [ ] Sufficient color contrast
- [ ] Non-color visual indicators
- [ ] Keyboard navigation support
- [ ] Screen reader compatibility
- [ ] Accessibility testing completed
