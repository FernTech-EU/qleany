# Generated Infrastructure - Rust

This document details the infrastructure Qleany generates for Rust. It's a reference material — read it when you need to understand, extend, or debug the generated code, not as a getting-started guide.

## Rust Infrastructure

### HashMap Store Backend

In-memory HashMap storage behind `RwLock` for thread safety. Qleany generates a trait-based abstraction layer:

```rust
// Table trait (generated) — implemented by HashMap store
pub trait WorkspaceTable {
    fn create(&mut self, entity: &Workspace) -> Result<Workspace, Error>;
    fn create_multi(&mut self, entities: &[Workspace]) -> Result<Vec<Workspace>, Error>;
    fn get(&self, id: &EntityId) -> Result<Option<Workspace>, Error>;
    fn get_multi(&self, ids: &[EntityId]) -> Result<Vec<Option<Workspace>>, Error>;
    fn update(&mut self, entity: &Workspace) -> Result<Workspace, Error>;
    fn update_multi(&mut self, entities: &[Workspace]) -> Result<Vec<Workspace>, Error>;
    fn remove(&mut self, id: &EntityId) -> Result<(), Error>;
    fn remove_multi(&mut self, ids: &[EntityId]) -> Result<(), Error>;

    fn get_relationship(
        &self,
        id: &EntityId,
        field: &WorkspaceRelationshipField,
    ) -> Result<Vec<EntityId>, Error>;
    fn get_relationship_many(
        &self,
        ids: &[EntityId],
        field: &WorkspaceRelationshipField,
    ) -> Result<HashMap<EntityId, Vec<EntityId>>, Error>;
    fn get_relationship_count(
        &self,
        id: &EntityId,
        field: &WorkspaceRelationshipField,
    ) -> Result<usize, Error>;
    fn get_relationship_in_range(
        &self,
        id: &EntityId,
        field: &WorkspaceRelationshipField,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<EntityId>, Error>;
    fn get_relationships_from_right_ids(
        &self,
        field: &WorkspaceRelationshipField,
        right_ids: &[EntityId],
    ) -> Result<Vec<(EntityId, Vec<EntityId>)>, Error>;
    fn set_relationship_multi(
        &mut self,
        field: &WorkspaceRelationshipField,
        relationships: Vec<(EntityId, Vec<EntityId>)>,
    ) -> Result<(), Error>;
    fn set_relationship(
        &mut self,
        id: &EntityId,
        field: &WorkspaceRelationshipField,
        right_ids: &[EntityId],
    ) -> Result<(), Error>;
    fn move_relationship_ids(
        &mut self,
        id: &EntityId,
        field: &WorkspaceRelationshipField,
        ids_to_move: &[EntityId],
        new_index: i32,
    ) -> Result<Vec<EntityId>, Error>;
}

// Repository wraps table with event emission
pub struct WorkspaceRepository<'a> {
    table: Box<dyn WorkspaceTable + 'a>,
    transaction: &'a Transaction,
}
```

Read-only operations use a separate `WorkspaceTableRO` trait and `WorkspaceRepositoryRO` struct, enforcing immutability at the type level.

Table operations that violate one-to-one constraints return `RepositoryError::ConstraintViolation` instead of panicking. The `RepositoryError` enum also includes an `Other(anyhow::Error)` variant for wrapping generic errors. Repository factory functions return `Result`, propagating transaction errors to the caller.

### List Field Storage

Entity fields marked `is_list: true` in the manifest are stored as `Vec<T>` in the entity struct. Since entities are stored as plain Rust types in the HashMap store, list fields require no special storage treatment. Supported list types are `Vec<String>`, `Vec<i32>`, `Vec<u32>`, `Vec<f32>`, `Vec<bool>`, `Vec<Uuid>`, and `Vec<DateTime<Utc>>`.

### Long Operation Manager

Threaded execution for heavy tasks:

