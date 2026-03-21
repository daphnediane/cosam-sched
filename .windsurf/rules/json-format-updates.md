# JSON Format Documentation Updates

This document outlines the process for updating JSON format documentation when introducing new schema versions.

## Process Overview

When adding a new JSON format version (e.g., v6 → v7):

### 1. Create Structure Documentation

Create individual markdown files for each structure in the new version:

```bash
# For version variants (like v5/v7)
docs/json-schedule/meta-v7.md          # shared meta
docs/json-schedule/panels-v7.md       # private format
docs/json-schedule/panels-public-v7.md # public format
docs/json-schedule/panelTypes-v7.md   # panel type definitions
# ... other structures as needed
```

### 2. Structure Documentation Template

Each structure file should follow this format:

```markdown
# [Structure Name] v7

Brief description of the structure and its purpose.

**Access:** Public/Private

**Status:** Supported in v7

## Key Changes from v6

- List of significant changes
- New fields added
- Format changes (e.g., UID format)
- Behavioral changes

## Fields

| Field | Type | Public | Description |
|-------|------|--------|-------------|
| ... field definitions ... |

## Examples

```json
{... example JSON ...}
```

## Migration Notes

- How to migrate from previous version
- Breaking changes
- Compatibility considerations
```

### 3. Create Version Entry File

Create the main version entry file:

```bash
docs/json-schedule/v7-public.md
```

The entry file should include:

- Top-level JSON structure
- Structures section with links to all structure files
- Key changes from previous version
- Migration notes
- Complete example

### 4. Update Main Documentation

Update the main documentation files:

1. **Update README.md** - Add the new version to the version history section
2. **Update `docs/json-format.md`** - Add links to the new generated documents

### 5. Regenerate Combined Documentation

Run the documentation generator:

```bash
# From project root
perl scripts/combine-json-docs.pl
```

This will automatically:
- Discover the new version files
- Extract structure dependencies
- Generate combined documents in `docs/`:
  - `docs/json-format-v7.md` (for simple versions)
  - `docs/json-private-v7.md` (for private variants)
  - `docs/json-public-v7.md` (for public variants)

## Documentation Standards

### Field Tables

- Use aligned markdown tables
- Include Type, Public, Description columns
- Mark optional fields clearly
- Note breaking changes

### Code Examples

- Use fenced code blocks with json language
- Include realistic example data
- Show both old and new formats for comparison
- Highlight new fields in examples

### Migration Guidance

- Clearly document breaking changes
- Provide migration paths
- Note compatibility considerations
- Include timeline for deprecation if applicable

## Version Numbering

- Increment major version for breaking changes
- Use minor version for additive changes
- Document version compatibility matrix
- Note when older versions are deprecated

## Quality Checklist

Before submitting new format documentation:

- [ ] All structure files created with proper template
- [ ] Version entry file includes complete example
- [ ] README.md updated with version history
- [ ] Field tables are properly aligned
- [ ] Code examples are valid JSON
- [ ] Migration notes are comprehensive
- [ ] Combined documentation regenerated
- [ ] Links between documents work correctly
- [ ] Breaking changes clearly documented

## Generated Files

The following files are automatically generated and should not be edited directly:

- `docs/json-format-v4.md`
- `docs/json-private-v5.md`
- `docs/json-public-v5.md`
- `docs/json-public-v7.md`
- (Future versions as added)

Edit the source files in `docs/json-schedule/` instead.

## Testing Documentation

After updating documentation:

1. Test code examples for validity
2. Verify all links work correctly
3. Check that generated documentation matches expectations
4. Validate migration examples work as described
5. Test with actual schedule data if applicable
