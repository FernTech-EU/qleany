# API Reference - Rust

This document is the API reference for Qleany-generated Rust code. It covers the APIs you interact with as a developer: **Entity Controllers**, **Feature Controllers**, and the **Unit of Work proc macros** you adapt when implementing custom use cases.

For general architecture and code structure, see [Generated Code - Rust](generated-code-rust.md).

---

## Entity Controller

**File:** `crates/direct_access/src/{entity}/{entity}_controller.rs`

Entity controllers are free functions (not methods on a struct) that provide CRUD and relationship operations on a single entity type. All operations are **synchronous** and return `anyhow::Result<T>`.

### CRUD Functions

#### create_orphan

```rust
pub fn create_orphan(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    entity: &CreateCarDto,
) -> Result<CarDto>
```

Creates a single entity without an owner.

#### create_orphan_multi

```rust
pub fn create_orphan_multi(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    entities: &[CreateCarDto],
) -> Result<Vec<CarDto>>
```

Batch version of `create_orphan`.

#### create

```rust
// Only available if the entity has an owner (defined in the manifest)
pub fn create(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    entity: &CreateCarDto,
    owner_id: EntityId,
    index: i32,              // insertion position; -1 = append
) -> Result<CarDto>
```

Creates an entity and attaches it to its owner.

#### create_multi

```rust
pub fn create_multi(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    entities: &[CreateCarDto],
    owner_id: EntityId,
    index: i32,
) -> Result<Vec<CarDto>>
```

Batch version of `create`.

#### get

```rust
pub fn get(
    db_context: &DbContext,
    id: &EntityId,
) -> Result<Option<CarDto>>
```

Fetches a single entity by ID. Returns `None` if not found.

#### get_multi

```rust
pub fn get_multi(
    db_context: &DbContext,
    ids: &[EntityId],
) -> Result<Vec<Option<CarDto>>>
```

Fetches multiple entities. Each entry is `None` if the corresponding ID was not found.

#### get_all

```rust
pub fn get_all(
    db_context: &DbContext,
) -> Result<Vec<CarDto>>
```

Returns all entities of this type. Use with caution on large tables.

#### update

```rust
pub fn update(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    entity: &UpdateCarDto,
) -> Result<CarDto>
```

Updates scalar fields only (no relationship changes). Accepts `UpdateCarDto` which contains `id` + scalar fields. Convert from `CarDto` via `.into()`:

```rust
let dto: CarDto = car_controller::get(&db, &id)?.unwrap();
let update_dto: UpdateCarDto = dto.into(); // drops relationship fields
```

#### update_multi

```rust
pub fn update_multi(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    entities: &[UpdateCarDto],
) -> Result<Vec<CarDto>>
```

Batch version of `update`.

#### update_with_relationships

```rust
pub fn update_with_relationships(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    entity: &CarDto,
) -> Result<CarDto>
```

Updates both scalar fields and relationship (junction table) data. Accepts the full `CarDto`. Use when you need to change relationship fields alongside scalar fields.

#### update_with_relationships_multi

```rust
pub fn update_with_relationships_multi(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    entities: &[CarDto],
) -> Result<Vec<CarDto>>
```

Batch version of `update_with_relationships`.

#### remove

```rust
pub fn remove(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    id: &EntityId,
) -> Result<()>
```

Deletes a single entity. Strong (owned) children are cascade-deleted.

#### remove_multi

```rust
pub fn remove_multi(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    ids: &[EntityId],
) -> Result<()>
```

Batch version of `remove`.

### Relationship Functions

Only available if the entity has forward relationships defined in the manifest.

#### get_relationship

```rust
pub fn get_relationship(
    db_context: &DbContext,
    id: &EntityId,
    field: &CarRelationshipField,
) -> Result<Vec<EntityId>>
```

Returns the IDs of related entities for a given relationship field.

#### get_relationship_many

```rust
pub fn get_relationship_many(
    db_context: &DbContext,
    ids: &[EntityId],
    field: &CarRelationshipField,
) -> Result<HashMap<EntityId, Vec<EntityId>>>
```

Batch version of `get_relationship`. Returns a map from each entity ID to its related IDs.

#### get_relationship_count

```rust
pub fn get_relationship_count(
    db_context: &DbContext,
    id: &EntityId,
    field: &CarRelationshipField,
) -> Result<usize>
```

Returns the number of related entities without loading them.

#### get_relationship_in_range

```rust
pub fn get_relationship_in_range(
    db_context: &DbContext,
    id: &EntityId,
    field: &CarRelationshipField,
    offset: usize,
    limit: usize,
) -> Result<Vec<EntityId>>
```

