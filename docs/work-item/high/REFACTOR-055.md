# Transaction System with Bundling

## Summary

Implement command-based transaction system with undo/redo stacks and scope-based bundling for atomic multi-operation changes.

## Status

Not Started

## Priority

High

## Description

Port EditCommand concepts from schedule-core into a unified transaction system. Supports bundling multiple operations into single undo steps, critical for UI actions like "import panel with presenters" or "merge import". Replaces REFACTOR-005.

## Implementation Details

### 1. Define Operation Types

**File: `edit/operation.rs`**

```rust
#[derive(Clone, Debug)]
pub enum Operation {
    EntityCreate { type_name: String, uuid: NonNilUuid, initial_fields: FieldMap },
    EntityUpdate { type_name: String, uuid: NonNilUuid, field_changes: Vec<FieldChange> },
    EntityDelete { type_name: String, uuid: NonNilUuid },
    EdgeCreate { edge_type: String, from: NonNilUuid, to: NonNilUuid },
    EdgeDelete { edge_type: String, uuid: NonNilUuid },
}

#[derive(Clone, Debug)]
pub struct FieldChange {
    pub field_name: String,
    pub old_value: Option<FieldValue>,
    pub new_value: Option<FieldValue>,
}
```

### 2. Implement Transaction

**File: `edit/transaction.rs`**

```rust
pub struct Transaction {
    pub id: NonNilUuid,
    pub description: String,
    pub operations: Vec<Operation>,
    pub timestamp: DateTime<Utc>,
}

impl Transaction {
    pub fn apply(&self, schedule: &mut Schedule) -> Result<(), TransactionError> { ... }
    pub fn inverse(&self) -> Transaction { ... }  // Generate undo transaction
}
```

### 3. Transaction Manager with Bundling

**File: `edit/manager.rs`**

```rust
pub struct TransactionManager {
    history: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
    current_bundle: Option<Transaction>,
    max_history: usize,
}

impl TransactionManager {
    pub fn scoped<F, R>(&mut self, description: &str, f: F) -> Result<R, TransactionError>
        where F: FnOnce(&mut ScopedTransaction) -> Result<R, TransactionError>;
    
    pub fn push_bundle(&mut self, description: &str);
    pub fn pop_bundle(&mut self);
    pub fn undo(&mut self, schedule: &mut Schedule) -> Result<(), TransactionError>;
    pub fn redo(&mut self, schedule: &mut Schedule) -> Result<(), TransactionError>;
}

pub struct ScopedTransaction<'a> {
    manager: &'a mut TransactionManager,
    ops: Vec<Operation>,
}
```

### 4. Integration with Schedule

**File: `schedule/mod.rs`**

- Add `transaction_manager: TransactionManager` field
- All mutation methods (add_entity, update_entity, etc.) record operations
- Auto-bundling for API operations (e.g., add_panel_with_presenters)

### 5. Edge-Entity Transaction Support

- Ensure edge-entity operations bundle with parent entity operations
- Undo of panel creation also removes edge-entities

## Acceptance Criteria

- [ ] Operation enum covers all entity and edge-entity mutations
- [ ] Transaction type with apply() and inverse() methods
- [ ] TransactionManager with history and redo stacks
- [ ] Scoped bundling API works
- [ ] Undo/redo operations restore previous state
- [ ] Auto-bundling for compound operations (panel + edges)
- [ ] Configurable max history depth
- [ ] All existing tests pass
- [ ] REFACTOR-005 marked as superseded/complete

## Dependencies

- REFACTOR-052 (Edge to Edge-Entity) - edge-entity operations need this
- REFACTOR-053 (Builder & Partial Updates) - transactions wrap builder operations

## Notes

This phase supersedes REFACTOR-005 (Edit Commands/History). The scoped bundling API addresses the "user action level" undo requirement - a merge import can be one undoable bundle.

When complete, update REFACTOR-051.md to mark REFACTOR-055 as complete.
