# CRDT Integration

## Summary

Integrate rust-crdt for operation-based replication, enabling distributed sync and offline editing with automatic conflict resolution.

## Status

Not Started

## Priority

Medium

## Description

Replace transaction history with CRDT-backed storage using rust-crdt's CmRDT (operation-based) types. Operations from multiple editors merge automatically, supporting offline editing and distributed synchronization.

## Implementation Details

### 1. Add rust-crdt Dependency

**File: `Cargo.toml`**

```toml
[dependencies]
crdts = "7"  # rust-crdt crate
```

### 2. Implement CmRDT for Operations

**File: `edit/crdt.rs`**

```rust
use crdts::{CmRDT, GSet, Map, VClock};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleOp {
    pub actor: ActorId,           // Site/editor identifier
    pub counter: u64,             // Lamport clock for ordering
    pub op: Operation,            // The actual change
    pub timestamp: DateTime<Utc>,
}

pub type ActorId = u32;

impl CmRDT for ScheduleOp {
    fn valid(&self, ops: &GSet<ScheduleOp>) -> bool {
        // Validate causal ordering - operation must not precede its dependencies
        true
    }
    
    fn apply(&self, state: &mut ScheduleState) {
        // Apply operation to local state
        match &self.op {
            Operation::EntityCreate { .. } => { ... }
            Operation::EntityUpdate { .. } => { ... }
            // ... etc
        }
    }
}
```

### 3. CRDT-Based Operation Log

**File: `edit/oplog.rs`**

```rust
pub struct OpLog {
    pub ops: GSet<ScheduleOp>,           // Grow-only set of all operations
    pub actor_id: ActorId,              // This site's identifier
    pub counter: u64,                   // Monotonic operation counter
}

impl OpLog {
    pub fn new_op(&mut self, op: Operation) -> ScheduleOp {
        self.counter += 1;
        ScheduleOp {
            actor: self.actor_id,
            counter: self.counter,
            op,
            timestamp: Utc::now(),
        }
    }
    
    pub fn merge(&mut self, other: &OpLog) {
        self.ops.merge(&other.ops);
    }
}
```

### 4. Entity Storage with CRDT Fields

**File: `schedule/storage.rs`**

For fields that may conflict during concurrent edits:

```rust
use crdts::Map;

// For fields where last-write-wins is acceptable:
pub type EntityFields = Map<String, LWWReg<FieldValue>, ActorId>;

// LWWReg = Last-Write-Wins Register
// Automatically resolves conflicts by timestamp/actor priority
```

### 5. Merge UI Hooks

**File: `edit/merge.rs`**

```rust
pub enum MergeConflict {
    FieldConflict { entity_id: NonNilUuid, field: String, local: FieldValue, remote: FieldValue },
    EntityDeleted { entity_id: NonNilUuid },
}

pub trait MergeResolver {
    fn resolve_conflict(&self, conflict: MergeConflict) -> Resolution;
}
```

### 6. Sync Protocol

**File: `sync/mod.rs`**

```rust
pub fn export_operations_since(&self, since: Option<DateTime<Utc>>) -> Vec<ScheduleOp>;
pub fn import_operations(&mut self, ops: Vec<ScheduleOp>) -> Result<Vec<MergeConflict>, SyncError>;
```

## Acceptance Criteria

- [ ] rust-crdt dependency added
- [ ] CmRDT implemented for ScheduleOp
- [ ] OpLog using GSet for operation storage
- [ ] Operations merge correctly via set union
- [ ] Edge-entity operations merge correctly
- [ ] Actor ID assignment for each editor instance
- [ ] Export/import operations for sync
- [ ] Conflict detection with UI hooks
- [ ] Offline editing and reconciliation works
- [ ] All existing tests pass

## Dependencies

- REFACTOR-052 (Edge to Edge-Entity) - edge-entities need CRDT support
- REFACTOR-055 (Transaction System) - CRDT operations replace transaction history

## Notes

This is the foundation for "convention after next" offline editing. The CRDT design ensures operations from different editors can always merge without data loss.

### Secondary Index Considerations

Edge-entity storage uses HashMap<NonNilUuid, EdgeData> with V5 UUIDs. For endpoint-based queries (e.g., "all presenters for a panel"), we may need secondary indexes. CRDT implications:

- **Recompute on merge**: Rebuild indexes after merge (simpler, O(n) per merge)
- **CRDT indexes**: Make indexes themselves CRDTs (complex, but incremental)

Decision deferred to implementation phase based on performance needs.

When complete, update REFACTOR-051.md to mark REFACTOR-057 as complete.