Returns a paginated slice of related entity IDs, starting at `offset` and returning at most `limit` entries.

#### set_relationship

```rust
pub fn set_relationship(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    dto: &CarRelationshipDto,
) -> Result<()>
```

Replaces the relationship. The `CarRelationshipDto` contains the entity ID, the relationship field, and the new list of related IDs.

#### move_relationship

```rust
pub fn move_relationship(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    // only if entity is undoable:
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    id: &EntityId,
    field: &CarRelationshipField,
    ids_to_move: &[EntityId],
    new_index: i32,
) -> Result<Vec<EntityId>>
```

Reorders specific related IDs within an ordered relationship. Takes the entity ID, the relationship field, the IDs to move, and the new index (`-1` means append at end). Returns the reordered list of related IDs.

### Usage Examples

```rust
use direct_access::car::controller;
use direct_access::car::dtos::{CreateCarDto, CarDto};
use common::types::EntityId;

// Read-only operations (no event_hub needed)
let car = controller::get(&db_context, &EntityId::new(1))?;
let all_cars = controller::get_all(&db_context)?;
let some_cars = controller::get_multi(&db_context, &[EntityId::new(1), EntityId::new(2)])?;

// Write operations (need event_hub for event emission)
let created = controller::create_orphan(&db_context, &event_hub, &create_dto)?;
let updated = controller::update(&db_context, &event_hub, &car_dto)?;
controller::remove(&db_context, &event_hub, &EntityId::new(1))?;

// Undoable write operations (need undo_redo_manager)
let created = controller::create_orphan(
    &db_context, &event_hub, &mut undo_redo_manager, Some(stack_id), &create_dto,
)?;

// Relationships
let passenger_ids = controller::get_relationship(
    &db_context, &EntityId::new(1), &CarRelationshipField::Passengers,
)?;
let many = controller::get_relationship_many(
    &db_context, &[EntityId::new(1), EntityId::new(2)], &CarRelationshipField::Passengers,
)?;
let count = controller::get_relationship_count(
    &db_context, &EntityId::new(1), &CarRelationshipField::Passengers,
)?;
let page = controller::get_relationship_in_range(
    &db_context, &EntityId::new(1), &CarRelationshipField::Passengers, 0, 10,
)?;
controller::set_relationship(
    &db_context, &event_hub, &relationship_dto,
)?;
let reordered = controller::move_relationship(
    &db_context, &event_hub, &EntityId::new(1),
    &CarRelationshipField::Passengers, &[EntityId::new(3), EntityId::new(5)], -1,
)?;
```

---

## Feature Controller

**File:** `crates/{feature}/src/{feature}_controller.rs`

Feature controllers are free functions for custom use cases grouped by feature. The controller is generated; **you implement the use case logic**.

### Generated Functions

For each use case defined in the manifest, the controller generates a function. The shape depends on the use case configuration:

#### Standard use case (with input DTO, with output DTO)

```rust
pub fn save(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    dto: &SaveDto,
) -> Result<SaveResultDto>
```

#### Standard use case (no input DTO, no output DTO)

```rust
pub fn initialize(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
) -> Result<()>
```

#### Long operation use case

Long operations run on a background thread with progress tracking:

```rust
// Start the operation, returns an operation ID
pub fn generate_code(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    long_operation_manager: &mut LongOperationManager,
    dto: &GenerateCodeDto,
) -> Result<String>

// Poll progress
pub fn get_generate_code_progress(
    long_operation_manager: &LongOperationManager,
    operation_id: &str,
) -> Option<OperationProgress>

// Get result
pub fn get_generate_code_result(
    long_operation_manager: &LongOperationManager,
    operation_id: &str,
) -> Result<Option<GenerateCodeResultDto>>
```

### Event Emission

After a successful use case execution, the use case emits a feature event via the UoW's `publish_*_event()` method. This is called from within the use case, after the transaction commits:

```rust
// In the use case's execute() method, after commit:
uow.publish_save_event(vec![], None);
```

The UoW implementation sends the event directly through the `EventHub`:

```rust
// Generated implementation in the UoW:
fn publish_save_event(&self, ids: Vec<EntityId>, data: Option<String>) {
    self.event_hub.send_event(Event {
        origin: Origin::HandlingManifest(Save),
        ids,
        data,
    });
}
```

The `ids` and `data` parameters let you attach context to the event (e.g., which entity IDs were affected). The generated scaffold passes `vec![]` and `None` by default — adapt these in your use case implementation.

