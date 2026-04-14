# IDEA-048: Extended config file handling

## Summary

Extend the current `DeviceConfig` / `identity.toml` system with richer
identity fields, per-app metadata, and optional named profiles.

## Status

Open

## Priority

Low

## Description

The basic config system is already implemented in
`crates/schedule-data/src/crdt/actor.rs` (`DeviceConfig`).  This idea
records the extensions that were deferred from that initial implementation.

### What is already implemented

The config directory layout is:

```text
~/Library/Application Support/com.CosplayAmerica.cosam_sched/   (macOS)
C:\Users\<user>\AppData\Roaming\CosplayAmerica\cosam_sched\     (Windows)
~/.config/cosam_sched/                                           (Linux)
├── identity.toml            ← display_name; shared across all apps
├── actor-cosam-editor.toml  ← actor UUID for the GUI editor
└── actor-cosam-modify.toml  ← actor UUID for the CLI tool
```

`identity.toml` currently holds only `display_name`.  Each `actor-<app>.toml`
holds only `actor_id` (UUID v4).  Apps call `DeviceConfig::load_or_create` at
startup to obtain both values.

### Deferred: richer identity fields

The identity file should grow to include optional user metadata used for
change attribution and (eventually) role-based ranking:

```toml
# identity.toml
version = 1
display_name = "Daphne"           # required; shown in change attribution
real_name = "Daphne Pfister"      # optional; full legal name
public_email = "..."              # optional; contact email
role = "Programming"              # optional; one of the roles below
base_uuid = "..."                 # UUID v7; generated once, never changes
profiles = ["director"]           # optional; names of non-default profiles
```

Defined roles: `Programming`, `Director`, `Website`, `Printing`, `Promotion`.
Role will be used for ranking/ordering in a future feature.  The `base_uuid`
is created when the identity file is first written and never regenerated; it
serves as a stable human identity across devices (distinct from the per-app
actor UUIDs used by the CRDT layer).

### Deferred: per-app config metadata

The current `actor-<app>.toml` file holds only the actor UUID.  It should
grow to support arbitrary app-specific metadata so each app can persist its
own preferences without needing a separate file:

```toml
# actor-cosam-editor.toml
version = 1
app_version = "0.1.0"
actor_id = "..."
[meta]
# app-defined key/value pairs go here
last_open_dir = "/Volumes/..."
```

Open question: should `meta` allow only flat string keys, or arbitrary TOML
values?

### Deferred: actor UUID derivation strategy

Currently each app stores a random UUID v4 in `actor-<app>.toml`, generated
once on first launch and reused forever.  Two alternative strategies are worth
considering before the file format is finalized.

#### Derived UUIDs via UUID v5

UUID v5 produces a deterministic UUID from a namespace UUID and a name string
(SHA-1 based).  If the identity file gains a `base_uuid`, per-app actor UUIDs
could be derived rather than stored:

```
actor_id = UUID_v5(namespace: base_uuid, name: app_name)
```

This means the `actor-<app>.toml` files become unnecessary for the actor UUID
itself — the UUID can always be recomputed from the identity.  Advantages:

- One fewer file to manage and sync
- Actor UUID is stable across reinstalls as long as `identity.toml` is kept

Disadvantages:

- Two devices with the same `base_uuid` (e.g. from a synced dotfiles repo)
  would produce identical actor UUIDs, which breaks CRDT correctness
- Requires `base_uuid` to be in place before any actor UUID can be derived,
  complicating first-launch ordering

A hybrid: use UUID v5 to derive a *device-scoped* UUID by including a
per-device salt (the current `actor-<app>.toml` UUID) in the name string:

```
actor_id = UUID_v5(namespace: base_uuid, name: "<device-uuid>/<app_name>")
```

This retains uniqueness across devices while still being deterministic given
both inputs.

#### Per-session UUIDs

Some app invocations may prefer a fresh actor UUID each run so that every
session is distinguishable in the CRDT history (e.g. a batch CLI tool that
should not appear to share identity across invocations).  Options:

- **UUID v7** — time-ordered, monotonic; fresh per launch; no coordination
  needed
- **PID-based** — combine a stable device UUID with the process ID:
  `UUID_v5(device_uuid, "<pid>")`.  Guaranteed unique while the process is
  alive; PID values are not reused by the OS until after the process exits.
- **Timestamp** — similar to UUID v7 but can be implemented without the
  uuid v7 crate if it is not already a dependency

Per-session UUIDs cause automerge's internal actor table to grow by one entry
per invocation, which is harmless for tools run infrequently but should be
considered for high-frequency batch use.

`DeviceConfig::load_or_create` could gain a `session: bool` parameter (or a
separate `DeviceConfig::session(app)` constructor) to select per-session behavior without changing the default persistent path.

### Deferred: named profiles

A profile is a named subdirectory that overrides the default identity and/or
app config for a specific role or context (e.g. acting as Director at one con
while Programming at another):

```text
~/.config/cosam_sched/
├── identity.toml
├── actor-cosam-editor.toml
└── profile-director/
    ├── identity.toml        ← overrides display_name and role for this profile
    └── actor-cosam-editor.toml  ← separate actor UUID for the director role
```

The active profile would be selected via a CLI flag or env var.  Profile names
are listed in the root `identity.toml` so apps can enumerate them.

Open questions:

- Does each profile get its own actor UUID, or share the device UUID?  (Sharing
  gives simpler merge history; separate UUIDs let the CRDT layer distinguish
  which role authored each change.)
- Should `DeviceConfig::load_or_create` gain a `profile: Option<&str>`
  parameter, or should profile selection be handled by the caller before
  calling into `schedule-data`?
- What happens when a profile-specific `identity.toml` is absent — fall through
  to the root identity, or error?