```rust
pub fn generate_rust_files(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    long_operation_manager: &mut LongOperationManager,
    dto: &GenerateRustFilesDto,
) -> Result<String> {
    let uow_context = GenerateRustFilesUnitOfWorkFactory::new(db_context, event_hub);
    let uc = GenerateRustFilesUseCase::new(Box::new(uow_context), dto);
    let operation_id = long_operation_manager.start_operation(uc);
    Ok(operation_id)
}

pub fn get_generate_rust_files_progress(
    long_operation_manager: &LongOperationManager,
    operation_id: &str,
) -> Option<OperationProgress> {
    long_operation_manager.get_operation_progress(operation_id)
}

pub fn get_generate_rust_files_result(
    long_operation_manager: &LongOperationManager,
    operation_id: &str,
) -> Result<Option<GenerateRustFilesReturnDto>> {
    // Get the operation result as a JSON string
    let result_json = long_operation_manager.get_operation_result(operation_id);

    // If there's no result, return None
    if result_json.is_none() {
        return Ok(None);
    }

    // Parse the JSON string into a GenerateRustFilesReturnDto
    let result_dto: GenerateRustFilesReturnDto = serde_json::from_str(&result_json.unwrap())?;

    Ok(Some(result_dto))
}
```

Features:
- Progress callbacks with percentage and message
- Cancellation support
- Result or error on completion
- Mutex poisoning recovery via `lock_or_recover` helper — all `Mutex` accesses in `LongOperationManager` and `OperationHandle` gracefully recover from poisoned locks instead of panicking

### Ephemeral Database Pattern

The internal database lives in memory, decoupled from user files:

1. **Load**: Transform file → internal database
2. **Work**: All operations against ephemeral database
3. **Save**: Transform internal database → file

This pattern separates the user's file format from internal data structures. Your `.myapp` file can be JSON, XML, SQLite, or any format. The internal database remains consistent.

The user must implement this pattern in dedicated custom use cases.

### Synchronous Undo/Redo Commands