You can subscribe to these events to trigger UI updates or other reactions.

### Usage Examples

```rust
use handling_manifest::controller;

// Standard use case
let result = controller::save(&db_context, &event_hub, &save_dto)?;

// Long operation
let op_id = controller::generate_code(
    &db_context, &event_hub, &mut long_op_manager, &dto,
)?;

// Check progress (e.g., in a loop or callback)
if let Some(progress) = controller::get_generate_code_progress(&long_op_manager, &op_id) {
    println!("{}% - {}", progress.percentage, progress.message.unwrap_or_default());
}

// Get result when done
if let Some(result) = controller::get_generate_code_result(&long_op_manager, &op_id)? {
    println!("Generated {} files", result.file_count);
}
```

---

## Custom Unit of Work (Proc Macros)

**Files you edit:**

- Use case + trait: `crates/{feature}/src/use_cases/{use_case}_uc.rs`
- Implementation: `crates/{feature}/src/units_of_work/{use_case}_uow.rs`

When Qleany generates a custom feature use case, it scaffolds a UoW trait and implementation with `TODO` comments. Your job is to **adapt the `#[macros::uow_action]` attributes** to expose only the entity operations your use case needs.

### How It Works

The `#[macros::uow_action]` proc macro decorates the `impl Trait for UoW` block. Each attribute generates a trait method (on the trait) or an implementation (on the impl block). The same set of attributes must appear on **both** the trait definition and the impl block.

The generated UoW implements either `CommandUnitOfWork` (read-write) or `QueryUnitOfWork` (read-only) for transaction management.

### Available Actions

**Read-write actions** (use with `CommandUnitOfWork`):

| Action                         | Generated method signature                                                        |
|--------------------------------|-----------------------------------------------------------------------------------|
| `CreateOrphan`                 | `fn create_orphan_name(&self, entity: &Name) -> Result<Name>`                    |
| `CreateOrphanMulti`            | `fn create_orphan_name_multi(&self, entities: &[Name]) -> Result<Vec<Name>>`     |
| `Create`                       | `fn create_name(&self, entity: &Name, owner_id: EntityId, index: i32) -> Result<Name>` |
| `CreateMulti`                  | `fn create_name_multi(&self, entities: &[Name], owner_id: EntityId, index: i32) -> Result<Vec<Name>>` |
| `Get`                          | `fn get_name(&self, id: &EntityId) -> Result<Option<Name>>`                      |
| `GetMulti`                     | `fn get_name_multi(&self, ids: &[EntityId]) -> Result<Vec<Option<Name>>>`        |
| `GetAll`                       | `fn get_all_name(&self) -> Result<Vec<Name>>`                                    |
| `Update`                       | `fn update_name(&self, entity: &Name) -> Result<Name>`                           |
| `UpdateMulti`                  | `fn update_name_multi(&self, entities: &[Name]) -> Result<Vec<Name>>`            |
| `UpdateWithRelationships`      | `fn update_name_with_relationships(&self, entity: &Name) -> Result<Name>`                           |
| `UpdateWithRelationshipsMulti` | `fn update_name_with_relationships_multi(&self, entities: &[Name]) -> Result<Vec<Name>>`            |
| `Remove`                       | `fn remove_name(&self, id: &EntityId) -> Result<()>`                             |
| `RemoveMulti`                  | `fn remove_name_multi(&self, ids: &[EntityId]) -> Result<()>`                    |
| `GetRelationship`              | `fn get_name_relationship(&self, id: &EntityId, field: &RF) -> Result<Vec<EntityId>>` |
| `GetRelationshipsFromRightIds` | `fn get_name_relationships_from_right_ids(&self, field: &RF, right_ids: &[EntityId]) -> Result<Vec<(EntityId, Vec<EntityId>)>>` |
| `SetRelationship`              | `fn set_name_relationship(&self, id: &EntityId, field: &RF, right_ids: &[EntityId]) -> Result<()>` |
| `SetRelationshipMulti`         | `fn set_name_relationship_multi(&self, field: &RF, relationships: Vec<(EntityId, Vec<EntityId>)>) -> Result<()>` |
| `MoveRelationship`             | `fn move_name_relationship(&self, id: &EntityId, field: &RF, ids_to_move: &[EntityId], new_index: i32) -> Result<Vec<EntityId>>` |
| `Snapshot`                     | `fn snapshot_name(&self, ids: &[EntityId]) -> Result<EntityTreeSnapshot>`        |
| `Restore`                      | `fn restore_name(&self, snap: &EntityTreeSnapshot) -> Result<()>`                |

