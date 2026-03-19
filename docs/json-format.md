# Schedule JSON Format — Index

The JSON format has been updated to version 5. See the appropriate document below.

## Version 5 (current)

- **[json-private-v5.md](json-private-v5.md)** — Full/internal format used by
  the Rust editor and converter. Contains all fields including private
  internal-use fields. The `panels` top-level key is a hash indexed by base ID.

- **[json-public-v5.md](json-public-v5.md)** — Public format consumed by the
  `widget/cosam-calendar.js` widget. Flat ordered `panels` array with only
  public fields. Private fields are excluded.

## Version 4 (archived)

- **[json-format-v4.md](json-format-v4.md)** — Specification for the v4 format
  and earlier. Kept for reference; no longer produced by current tooling.