Rust uses synchronous command execution (unlike C++/Qt's async controller layer). Each use case implements `UndoRedoCommand` and maintains its own undo/redo stacks using `VecDeque`:

```rust
pub struct UpdateWorkspaceUseCase {
    uow_factory: Box<dyn WorkspaceUnitOfWorkFactoryTrait>,
    undo_stack: VecDeque<Workspace>,
    redo_stack: VecDeque<Workspace>,
}

impl UndoRedoCommand for UpdateWorkspaceUseCase {
    fn undo(&mut self) -> Result<()> {
        if let Some(last_entity) = self.undo_stack.pop_back() {
            let mut uow = self.uow_factory.create();
            uow.begin_transaction()?;
            uow.update_workspace(&last_entity)?;
            uow.commit()?;
            self.redo_stack.push_back(last_entity);
        }
        Ok(())
    }

    fn redo(&mut self) -> Result<()> {
        if let Some(entity) = self.redo_stack.pop_back() {
            let mut uow = self.uow_factory.create();
            uow.begin_transaction()?;
            uow.update_workspace(&entity)?;
            uow.commit()?;
            self.undo_stack.push_back(entity);
        }
        Ok(())
    }
}
```

Controllers manage the `UndoRedoManager` and optional scoped stacks:

```rust
pub fn update(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    entity: &WorkspaceDto,
) -> Result<WorkspaceDto> {
    let uow_factory = WorkspaceUnitOfWorkFactory::new(&db_context, &event_hub);
    let mut uc = UpdateWorkspaceUseCase::new(Box::new(uow_factory));
    let result = uc.execute(entity)?;
    undo_redo_manager.add_command_to_stack(Box::new(uc), stack_id)?;
    Ok(result)
}
```

Unlike C++/Qt's async controller layer, Rust uses fully synchronous execution throughout, which works well for CLI where blocking is acceptable. I choose to avoid async/await complexity here.

After v1.0.34, use case templates classes in `common/direct_access/use_cases/` were introduced to simplify the code further.

```rust
pub fn update(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    entity: &WorkDto,
) -> Result<WorkDto> {
    let uow_factory = WorkWriteUoWFactory::new(db_context, event_hub);
    let entity_in: common::entities::Work = entity.into();
    let mut uc = use_cases::UpdateUseCase::new(uow_factory);
    let result = uc.execute(&entity_in)?;
    undo_redo_manager.add_command_to_stack(Box::new(uc), stack_id)?;
    Ok(result.into())
}

```


### Event Hub

Channel-based event dispatch using a unified `Event` struct. The `start_event_loop` function returns a `thread::JoinHandle<()>` and blocks on a `flume::Selector` that waits on either the event channel or a shutdown receiver — zero idle CPU, no polling. The thread exits within microseconds of `AppContext::shutdown()` being called (the shared shutdown `Sender` is taken out of `Mutex<Option<…>>` and dropped, which makes every cloned `shutdown_rx` see `Disconnected`):

```rust
// Event structure (generated)
pub struct Event {
    pub origin: Origin,
    pub ids: Vec<EntityId>,
    pub data: Option<String>,
}

pub enum Origin {
    DirectAccess(DirectAccessEntity),
    Feature(FeatureEntity),
}

pub enum DirectAccessEntity {
    Workspace(EntityEvent),
    Entity(EntityEvent),
    // ... other entities
}

pub enum EntityEvent {
    Created,
    Updated,
    Removed,
}
...

// Publishing (from the repositories)
event_hub.send_event(Event {
    origin: Origin::DirectAccess(DirectAccessEntity::Workspace(EntityEvent::Updated)),
    ids: vec![entity.id.clone()],
    data: None,
});
```

---

### Repository

Both languages generate repositories with batch-capable interfaces:

| Method                                                                        | Purpose                                            |
|-------------------------------------------------------------------------------|----------------------------------------------------|
| `create(entity, owner_id, index)` / `create_multi(entities, owner_id, index)` | Insert new entities and attach it to owner         |
| `create_orphan(entity)` / `create_orphan_multi(entities)`                     | Insert new entities without owner                  |
| `get(id)` / `get_multi(ids)`                                                  | Fetch entities                                     |
| `get_all()`                                                                   | Fetch all entities                                 |
| `update(entity)` / `update_multi(entities)`                                   | Update scalar fields only                          |
| `update_with_relationships(entity)` / `update_with_relationships_multi(entities)` | Update scalar fields and relationships          |
| `remove(id)` / `remove_multi(ids)`                                            | remove entities (cascade for strong relationships) |


Relationship-specific methods:

| Method                                                | Purpose                                   |
|-------------------------------------------------------|-------------------------------------------|
| `get_relationship(id, field)`                         | Get related IDs for one entity            |
| `get_relationship_many(ids, field)`                   | Get related IDs for multiple entities     |
| `get_relationship_count(id, field)`                   | Count related entities without loading    |
| `get_relationship_in_range(id, field, offset, limit)` | Paginated slice of related IDs            |
| `get_relationships_from_right_ids(field, ids)`        | Reverse lookup                            |
| `set_relationship(id, field, ids)`                    | Set relationship for one entity           |
| `set_relationship_multi(field, relationships)`        | Batch relationship updates                |
| `move_relationship_ids(id, field, ids_to_move, new_index)` | Reorder IDs within an ordered relationship |

### Unit of Work

In Rust, the units of work are helped by macros to generate all the boilerplate for transaction management and repository access. This can be a debatable design choice, since all is already generated by Qleany. The reality is: not all can be generated. The user (developer) has the responsibility to adapt the units of work for each custom use case. The macros are here to ease this task.

> The user is to adapt the macros in custom use cases.

Each use case receives a unit of work factory which handles the unit of work creation that allow transaction-scoped operations:

```rust
// In the controller, we create the use case with a factory for the unit of work

pub fn create(
    db_context: &DbContext,
    event_hub: &Arc<EventHub>,
    undo_redo_manager: &mut UndoRedoManager,
    stack_id: Option<u64>,
    entity: &CreateWorkspaceDto,
) -> Result<WorkspaceDto> {
    let uow_factory = WorkspaceUnitOfWorkFactory::new(db_context, event_hub);
    let mut uc = CreateWorkspaceUseCase::new(Box::new(uow_factory));
    let result = uc.execute(entity.clone())?;
    undo_redo_manager.add_command_to_stack(Box::new(uc), stack_id)?;
    Ok(result)
}

// In the unit of work, you see a bit of macro magic to generate all the boilerplate:

#[macros::uow_action(entity = "Workspace", action = "Create")]
#[macros::uow_action(entity = "Workspace", action = "CreateMulti")]
#[macros::uow_action(entity = "Workspace", action = "Get")]
#[macros::uow_action(entity = "Workspace", action = "GetMulti")]
#[macros::uow_action(entity = "Workspace", action = "Update")]
#[macros::uow_action(entity = "Workspace", action = "UpdateMulti")]
#[macros::uow_action(entity = "Workspace", action = "Remove")]
#[macros::uow_action(entity = "Workspace", action = "RemoveMulti")]
#[macros::uow_action(entity = "Workspace", action = "GetRelationship")]
#[macros::uow_action(entity = "Workspace", action = "GetRelationshipsFromRightIds")]
#[macros::uow_action(entity = "Workspace", action = "SetRelationship")]
#[macros::uow_action(entity = "Workspace", action = "SetRelationshipMulti")]
#[macros::uow_action(entity = "Workspace", action = "MoveRelationship")]
impl WorkspaceUnitOfWorkTrait for WorkspaceUnitOfWork {}
```

The macro accepts a fixed set of action names. See
[Available Actions](api-reference-rust.md#available-actions) for the full list.
Any other name is a compile error (`Unknown action`).

### DTO Mapping

DTOs are generated for boundary crossings between UI and use cases. DTO←→Entity conversion is done in the use cases:

```
|----------------DTO-------------------|------------------Entity----------|
UI ←→ Controller ←→ CreateCarDto ←→ UseCase ←→ Car (Entity) ←→ Repository
```

The separation ensures:
- Controllers don't expose entity internals
- You control what data flows in/out of each layer

---

## File Organization


```
Cargo.toml
crates/
├── cli/
│   ├── src/
│   │   ├── main.rs    
│   └── Cargo.toml
├── common/
│   ├── src/
│   │   ├── entities.rs             # Generated entities
│   │   ├── database.rs
│   │   ├── database/
│   │   │   ├── db_context.rs
│   │   │   ├── db_helpers.rs
│   │   │   └── transactions.rs
│   │   ├── direct_access.rs
│   │   ├── direct_access/         # Holds the repository and table implementations for each entity
│   │   │   ├── use_cases/         # Generics for direct access use cases
│   │   │   ├── car.rs
│   │   │   ├── car/
│   │   │   │   ├── car_repository.rs
│   │   │   │   └── car_table.rs
│   │   │   ├── customer.rs
│   │   │   ├── customer/
│   │   │   │   ├── customer_repository.rs
│   │   │   │   └── customer_table.rs
│   │   │   ├── sale.rs
│   │   │   ├── sale/
│   │   │   │   ├── sale_repository.rs
│   │   │   │   └── sale_table.rs
│   │   │   ├── root.rs
│   │   │   ├── root/
│   │   │   │   ├── root_repository.rs
│   │   │   │   └── root_table.rs
│   │   │   ├── repository_factory.rs
│   │   │   └── setup.rs
│   │   ├── event.rs             # event system for reactive updates
│   │   ├── lib.rs
│   │   ├── long_operation.rs    # infrastructure for long operations
│   │   ├── types.rs         
│   │   └── undo_redo.rs        # undo/redo infrastructure
│   └── Cargo.toml
├── frontend/                    # entry point for UI or CLI to interact with entities and features
│   ├── src/
│   │   ├── lib.rs
│   │   ├── event_hub_client.rs
│   │   ├── app_context.rs
│   │   ├── commands.rs
│   │   └── commands/           
│   │       ├── undo_redo_commands.rs
│   │       ├── car_commands.rs
│   │       ├── customer_commands.rs
│   │       ├── sale_commands.rs
│   │       └── root_commands.rs
│   └── Cargo.toml
├── direct_access/               # group feature CRUD operations
│   ├── src/
│   │   ├── car.rs
│   │   ├── car/
│   │   │   ├── car_controller.rs   # Entry point. Exposes CRUD operations to UI or CLI
│   │   │   ├── dtos.rs
│   │   │   └── units_of_work.rs
│   │   ├── customer/
│   │   │   └── ...
│   │   ├── sale.rs
│   │   ├── sale/
│   │   │   └── ...
│   │   ├── root.rs
│   │   ├── root/
│   │   │   └── ...
│   │   └── lib.rs
│   └── Cargo.toml
├── inventory_management/           # custom feature ( = group of use cases)
│   ├── src/
│   │   ├── inventory_management_controller.rs
│   │   ├── dtos.rs
│   │   ├── units_of_work.rs
│   │   ├── units_of_work/          # ← adapt the unit of works with macros here
│   │   │   └── ...
│   │   ├── use_cases.rs
│   │   ├── use_cases/              # ← You implement the business logic here
│   │   │   └── ...
│   │   └── lib.rs
│   └── Cargo.toml
└── teksilo_ui/                     # when ui.rust_teksilo is set
    ├── src/
    │   ├── main.rs
    │   ├── lib.rs                  # builds and runs the app
    │   ├── event_source.rs         # backend events → the UI thread
    │   ├── session.rs              # every single and model, wired
    │   ├── undo_redo.rs
    │   ├── app.rs                  # ← the demo window; replace with your UI
    │   ├── singles.rs
    │   ├── singles/
    │   │   └── single_car.rs       # one per single_model entity
    │   ├── models.rs
    │   └── models/
    │       ├── coalesced_reload.rs
    │       └── root_cars_list_model.rs  # one per list_model relationship
    ├── tests/
    │   └── demo_headless.rs        # drives the demo window with no display
    └── Cargo.toml

```

### The Teksilo UI crate

Generated when `ui.rust_teksilo` is set. It talks only to the `frontend` crate,
so it duplicates no backend access.

**Singles** (`singles/single_{entity}.rs`) — one per entity marked
`single_model: true`. A single holds one entity by id and exposes each scalar
field as a two-way `Signal`, alongside `loading_status`, `error_message` and
`dirty`. It refreshes itself when that entity's `Updated` event arrives and
writes edits back through `save()`, which is a read-modify-write so `created_at`
and uuids are carried rather than re-derived.

**List models** (`models/{entity}_{field}_list_model.rs`) — one per relationship
field marked `list_model: true`. Rows are read through the *owner's*
relationship, never `get_all_*`: the store is shared, so `get_all` would merge
every other owner's children and lose the order the user chose.

**Mocks** — each single and list model carries a second implementation behind
the crate's `mocks` feature, selected by `#[cfg]` with an identical public
surface, so no `#[cfg]` leaks into consuming code. `cargo run --features mocks`
renders the UI against fabricated data with no backend. Building both feature
modes is not enough to keep the two arms in step: the mocks arm is real
alternate logic, it carries its own generated tests, and clippy only ever lints
the `#[cfg]` arm it actually compiled, so the default pass says nothing about
this one. Build, test *and* lint it in CI:

```sh
cargo build  -p <app>-teksilo-ui --all-targets --features mocks
cargo test   -p <app>-teksilo-ui --features mocks
cargo clippy -p <app>-teksilo-ui --all-targets --features mocks --no-deps -- -D warnings
```

Scope with `-p`: the UI crate is the only one declaring the feature, so never
combine `--features mocks` with `--workspace`. `--no-deps` keeps `-D warnings`
off the generated feature crates, whose use cases are `unimplemented!()`
scaffolds by design.

**Wiring** — a subscription made from a widget's `build` lasts exactly one build
cycle, so `Session::wire_all(ctx)` must be called from `build` on *every* build.
Wiring once leaves the UI silently deaf after the first rebuild.

**The headless test** (`tests/demo_headless.rs`) — drives the demo window's own
handles with no window and no display: it bootstraps a session, adds rows
through the list model using `app::FormState::to_create_dto` (the Add button's
own conversion), points the single at a selection, and evaluates the very
`selected_id.zip(&single.field()).map(..)` expression the detail panel binds.
It exists because compiling this crate proves nothing about it *working*: three
defects that made the demo unusable — an owner signal never seeded, a required
reference left at id 0 so only the first Add succeeded, and a detail panel that
read `.get()` during `build` — all shipped through a green `cargo check`. It is
`Infrastructure`, so it is regenerated freely; replace `app.rs` with your real
UI and this test goes with it.