# Schedule JSON Format Documentation

This directory contains comprehensive documentation for the JSON file format used by the Cosplay America schedule system. The format is produced and consumed by the Rust editor (`apps/cosam-editor`) and Rust converter (`apps/cosam-convert`).

## Version History

The JSON format has evolved through several major versions:

- **v1-v3**: Legacy formats (see historical notes in v4 documentation)
- **v4**: Current stable format with flat events array and timeline support
- **v5**: Hierarchical panels structure (private/public variants)
- **v6**: Excel metadata integration
- **v7**: Latest format with panelTypes hashmap, named color sets, merged timeTypes, stable presenter IDs, baked-in breaks (full/display variants)
- **v8**: Full format with persistent edit history via changeLog field

## Format Variants

Starting with v5, the format has two variants:

### Full Format (Private)

- **Purpose**: Internal data storage and editing
- **Structure**: Hierarchical panels with base→part→session nesting
- **Fields**: Complete data including internal notes, workshop requirements, optional metadata
- **Access**: Private/internal use only

### Display Format (Public)

- **Purpose**: Public schedule display and widget consumption
- **Structure**: Flattened panels array for simple rendering, with baked-in breaks (v7+)
- **Fields**: Public-facing data only (credits, not internal notes; no metadata)
- **Access**: Public consumption

Note: In v5–v6 the variant names were `"full"` and `"public"`. In v7+ they are `"full"` and `"display"`.

## Adding a New Version

To add a new JSON format version (e.g., v6):

### 1. Create Structure Documentation

Create individual markdown files for each structure in the new version:

```bash
# For a simple version (like v4)
docs/json-schedule/meta-v6.md
docs/json-schedule/events-v6.md
docs/json-schedule/rooms-v6.md
# ... other structures

# For version variants (like v5)
docs/json-schedule/meta-v6.md          # shared meta
docs/json-schedule/panels-v6.md       # private format
docs/json-schedule/panels-public-v6.md # public format
docs/json-schedule/PanelPart-v6.md    # private format details
# ... other structures
```

Each structure file should follow this template:

- Structure name and description
- Access level (Public/Private)
- Status (Supported/Dropped)
- Fields table with Type, Public, Description columns
- Additional explanatory sections
- JSON examples

### 2. Create Version Entry File

Create the main version entry file:

```bash
# Simple version
docs/json-schedule/v6.md

# Version variants  
docs/json-schedule/v6-private.md
docs/json-schedule/v6-public.md
```

The entry file should include:

- Top-level JSON structure
- Structures section with links to all structure files
- Key changes from previous version
- Migration notes
- Complete example

### 3. Update Main Documentation

Update the main documentation files:

1. **Update this README** - Add the new version to the version history section
2. **Update `docs/json-format.md`** - Add links to the new generated documents in the appropriate version section (current or archived)

The `docs/json-format.md` file serves as the main index for all JSON format versions and should be updated to include links to the newly generated combined documents.

### 4. Regenerate Combined Documentation

Run the documentation generator:

```bash
perl scripts/combine-json-docs.pl
```

This will automatically:

- Discover the new version files
- Extract structure dependencies
- Generate combined documents in `docs/`:
  - `docs/json-format-v6.md` (for simple versions)
  - `docs/json-private-v6.md` (for private variants)
  - `docs/json-public-v6.md` (for public variants)

## Regenerating Documentation

After making changes to any structure files or adding new versions, regenerate the combined documentation:

```bash
# From project root
perl scripts/combine-json-docs.pl
```

The script will:

- Scan `docs/json-schedule/` for version entry files (`v#.md`, `v#-*.md`)
- Extract structure references from each entry file
- Generate comprehensive combined documents in `docs/`
- Clean up markdown formatting to pass lint checks

## Generated Files

The following files are automatically generated and should not be edited directly:

- `docs/json-format-v4.md`
- `docs/json-private-v5.md`
- `docs/json-public-v5.md`
- `docs/json-private-v6.md`
- `docs/json-public-v6.md`
- (Future versions as added)

Edit the source files in `docs/json-schedule/` instead.

## Documentation Structure

Each data structure is documented in its own file following the pattern `<structure>-v<version>.md`:

### v4 Documentation