**Read-only actions** (use with `QueryUnitOfWork`):

| Action                          | Generated method signature                                                        |
|---------------------------------|-----------------------------------------------------------------------------------|
| `GetRO`                         | `fn get_name(&self, id: &EntityId) -> Result<Option<Name>>`                      |
| `GetMultiRO`                    | `fn get_name_multi(&self, ids: &[EntityId]) -> Result<Vec<Option<Name>>>`        |
| `GetAllRO`                      | `fn get_all_name(&self) -> Result<Vec<Name>>`                                    |
| `GetRelationshipRO`             | `fn get_name_relationship(&self, id: &EntityId, field: &RF) -> Result<Vec<EntityId>>` |
| `GetRelationshipsFromRightIdsRO`| `fn get_name_relationships_from_right_ids(&self, field: &RF, right_ids: &[EntityId]) -> Result<Vec<(EntityId, Vec<EntityId>)>>` |

> Do not mix read-only (`*RO`) and write actions in the same unit of work.

These two tables are the complete action set. Any other name is a compile error
(`Unknown action`) pointing at the attribute. In particular, the relationship
`many` / `count` / `in_range` operations have no `uow_action` form: they live on
the entity repositories and are exposed by the generated direct-access entity
controllers, so a feature unit of work that needs one calls the repository
itself.

**For long operations**, add `thread_safe = true` to the **implementation** attributes (not the trait). This makes the generated code use `Mutex` instead of `RefCell` for thread safety.

### Full Example

Given a "Save" use case in the "HandlingManifest" feature that needs to read and update `Work` and `Setting` entities:

**Use case + trait** (`use_cases/save_uc.rs`):

The developer only have to adapt the `#[macros::uow_action]` attributes to expose only the entity operations your use case needs. Then, implement the use case logic.

```rust
use common::database::CommandUnitOfWork;

pub trait SaveUnitOfWorkFactoryTrait {
    fn create(&self) -> Box<dyn SaveUnitOfWorkTrait>;
}

// Adapt these macros to your needs:
#[macros::uow_action(entity = "Work", action = "Get")]
#[macros::uow_action(entity = "Work", action = "Update")]
#[macros::uow_action(entity = "Setting", action = "Get")]
#[macros::uow_action(entity = "Setting", action = "GetMulti")]
pub trait SaveUnitOfWorkTrait: CommandUnitOfWork {
    fn publish_save_event(&self, ids: Vec<EntityId>, data: Option<String>);
}

pub struct SaveUseCase {
    uow_factory: Box<dyn SaveUnitOfWorkFactoryTrait>,
}

impl SaveUseCase {
    pub fn new(uow_factory: Box<dyn SaveUnitOfWorkFactoryTrait>) -> Self {
        SaveUseCase { uow_factory }
    }

    pub fn execute(&mut self, dto: &SaveDto) -> Result<SaveResultDto> {
        let mut uow = self.uow_factory.create();
        uow.begin_transaction()?;

        // Use the UoW methods you declared:
        let work = uow.get_work(&dto.work_id)?
            .ok_or_else(|| anyhow!("Work not found"))?;

        let setting = uow.get_setting(&dto.setting_id)?
            .ok_or_else(|| anyhow!("Setting not found"))?;

        // ... your business logic ...

        let updated_work = uow.update_work(&work)?;

        uow.commit()?;

        // Emit the feature event (adapt ids/data to your needs):
        uow.publish_save_event(vec![], None);

        Ok(SaveResultDto { /* ... */ })
    }
}
```

**Implementation** (`units_of_work/save_uow.rs`):

The developer only have to adapt the `#[macros::uow_action]` attributes to expose only the entity operations your use case needs.

