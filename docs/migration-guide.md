# Migration Guide

This document covers breaking changes between manifest schema versions and how to upgrade.

---

## v1.7.3 to v1.7.4 — Event-hub shutdown via channel wakeup

**Qleany version**: v1.7.4

### What changed

`AppContext::quit_signal: Arc<AtomicBool>` is removed. Background event-hub threads (`EventHub::start_event_loop`, `EventHubClient::start`, mobile bridge `start_event_dispatch`) no longer poll a flag every 100–500 ms. They now block on a `flume::Selector` that waits on either the event channel or a shutdown receiver, so idle wake-ups drop to 0 instead of 5–10 per thread per second.

`AppContext` gains two new fields:

- `pub shutdown_rx: Receiver<()>` — handed to every spawned event-loop thread.
- `shutdown_tx: Arc<Mutex<Option<Sender<()>>>>` (private) — the only live `Sender` clone. `AppContext::shutdown()` `.take()`s it, which makes every cloned receiver see `Disconnected` within microseconds.

`EventHub::start_event_loop`, `EventHubClient::start`, and `start_event_dispatch` now take `Receiver<()>` instead of `Arc<AtomicBool>`.

### Behavioral changes

- **Idle CPU**: ~0 wake-ups per event-loop thread (previously 5/s for `EventHubClient`, 10/s for `EventHub::start_event_loop`, 2/s for the mobile dispatcher). On a host app with many widgets each owning their own backend, savings scale linearly.
- **Shutdown latency**: microseconds (was up to one polling interval).
- **`AppContext::shutdown()` is still `&self` and idempotent** — second call is a no-op because the sender has already been taken.

### How to upgrade