- [v4.md](v4.md) - Main v4 format entry point
- [meta-v4.md](meta-v4.md) - Metadata structure
- [events-v4.md](events-v4.md) - Event objects
- [rooms-v4.md](rooms-v4.md) - Room definitions
- [panelTypes-v4.md](panelTypes-v4.md) - Panel type categories
- [timeTypes-v4.md](timeTypes-v4.md) - Time type categories
- [timeline-v4.md](timeline-v4.md) - Timeline markers
- [presenters-v4.md](presenters-v4.md) - Presenter and group definitions
- [conflicts-v4.md](conflicts-v4.md) - Conflict detection structures

### v5 Documentation

- [v5-private.md](v5-private.md) - Private format entry point
- [v5-public.md](v5-public.md) - Public format entry point
- [meta-v5.md](meta-v5.md) - Metadata structure (shared)
- [panels-v5.md](panels-v5.md) - Hierarchical panels hash (private)
- [PanelPart-v5.md](PanelPart-v5.md) - Panel part objects (private)
- [PanelSession-v5.md](PanelSession-v5.md) - Panel session objects (private)
- [panels-public-v5.md](panels-public-v5.md) - Flattened panels array (public)

### v6 Documentation

- [v6-private.md](v6-private.md) - Private format entry point
- [v6-public.md](v6-public.md) - Public format entry point
- [meta-v6.md](meta-v6.md) - Metadata structure with Excel metadata

### v7 Documentation

- [v7-full.md](v7-full.md) - Full format entry point
- [v7-display.md](v7-display.md) - Display format entry point
- [meta-v7.md](meta-v7.md) - Metadata with `nextPresenterId` and variant naming
- [panelTypes-v7.md](panelTypes-v7.md) - Panel types hashmap with named color sets
- [rooms-v7.md](rooms-v7.md) - Room definitions with `is_break` flag
- [presenters-v7.md](presenters-v7.md) - Presenters with stable integer ID and corrected group semantics
- [panels-v7.md](panels-v7.md) - Hierarchical panels hash (full format)
- [PanelSession-v7.md](PanelSession-v7.md) - Panel session objects (`extras` → `metadata`)
- [panels-display-v7.md](panels-display-v7.md) - Flattened panels with baked-in breaks (display format)
- [timeline-v7.md](timeline-v7.md) - Timeline markers referencing panelType prefix
- [conflicts-v7.md](conflicts-v7.md) - Conflict detection structures

### v8 Documentation

- [v8-full.md](v8-full.md) - Full format entry point with changeLog support
- [meta-v8.md](meta-v8.md) - Metadata with version 8 and variant `"full"`
- [changeLog-v8.md](changeLog-v8.md) - Edit history with undo/redo stacks

## Quick Reference

| Version    | Entry Point                    | Structure                   | Use Case             |
| ---------- | ------------------------------ | --------------------------- | -------------------- |
| v4         | [v4.md](v4.md)                 | Flat events array           | Legacy compatibility |
| v5-private | [v5-private.md](v5-private.md) | Hierarchical panels         | Internal editing     |
| v5-public  | [v5-public.md](v5-public.md)   | Flattened panels            | Public widget        |
| v6-private | [v6-private.md](v6-private.md) | Hierarchical + Excel meta   | Internal editing     |
| v6-public  | [v6-public.md](v6-public.md)   | Flattened + Excel meta      | Public widget        |
| v7-full    | [v7-full.md](v7-full.md)       | Hashmap panelTypes + breaks | Internal editing     |
| v7-display | [v7-display.md](v7-display.md) | Flattened + baked breaks    | Public widget        |
| v8-full    | [v8-full.md](v8-full.md)       | Full format + changeLog     | Internal editing     |

## Migration Notes

- **v4 → v5-private**: Convert flat `events` array to hierarchical `panels` hash
- **v5-private → v5-public**: Flatten hierarchical structure and filter private fields
- **v4 → v5-public**: Use v4→v5-private conversion then flatten for public output
- **v6 → v7**: No migration needed — all JSON files are regenerated from spreadsheet each release
- **v7 → v8**: No migration needed — alpha software, all files regenerated from canonical spreadsheets

## Related Documentation

- [../json-format-v4.md](../json-format-v4.md) - Original v4 format documentation
- [../json-private-v5.md](../json-private-v5.md) - Original v5 private format documentation  
- [../json-public-v5.md](../json-public-v5.md) - Original v5 public format documentation

These original documents are preserved for reference but have been superseded by the structured documentation in this directory.