```rust
use crate::use_cases::save_uc::{SaveUnitOfWorkFactoryTrait, SaveUnitOfWorkTrait};
use common::database::CommandUnitOfWork;

pub struct SaveUnitOfWork {
    context: DbContext,
    transaction: Option<Transaction>,
    event_hub: Arc<EventHub>,
    event_buffer: RefCell<EventBuffer>,
}

impl SaveUnitOfWork {
    pub fn new(db_context: &DbContext, event_hub: &Arc<EventHub>) -> Self {
        SaveUnitOfWork {
            context: db_context.clone(),
            transaction: None,
            event_hub: event_hub.clone(),
            event_buffer: RefCell::new(EventBuffer::new()),
        }
    }
}

impl CommandUnitOfWork for SaveUnitOfWork {
    fn begin_transaction(&mut self) -> Result<()> {
        self.transaction = Some(Transaction::begin_write_transaction(&self.context)?);
        self.event_buffer.get_mut().begin_buffering();
        Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        self.transaction
            .take()
            .ok_or_else(|| anyhow!("No active transaction to commit"))?
            .commit()?;
        for event in self.event_buffer.get_mut().flush() {
            self.event_hub.send_event(event);
        }
        Ok(())
    }

    fn rollback(&mut self) -> Result<()> {
        self.transaction
            .take()
            .ok_or_else(|| anyhow!("No active transaction to rollback"))?
            .rollback()?;
        self.event_buffer.get_mut().discard();
        Ok(())
    }

    fn create_savepoint(&self) -> Result<types::Savepoint> {
        self.transaction
            .as_ref()
            .ok_or_else(|| anyhow!("No active transaction for savepoint"))?
            .create_savepoint()
    }

    fn restore_to_savepoint(&mut self, savepoint: types::Savepoint) -> Result<()> {
        let mut transaction = self.transaction
            .take()
            .ok_or_else(|| anyhow!("No active transaction to restore"))?;
        transaction.restore_to_savepoint(savepoint)?;
        self.event_buffer.get_mut().discard();
        self.event_hub.send_event(Event {
            origin: Origin::DirectAccess(DirectAccessEntity::All(AllEvent::Reset)),
            ids: vec![],
            data: None,
        });
        self.transaction = Some(transaction);
        Ok(())
    }
}

// Same macros as the trait, matching exactly:
#[macros::uow_action(entity = "Work", action = "Get")]
#[macros::uow_action(entity = "Work", action = "Update")]
#[macros::uow_action(entity = "Setting", action = "Get")]
#[macros::uow_action(entity = "Setting", action = "GetMulti")]
impl SaveUnitOfWorkTrait for SaveUnitOfWork {
    fn publish_save_event(&self, ids: Vec<EntityId>, data: Option<String>) {
        self.event_hub.send_event(Event {
            origin: Origin::HandlingManifest(Save),
            ids,
            data,
        });
    }
}

pub struct SaveUnitOfWorkFactory {
    context: DbContext,
    event_hub: Arc<EventHub>,
}

impl SaveUnitOfWorkFactory {
    pub fn new(db_context: &DbContext, event_hub: &Arc<EventHub>) -> Self {
        SaveUnitOfWorkFactory {
            context: db_context.clone(),
            event_hub: event_hub.clone(),
        }
    }
}

impl SaveUnitOfWorkFactoryTrait for SaveUnitOfWorkFactory {
    fn create(&self) -> Box<dyn SaveUnitOfWorkTrait> {
        Box::new(SaveUnitOfWork::new(&self.context, &self.event_hub))
    }
}
```

### Transaction Methods

These are available on every UoW via `CommandUnitOfWork` or `QueryUnitOfWork`.

**CommandUnitOfWork** (read-write):

| Method                                      | Purpose                                                            |
|---------------------------------------------|--------------------------------------------------------------------|
| `begin_transaction(&mut self)`              | Start a write transaction and begin event buffering                |
| `commit(&mut self)`                         | Commit; flush buffered events on success                           |
| `rollback(&mut self)`                       | Roll back the transaction and discard buffered events              |
| `create_savepoint(&self)`                   | Create a savepoint within the current transaction                  |
| `restore_to_savepoint(&mut self, sp)`       | Restore to a savepoint, discard events, emit Reset                 |

Do not use savepoint without understanding the implications: please read [Undo-Redo Architecture # savepoints](undo-redo-architecture.md#savepoints)

**QueryUnitOfWork** (read-only):

| Method                        | Purpose                              |
|-------------------------------|--------------------------------------|
| `begin_transaction(&self)`    | Start a read transaction             |
| `end_transaction(&self)`      | End the read transaction             |

The event buffering ensures that if a transaction fails, no events are emitted and the UI stays consistent.

### Undoable Custom Use Cases

If a custom use case is marked `undoable: true` in the manifest, the generated scaffold includes an `UndoRedoCommand` impl:

```rust
impl UndoRedoCommand for SaveUseCase {
    fn undo(&mut self) -> Result<()> {
        // TODO: implement undo logic
        unimplemented!();
    }

    fn redo(&mut self) -> Result<()> {
        // TODO: implement redo logic
        unimplemented!();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
```

You implement the `undo()` and `redo()` methods. The controller will call `undo_redo_manager.add_command_to_stack(Box::new(uc), stack_id)` after a successful `execute()`.