1. Regenerate the affected files: `crates/common/src/event.rs`, `crates/frontend/src/app_context.rs`, `crates/frontend/src/event_hub_client.rs`, `crates/slint_ui/src/main.rs` (if using Slint), and the mobile bridge `events.rs` / `backend.rs` (if using the mobile bridge).
2. **Update hand-written callers**: any code calling `event_hub_client.start(ctx.quit_signal.clone())` must become `event_hub_client.start(ctx.shutdown_rx.clone())`. Same for `start_event_loop` and `start_event_dispatch`.
3. **Hand-written code that read `ctx.quit_signal` directly** (e.g. to coordinate other shutdown work) must switch to a different mechanism — the field no longer exists. The shutdown channel is single-purpose; if you need a broader shutdown bus, either subscribe to `shutdown_rx` from your own thread (you'll see `Disconnected` when shutdown fires) or layer your own signal alongside.

---

## v1.6.3 to v1.7.0 — redb replaced by in-memory HashMap store

**Qleany version**: v1.7.0

### What changed

The Rust storage backend has been replaced. The `redb` embedded database and `postcard` serialization are gone. The new backend is an in-memory store using `im::HashMap` (persistent data structure with structural sharing), giving O(1) snapshots for undo/redo.

### Behavioral changes

- **Rollback-safe transactions**: Write transactions now automatically create a savepoint on `begin_transaction()`. If the transaction is dropped without `commit()` (e.g., on error), `Drop` restores the savepoint — undoing all partial mutations. Previously with redb, this was handled by redb's own transaction abort on drop.
- **Faster snapshots**: Undo/redo snapshots are O(1) instead of O(n) deep clones, thanks to `im::HashMap` structural sharing.
- **No serialization**: Entities are stored as plain Rust types. No postcard encoding/decoding overhead.

### How to upgrade

1. **Regenerate affected files**: Use the Qleany UI or CLI to regenerate the storage-related files: `Cargo.toml` (common crate), `database.rs`, `db_context.rs`, `hashmap_store.rs`, `transactions.rs`, `snapshot.rs`, `error.rs`, `repository_factory.rs`, `setup.rs`, entity table files (`*_table.rs`), entity repository files (`*_repository.rs`), and test files (`transaction_tests.rs`). The old `redb_tests.rs` and `snapshot_tests.rs` can be deleted — their tests have been merged into `transaction_tests.rs`.
2. **Update your workspace `Cargo.toml`**: Remove `redb` and `postcard` from `[workspace.dependencies]` if present. The `im` crate is added automatically by the generated common `Cargo.toml`.
3. **Custom feature use cases**: No changes needed — the UoW trait interface (`begin_transaction`, `commit`, `rollback`, `create_savepoint`, `restore_to_savepoint`) is unchanged. Your use case code works as before, now with automatic rollback on error.

---

## v1.6.0 to v1.6.1 — Crate renaming and publishing metadata

**Qleany version**: v1.6.1

### What changed

No manifest schema changes. These are generated `Cargo.toml` and template improvements.

### Workspace publishing metadata

Generated `Cargo.toml` files now include workspace-level metadata and enable publishing:

```toml
# Before (v1.6.0)
[package]
name = "my-app-common"
version.workspace = true
publish = false

# After (v1.6.1)
[package]
name = "my-app-common"
description = "Shared infrastructure for My App"
authors.workspace = true
documentation.workspace = true
keywords.workspace = true
categories.workspace = true
version.workspace = true
readme = "../../README.md"
publish = true
```

The workspace root `Cargo.toml` now requires a `[workspace.package]` section with shared metadata (authors, documentation, keywords, categories). Generated crates inherit from it.

### Prompt templates

Prompt templates have been slimmed down — they now point to source files for DTOs and entities instead of inlining full definitions.

### How to upgrade

1. Regenerate all `Cargo.toml` files (infrastructure and feature crates) to pick up the new metadata fields.
2. If publishing to crates.io, ensure your workspace root has a `[workspace.package]` section with proper metadata (homepage, repository, license, etc.).
3. No code changes required — this is purely a packaging/metadata update.

---

## v1.5.3 to v1.6.0 — Event publishing moves to UoW layer

**Qleany version**: v1.5.4 through v1.6.0

### What changed

No manifest schema changes. The major change is that event publishing responsibility has moved from controllers into the Unit of Work layer. All UoW factories now receive `event_hub`, and each use case publishes its own event after commit.

### Event publishing (v1.6.0)

- **UoW factory constructor**: Both read-only and read-write use cases now take `(db_context, event_hub)`. Previously, read-only use cases took only `(db_context)`.

  ```rust
  // Before (v1.5.3) — read-only use cases
  let uow_context = MyUseCaseUnitOfWorkFactory::new(db_context);

  // After (v1.6.0) — all use cases
  let uow_context = MyUseCaseUnitOfWorkFactory::new(db_context, event_hub);
  ```

- **UoW trait**: All feature use case traits now require a `publish_*_event` method:

  ```rust
  pub trait MyUseCaseUnitOfWorkTrait: QueryUnitOfWork + Send + Sync {
      fn publish_my_use_case_event(&self, ids: Vec<EntityId>, data: Option<String>);
  }
  ```

- **Event publishing in use cases**: The use case now calls `uow.publish_*_event()` after commit/end_transaction, instead of the controller calling `event_hub.send_event()` directly:

  ```rust
  // In execute():
  uow.commit()?; // or uow.end_transaction()? for read-only
  uow.publish_my_use_case_event(vec![], None);
  ```

- **Controllers simplified**: Controllers no longer contain event-sending code. The `event_hub.send_event(Event { origin, ids, data })` block has been removed from controller templates.

- **UoW structs**: All UoW structs (including read-only) now carry `event_hub: Arc<EventHub>`.

### Float type support (v1.5.6)

- Generated entities and DTOs now exclude `Eq` from derive traits when float fields are present. This is automatic on regeneration.

### Entity test improvements (v1.5.5)

- Generated entity controller tests now include ownership chain validation. No API changes.

### How to upgrade

1. **Regenerate infrastructure files** (nature: Infra) to pick up the new controller and UoW templates.
2. If you have **custom feature use cases**, update:
   - Change `UnitOfWorkFactory::new(db_context)` to `UnitOfWorkFactory::new(db_context, event_hub)` for read-only use cases.
   - Add the `publish_{use_case}_event` method to your UoW trait and implementation:

     ```rust
     // In the trait:
     fn publish_my_use_case_event(&self, ids: Vec<EntityId>, data: Option<String>);

     // In the implementation:
     fn publish_my_use_case_event(&self, ids: Vec<EntityId>, data: Option<String>) {
         self.event_hub.send_event(Event {
             origin: Origin::MyFeature(MyUseCase),
             ids,
             data,
         });
     }
     ```

   - Move event publishing from your controller into the use case's `execute()` method.
   - Add `event_hub: Arc<EventHub>` to your UoW struct and accept it in the factory constructor.

---

## v1.5.0 to v1.5.3 — Error handling and robustness improvements

**Qleany version**: v1.5.1 through v1.5.3

### What changed

No manifest schema changes. These are generated code improvements that affect regenerated projects.

### Error handling (v1.5.1–v1.5.2)

- **Transactions**: `get_read_transaction()` and `get_write_transaction()` now return `Result` instead of panicking on wrong transaction type or consumed state. `commit()`, `rollback()`, `create_savepoint()`, and `restore_to_savepoint()` return descriptive errors instead of panicking on double-commit or missing `begin_transaction()`.
- **Repository factory**: Factory functions return `Result`, so all unit of work call sites must use `?` to propagate errors. If you have custom UoW implementations, update repository creation calls from `repository_factory::write::create_*_repository(transaction)` to `repository_factory::write::create_*_repository(transaction)?`.
- **Undo/redo**: `begin_composite()` now returns `Result<()>` instead of panicking on mismatched stack IDs. `cancel_composite()` now undoes any already-executed sub-commands before clearing state. Failed `undo()` and `redo()` operations re-push the command to its original stack instead of dropping it.
- **Table constraints**: One-to-one constraint violations return `RepositoryError::ConstraintViolation` instead of panicking.
- **New error variants**: `RepositoryError` gains `ConstraintViolation(String)` and `Other(anyhow::Error)`.
- **Proc macros**: `#[macros::uow_action]` with missing arguments now emits a compile error instead of panicking.
- **DTO enums**: Enum imports in generated DTO files are now `pub use` instead of `use`, making them accessible to external crates.

### Event loop and long operations (v1.5.3)

- **Event loop**: `start_event_loop` now returns `thread::JoinHandle<()>` and uses `recv_timeout(100ms)` so the stop signal is checked even when no events arrive. This fixes unresponsive shutdown. (Superseded in v1.7.4 — the `recv_timeout` poll is gone, replaced by a true blocking `flume::Selector` wakeup. See the v1.7.3 → v1.7.4 entry.)
- **Long operations**: A `lock_or_recover` helper handles mutex poisoning gracefully in `LongOperationManager` and `OperationHandle`, replacing all `.lock().unwrap()` calls.

### Mobile bridge (v1.5.1)

- **Feature method naming**: Feature use case methods now include the feature prefix (e.g., `handling_manifest_save()` instead of `save()`). Swift/Kotlin async wrappers follow suit (`handlingManifestSaveAsync()`).
- **Cross-module types**: A `mobile_types` module re-exports entity types across command modules.
- **Entity conversions**: `From<Entity> for MobileEntityDto` and reverse conversions are now generated.

### How to upgrade

1. Regenerate your project's infrastructure files (nature: Infra) to pick up the new error handling patterns.
2. If you have **custom UoW implementations** (feature use cases), update:
   - Replace `.take().unwrap()` on transaction `Option`s with `.take().ok_or_else(|| anyhow!("No active transaction"))?`
   - Add `?` after `repository_factory::write::create_*_repository(...)` and `repository_factory::read::create_*_repository(...)` calls
   - Update `begin_composite()` call sites to handle the new `Result<()>` return type
3. If you use the **mobile bridge**, update Swift/Kotlin call sites to use the new feature-prefixed method names.

### Cargo workspace dependencies

Generated `Cargo.toml` templates now use workspace-level dependency declarations. Regenerate your Cargo files to pick up this change.

---

## Schema v4 to v5 — `is_list` for entity fields

**Qleany version**: v1.4.0

### What changed

Entity fields now support `is_list: true`, the same way DTO fields already did. This allows declaring list/array fields of primitive types (string, integer, uinteger, float, boolean, uuid, datetime) directly on entities.

### Constraints

- `is_list` cannot be used with `entity` or `enum` field types.
- `is_list` and `optional` are mutually exclusive on the same field.

### Example

```yaml
entities:
  - name: Project
    inherits_from: EntityBase
    fields:
      - name: title
        type: string
      - name: labels
        type: string
        is_list: true
      - name: scores
        type: float
        is_list: true
```

### Automatic migration

Qleany auto-migrates v2+ manifests on load. When you open a v4 manifest, the migrator bumps the version to 5 before validation. No manual editing is required.

### Manual migration

Change the schema version:

```yaml
schema:
  version: 5    # was 4
```

No other manifest changes are needed — `is_list` defaults to `false` when omitted.

### Storage

- **Rust**: list fields are stored as `Vec<T>` in the entity struct, held as plain Rust types in the in-memory HashMap store.
- **C++/Qt**: list fields are stored as `QList<T>` in the entity struct, serialized as JSON arrays in SQLite TEXT columns.

---

## Schema v3 to v4

**Qleany version**: v1.0.31

### What changed

The `validator` use case property has been removed.

### Reasons for the change

Validation is the responsibility of the developer.

### Automatic migration

Qleany auto-migrates v2+ manifests on load. When you open a v3 manifest, the migrator strips all `validator` fields and bumps the version to 4 before validation. No manual editing is required to load an old manifest.

If you save the manifest afterwards (from the UI), the file is written as v4.

From the CLI, it's the same: if you run `qleany generate` on a v3 manifest, it will be auto-migrated to v4 before generation. To only migrate the manifest, use `qleany migrate` instead.


### Manual migration

If you prefer to update the file yourself:

1. Change the schema version:

```yaml
schema:
  version: 4    # was 3
```

2. Remove every `validator:` line from your entities:

```diff
 feature:
   - name : my_feature
     use_cases:
       - name: my_use_case
-        validator: true
```

No other manifest changes are needed.

### Behavioral differences

None

### Code generation templates

Never used.

---

## Schema v2 to v3

**Qleany version**: v1.0.29

### What changed

The `allow_direct_access` entity property has been removed. Every entity that isn't heritage-only now always gets its `direct_access/` files generated.

### Reasons for the change

The direct_access/ is an internal API. `allow_direct_access: true` skipped generation of the files for an entity. Yet, this entity could have needed to offer a list_model or a single model, which wouldnt be possible without direct_access/ files.
So, from now on, all non-heritage entities always get their `direct_access/` files generated. At compilation time, unused C++ functions (static libraries) are stripped from the binary. Same for Rust. In shared C++ libraries, C++ unused functions are compiled, yet the overweight is negligible.

### Automatic migration

Qleany auto-migrates v2+ manifests on load. When you open a v2 manifest, the migrator strips all `allow_direct_access` fields and bumps the version to 3 before validation. No manual editing is required to load an old manifest.

If you save the manifest afterwards (from the UI), the file is written as v3.

From the CLI, it's the same: if you run `qleany generate` on a v2 manifest, it will be auto-migrated to v3 before generation. To only migrate the manifest, use `qleany migrate` instead.

### Manual migration

If you prefer to update the file yourself:

1. Change the schema version:

```yaml
schema:
  version: 3    # was 2
```

2. Remove every `allow_direct_access:` line from your entities:

```diff
 entities:
   - name: EntityBase
     only_for_heritage: true
-    allow_direct_access: false
     fields:
       ...

   - name: Car
     inherits_from: EntityBase
-    allow_direct_access: true
     fields:
       ...
```

That's it. No other manifest changes are needed.

### Behavioral differences

| Before (v2) | After (v3) |
|---|---|
| `allow_direct_access: false` hid an entity from `direct_access/` generation | Use `only_for_heritage: true` instead (which also skips generation) |
| `allow_direct_access: true` (the default) generated files | All non-heritage entities always generate files |

If you had entities with `allow_direct_access: false` that were **not** `only_for_heritage: true`, those entities will now generate `direct_access/` files. If you don't want that, mark them `only_for_heritage: true`.

### Code generation templates

Tera templates that referenced `ent.inner.allow_direct_access` now use `not ent.inner.only_for_heritage`. If you've written custom templates that check this field, update them accordingly.
