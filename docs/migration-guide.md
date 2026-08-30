# Migration Guide

This document covers breaking changes between manifest schema versions and how to upgrade.

---

## v1.10.0 to v1.11.0 — Undo entries gain identity: sequences, labels, an untracked stack

**Qleany version**: v1.11.0

### What changed

Template-only release: no manifest schema change. The undo/redo manager grows the four
things an application needs before it can offer *one* Undo command over the whole app rather
than a per-feature button, and every one of them was a real defect somewhere first.

**1. Entries are identified.** `UndoRedoManager` now stores an `UndoEntry { command, seq }`
rather than a bare `Box<dyn UndoRedoCommand>`, and mints a process-unique sequence for each
push. Read the one just created with `last_pushed_seq()`, and hand it back later to:

```rust
pub fn undo_if_head(&mut self, stack_id: Option<u64>, seq: u64) -> Result<UndoStatus>
pub enum UndoStatus { Undone, Superseded, Empty }
```

This exists because *"Undo"* on a toast is offered **for one operation**, and the stack keeps
moving underneath it — an autosave, a second window, a background job. Plain `undo()` pops
whatever is on top, so the button silently reverses the wrong thing while the user believes
they took back what the toast named. `undo_if_head` refuses instead.

Also new, alongside: `head_seq(stack_id)` and `get_redo_stack_size(stack_id)`.

**2. Commands can name themselves.**

```rust
pub struct UndoLabel { pub subject: &'static str, pub action: &'static str }

trait UndoRedoCommand {
    fn label(&self) -> Option<UndoLabel> { None }   // defaulted — existing impls compile
}

// on the manager
pub fn undo_label(&self, stack_id: Option<u64>) -> Option<UndoLabel>
pub fn redo_label(&self, stack_id: Option<u64>) -> Option<UndoLabel>
```

Deliberately a **machine key**, not a sentence: generated code knows nothing about locales
and must not carry an application's wording. The application maps `("binder_item", "remove")`
to its own translation. A group names itself through
`begin_composite_labeled(stack_id, label)` or `CompositeCommand::labeled` — a composite's
constituent labels describe its parts, so a menu built from them reads *"Undo update"* for
what the user experienced as one act.

**3. `UNTRACKED_STACK_ID` — a write that must happen and must not be history.**

```rust
pub const UNTRACKED_STACK_ID: u64 = u64::MAX;
```

The generated commands always push; `stack_id: None` resolves to the global stack `0` rather
than opting out. So the only way to keep a mirrored buffer, a cache line or an index rebuild
out of the user's history was to push it somewhere and clear that somewhere afterwards —
which nobody remembers to do, and which is why stack 0 grows forever in most consuming
projects. Passing this id drops the command at the door. `begin_composite` on it is refused
with an error: a group whose commands are dropped could never be undone.

**4. A bound, and a way to break a merge.**

```rust
pub fn set_undo_limit(&mut self, limit: Option<usize>)  // None = unbounded, the old behaviour
pub fn undo_limit(&self) -> Option<usize>
pub fn seal_head(&mut self, stack_id: Option<u64>)
```

Undo history was unbounded, and every entry can pin an `EntityTreeSnapshot`. `seal_head`
closes the top entry to merging: merging decides on the *shape* of two commands — adjacent,
close in time — and cannot see that something unrelated happened between them, so a caller
that knows a dividing line was crossed says so.

**5. Events say which stack, and announce growth.**

`UndoRedoEvent` gains `StackChanged` — a command pushed, a command merged, a stack cleared —
and **every** variant now carries the stack id in `Event::data`. Before this, a UI could
learn that history had been *consumed* but never that it had been *created*, so an Undo menu
row had nothing to react to and had to be polled; and a process holding one stack per open
document could not tell whose history had moved. `FlatEventKind` gains the matching
`UndoStackChanged`, which `is_mutation` reports as `false` like its siblings.

**6. Four fixes in the same code, each of which was a live bug.**

- **`end_composite` now releases the composite's target stack.** `begin_composite` refuses a
  stack other than the one already open, and only `cancel_composite` ever cleared
  `composite_stack_id` — so once a group had *ended* normally on stack A, every later
  `begin_composite` on any other stack returned
  `"Cannot begin a composite on a different stack while another composite is in progress"`,
  for the life of the manager. In a process holding one stack per open document, the first
  document to group anything was the only one that ever could again. `clear_all_stacks`
  cleared the label and the in-progress group but had the same omission, and now clears it too.
- **`last_pushed_seq()` reports `None` on every path that records nothing** — a command folded
  into an open composite, a group that closed empty, a group whose stack had been removed, a
  cancelled group, and a push to a missing stack. It used to keep whatever it last held, which
  is worse than wrong: the number it hands back names an *earlier* entry that is still the
  head, so `undo_if_head` — the mechanism that exists to stop a toast reversing the wrong
  thing — cheerfully undoes it. Read it straight after the call that pushed, as its contract
  says.
- **`push_entry` on a missing stack stores nothing, so it now mints no sequence and emits no
  `StackChanged`.** It used to announce a change to a stack that does not exist.
- **`AppContext::new` injects the event hub into the undo manager**, beside the long-operation
  manager. It was previously injected lazily by whichever of `undo`/`redo`/`begin_composite`/
  `end_composite`/`cancel_composite` ran first — and *nothing pushes through those*, so on a
  context where the user had only ever edited, every `StackChanged` was emitted into a `None`
  hub and dropped. A subscriber cannot learn that "can undo" just became true from an event
  first delivered after the first undo.

### What you must do

**Regenerate, then check three things.**

1. **`add_command_to_stack` still returns `Result<()>`** and every generated controller is
   unchanged, so the 200-odd push sites need no edits.

2. **A hand-written `impl UndoRedoCommand`** compiles untouched — `label` is defaulted. Add
   it where you want a menu to name the command; a feature use case should use
   `UndoLabel::act("trash_binder_items")`, since its subject *is* the act.

3. **⚠ Check whether your copies of `event.rs`, `flat_event.rs` and
   `frontend/commands/undo_redo_commands.rs` are hand-edited before regenerating them.**
   This bit us in a real project: one consumer had switched `event.rs` to
   `parking_lot::Mutex` and its command facade to `lock()` rather than `lock_or_recover`, and
   its `flat_event.rs` carried a use case the manifest had since lost — so regenerating any
   of the three would have reverted working code or silently deleted a variant. Run
   `qleany diff <path>` on each and hand-apply the two new lines where the file has drifted.
   Only `common/src/undo_redo.rs` and `common/tests/undo_redo_tests.rs` are safe to
   regenerate blind.

4. **The generated undo tests were stale and are now fixed.** They called
   `manager.begin_composite(None)` without `.unwrap()`, from before that method returned
   `Result` — so regenerating `common/tests/undo_redo_tests.rs` used to break the build under
   `-D warnings`. It no longer does, and the file gains coverage for the untracked stack,
   sequences, labels, composite naming, the entry limit, `seal_head` and multi-stack
   isolation (which nothing tested before).

### What did not change

`undo`, `redo`, `can_undo`, `can_redo`, `add_command`, `add_command_to_stack`,
`begin_composite`, `end_composite`, `cancel_composite`, `clear_stack`, `clear_all_stacks`,
`create_new_stack`, `delete_stack` and `get_stack_size` all keep their **signatures**. `None`
still means stack `0`; `undo()` on an empty stack is still a successful no-op, which is why
`undo_if_head` returns a status rather than a `bool`.

Their *behaviour* is unchanged too, with the four exceptions in point 6 above — all of which
are corrections, not choices. If your application worked around the composite-target lockout
(by cancelling instead of ending a group, say, or by keeping one manager per document to avoid
it), that workaround is now unnecessary; leaving it in place is harmless.

---

## v1.9.0 to v1.10.0 — Generated code stops panicking, and passes `-D warnings`

**Qleany version**: v1.10.0

### What changed

Template-only release: no manifest schema change, no API surface added or removed. Two
themes, both of which show up the moment you regenerate.

**Panics become errors.** Every unit-of-work method already returned `Result`, yet the
templates reached for `.expect("Transaction not started")` and
`.read()/.write()/.lock().unwrap()` throughout — 969 non-test sites in a consuming project of
moderate size. They landed in the worst place: a use case running as a long operation on a
background thread, where `LongOperationManager`'s `catch_unwind` reports a failure as `Failed`
only if the failure is an `Err` and not a process-killing panic.

- `entity_units_of_work` gains a file-local `no_transaction()` helper and propagates a missing
  transaction with `.ok_or_else(no_transaction)?` instead of `.expect(...)`.
- `common_entity_repository`, `common_entity_table` and `hashmap_store` take every store lock
  through `read_or_recover` / `write_or_recover`, which already existed next to them but were
  wired into only the snapshot paths.
- The four `frontend_*` templates, `feature_use_case_uow` and `common_event` go through
  `lock_or_recover` — **now `pub`** (`common::long_operation::lock_or_recover`), where it was
  private before, and it now calls `clear_poison()` like its `RwLock` siblings instead of
  leaving the flag set for the next plain `.lock().unwrap()` to trip on.
- `macros_direct_access` gets both fixes inside its `quote!{}` bodies, which matter most:
  that code is emitted into *every* consumer's UoW.

Three latent bugs fixed alongside:

- `create` / `update` / `update_with_relationships` `.unwrap()`ed the row `*_multi` had just
  returned; a broken store contract now fails the one call that noticed, as
  `RepositoryError::Other`, rather than aborting.
- `HashMapStore::restore_savepoint` panicked on a missing savepoint, and it is reached from
  `Transaction`'s **`Drop`-time rollback** — where a panic aborts the process whatever the
  panic strategy. `UndoRedoManager::end_composite` did the same over a lost undo entry. Both
  now `debug_assert!` in debug and degrade in release, matching `write_guard::acquire`'s
  existing shape.
- The macro crate's `pluralize` indexed `chars().nth(word.len() - 2)` — mixing byte and char
  counts — and sliced `&word[..len - 1]`. Any non-ASCII entity name ending in `y` panicked at
  macro-expansion time.

`direct_access_lib` and `macros_lib` now carry
`#![warn(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`. Both crates generate at
zero panic paths outside tests; keeping the lint in the template rather than in the consuming
project is what keeps it that way across regeneration.

**Generated code passes `cargo clippy -- -D warnings`.** Six lints fired in every generated
project, which is a red CI for any consumer gating on warnings:

- `common_entity_table`, leaf branch: dropped the dead `RepositoryError` and `EntityId`
  imports (`impl_leaf_entity_table!` names both through fully-qualified `$crate::` paths). Two
  unused-import errors per leaf entity. The relationship branch keeps them — its expanded code
  names them unqualified.
- `common_entity_table` imported `delete_from_backward_junction` unconditionally, leaving a
  dead `use` on any trunk entity that nothing points at — `Root`, in practice. A v1.9
  regression: v1.8 emitted the import list conditionally and v1.9 lost the condition.
- `write_guard`: `assert!(cfg!(debug_assertions), ..)` asserts on a compile-time constant
  (`assertions_on_constants`), restated as `if !cfg!(..) { panic!(..) }`. The check is what
  proves the build profile and the observed contention behaviour agree, so it is kept.
- `hashmap_store` and `common_da_use_cases_create`: nested `if let` blocks became let-chains
  (`collapsible_if`; generated crates are edition 2024, so they are available), and
  `!(a && !b)` became `!a || b` (`nonminimal_bool`).
- `entity_controller`: the undoable `move_relationship` takes eight parameters — six of its own
  plus the `undo_redo_manager`/`stack_id` pair every undoable controller carries — and now
  carries `#[allow(clippy::too_many_arguments)]`. Allowed rather than restructured: bundling
  the arguments into a struct would obscure the correspondence with the use case it forwards to
  and change the call shape for every consumer, to satisfy a threshold of seven. Emitted only
  for undoable entities; the non-undoable variant takes six.

### Behavioral changes

- **Out-of-order UoW use now returns an error instead of panicking.** Calling a UoW method
  before `begin_transaction` yields `Err("unit of work used before begin_transaction …")`.
  Code that relied on the panic — a test asserting `#[should_panic]`, or a `catch_unwind`
  around a controller call — sees an `Err` instead.
- **A panicking long operation is reported, not fatal.** This is the practical payoff: the
  failure paths above now reach `catch_unwind` as `Err` values, so the operation settles as
  `Failed` with a message rather than killing the process.
- **Poison recovery is permanent.** `lock_or_recover` clears the poison flag, so one panicking
  thread no longer leaves every later plain `.lock().unwrap()` in your own code panicking on
  the same mutex.
- **`restore_savepoint` on a missing savepoint is a no-op in release** (the store stays as it
  is) and a `debug_assert!` failure in debug. Same for `end_composite` on a stack that was
  removed while a composite was open: the grouped commands stay applied, they just are not
  undoable as a unit.
- **Non-ASCII entity names ending in `y` now compile.** Previously the plural form panicked
  during macro expansion.
- **No behaviour change from the clippy fixes** — they change the shape of the emitted code,
  not what it does.

### How to upgrade

1. **Regenerate everything.** This release touches templates across all four groups, so a
   whole-project regeneration is the cheapest path. If you regenerate selectively, the
   affected outputs are: `hashmap_store.rs`, `long_operation.rs`, `undo_redo.rs`,
   `database/write_guard.rs`, `event.rs`, entity table files (`*_table.rs`), entity repository
   files (`*_repository.rs`), entity unit-of-work files (`*_units_of_work.rs`), entity
   controllers (`*_controller.rs`), the generated `create.rs` use case, feature use-case UoWs
   (`*_uow.rs`), the `macros` crate, and `frontend/src/commands/*`.
2. **Rebuild the macros crate.** `macros_direct_access` changed substantially; a stale build
   cache will keep emitting the old panicking bodies into your UoWs.
3. **Hand-written units of work**: replace `self.transaction.lock().unwrap()` with
   `common::long_operation::lock_or_recover(&self.transaction)`, and
   `.as_ref().expect("Transaction not started")` with
   `.as_ref().ok_or_else(|| anyhow!("no transaction is open on this UoW"))?`. Note the `?`
   needs a `let` binding first — `&expr?` type-checks the `?` against the expectation from `&`,
   so the inline form asks for `Transaction` and gets `&Transaction`, which `.expect()` had
   been hiding behind an autoderef coercion.
4. **Hand-written code holding store locks**: prefer `read_or_recover` / `write_or_recover`
   (`common::database::hashmap_store`, crate-internal) and the now-public
   `common::long_operation::lock_or_recover` over `.unwrap()`, so a poisoned lock degrades
   instead of cascading.
5. **Tests asserting a panic** on out-of-order UoW use or a missing savepoint must assert on
   the `Err` / no-op instead.
6. **If you gate CI on `cargo clippy -- -D warnings`**, it should now pass without per-project
   `allow`s. Any `#![allow(...)]` you added to a generated crate to work around the six lints
   above can be removed.

---

## v1.8.0 to v1.9.0 — Concurrency hardening: write guard, frozen reads, blocking waits

**Qleany version**: v1.9.0

### What changed

This release closes several gaps that only appear once an application runs more than one
thing at a time — a second project opened in the same process, a long operation reading on a
background thread while the UI thread writes, or a caller that needs a long operation's
result. None of them change the manifest schema.

**Single-write-transaction guard (new generated file).** `Transaction::begin_write_transaction`
takes a *whole-store* savepoint, and `rollback`/`Drop` restore it wholesale. That is correct
for one writer, and silently wrong for two: a second write transaction opened concurrently on
the same store would roll back to a savepoint taken before the first one's edits existed,
erasing them with no error and no event. Nothing enforced the assumption. A new
`crates/common/src/database/write_guard.rs` emits a `WriteTransactionGuard` — RAII, keyed per
store by `Arc` pointer identity, recording the holding thread and call site. Every generated
write unit of work now acquires it as the first statement of `begin_transaction` and releases
it on `commit`/`rollback`, so a new entity or use case picks the guard up the moment it is
generated. It is generated unconditionally — there is no manifest flag to opt out of an
invariant the generated transaction layer already depends on.

**Blocking waits for long operations.** `LongOperationManager` spawned a thread per operation
and stored its result but exposed no way to block until that happened, so callers polled on a
timer — putting a sleep-interval floor under every call however trivial the work. New
`OperationCompletion` (a `Mutex<HashSet<String>>` + `Condvar`) with
`LongOperationManager::completion_signal()` and `wait_for_operation(id, timeout)`. Each worker
publishes its id **last** — after storing the result, emitting the event and writing the final
status — so a woken waiter always observes a fully-settled operation.

**Frozen read transactions.** `HashMapStore::freeze()` captures an atomic, isolated view of
the store (O(1) — it holds every table lock for one instant and clones `im::HashMap` handles),
and `Transaction::begin_frozen_read_transaction` reads through it. Intended for long-operation
readers that walk the entity tree on a background thread while the UI thread keeps writing.

**Panic safety and lock-poison recovery.** `LongOperation::execute()` is wrapped in
`catch_unwind`, so a panicking operation is reported `Failed` instead of being left stuck
`Running` forever. The store's restore paths use new poison-tolerant `read_or_recover` /
`write_or_recover` helpers that call `clear_poison()`, so recovery is permanent rather than
one-shot and a poisoned table lock can no longer turn a `Drop`-time rollback into a
double-panic.

**Cross-cutting long-operation commands (new generated file).** The feature-agnostic surface
of `LongOperationManager` — everything that needs only an operation id — had no home and was
hand-written per project. `crates/frontend/src/commands/long_operation_commands.rs` is now
generated (modeled on `undo_redo_commands.rs`) and listed in `commands.rs`, exposing
`cancel_operation`, `get_operation_status` / `_progress` / `_result`, `is_operation_finished`,
`list_operations`, `get_operations_summary` and `cleanup_finished_operations`.

**Rolled-back transactions no longer leak their savepoint.** `rollback()` and the `Drop` safety
net now `discard_savepoint` after restoring, instead of holding a whole-store snapshot alive
for the process's lifetime.

### Behavioral changes

- **A latent double-writer bug now fails loudly.** This is the one to plan for. If your app
  ever opened two write transactions on one store concurrently, it previously "worked" while
  silently corrupting data on rollback; it now **panics in debug builds and returns an error in
  release builds**. The message names both parties — the holding thread and call site, and the
  refused one — and distinguishes a genuine second writer from a same-thread re-entrant or
  retried `begin_transaction`. Treat a new panic here as the guard reporting a real
  pre-existing bug, not as a regression introduced by upgrading.
- **Unrelated stores never contend.** The guard is keyed per store, not process-wide, so tests
  that build a fresh `DbContext` per `#[test]` and run concurrently under `cargo test` do not
  trip each other. No test-only carve-out is needed.
- **Long-operation waits cost nothing.** Replacing a 50 ms polling loop removes the per-call
  sleep floor entirely — a loop over 40 trivial operations that spent ~4 s sleeping now returns
  as fast as the work itself.
- **A panicking long operation is now observable.** It settles as `Failed` and emits its event;
  previously it was left `Running` and any waiter or poller hung indefinitely.
- **Frozen reads are opt-in.** Nothing generated calls `begin_frozen_read_transaction` — the
  default read path still reads the live store. It is a tool for you to wire into a
  long-operation read unit of work, not a change to existing behaviour.
- **Frozen reads are atomic w.r.t. themselves, not w.r.t. a writer's multi-step cascade.** A
  write transaction is not one critical section, so a cascading delete/create can leave a brief
  window into which `freeze` can land and capture a cross-table-torn snapshot. This shrinks the
  torn-read window from the whole read to a single instant; it does not eliminate it. See the
  `freeze()` doc comment for the deadlock invariant it relies on.

### How to upgrade

1. **Regenerate affected files**: `database.rs` and the new `database/write_guard.rs`,
   `hashmap_store.rs`, `transactions.rs`, `long_operation.rs`, every entity unit-of-work file
   (`*_units_of_work.rs`), every feature use-case unit-of-work file (`*_uow.rs`), and
   `frontend/src/commands/commands.rs` plus the new
   `frontend/src/commands/long_operation_commands.rs`.
2. **Delete hand-written long-operation commands**: if you wrote your own `cancel_operation`,
   `get_operation_status`, `list_operations` and friends, remove them in favour of the
   generated `long_operation_commands` module, or you will have two copies to keep in sync.
3. **Replace polling loops with the completion signal.** Take the handle, release the manager
   lock, *then* block — waiting while holding that lock stalls every other operation query for
   the operation's whole duration:

   ```rust,ignore
   let completion = ctx.long_operation_manager.lock().unwrap().completion_signal();
   // manager lock released here
   completion.wait_for(&op_id, Some(Duration::from_secs(30)));
   let result = ctx.long_operation_manager.lock().unwrap().get_operation_result(&op_id);
   ```

   `wait_for_operation` on the manager is a convenience for an owner holding it directly — do
   not call it through a shared lock.
4. **Hand-written units of work** (rare): if you implemented `CommandUnitOfWork` yourself
   rather than using the generated one, add the guard by hand — acquire it as the first
   statement of `begin_transaction` and clear the field in both `commit` and `rollback`, *after*
   the transaction's own `commit()`/`rollback()` call. Releasing it before the transaction
   finishes reopens the exact window the guard exists to close.
5. **If a regenerated app starts panicking in `begin_transaction`**, read the message rather
   than removing the guard: it names the call site that still holds the store's slot. The usual
   causes are a second `DbContext`-sharing writer running concurrently, or a unit of work that
   retried `begin_transaction` without a `commit`/`rollback` in between.

---

## v1.7.8 to v1.8.0 — Scoped undo restore (cross-trunk isolation)

**Qleany version**: v1.8.0

### What changed

Undo/redo `snapshot`/`restore` are now **scoped to the subtree they target** instead of
operating on the whole store. The v1.7.0 switch to `im::HashMap` made snapshot *capture* O(1)
by cloning the entire store, but `restore` then *replaced* the entire store — so undoing an
operation on one undoable trunk reverted every other trunk and all non-undoable data (the
savepoint behaviour the snapshot system was meant to replace). This release fixes that while
keeping the O(1) capture.

- `snapshot(ids)` still clones the whole store (O(1), `im` structural sharing) but now also
  records `root_ids`. `EntityTreeSnapshot` gains a `pub root_ids: Vec<EntityId>` field.
- `restore` walks the subtree rooted at `root_ids` (strong relationships only), in both the
  snapshot and the live store, and reconciles **only that subtree**: in-scope rows and the
  forward junctions they own are restored wholesale; the subtree's placement in *external*
  owners/referrers is reconciled surgically (membership only, preserving sibling edits made on
  other undo stacks); entities created after the snapshot are deleted. New generated methods:
  `Repository::restore_subtree` and per-backward-relationship `reconcile_backref_*`.
  `HashMapStoreSnapshot`'s table/junction fields are now `pub(crate)`.
- `UndoableCreateUseCase` no longer wholesale-resets the owner relationship on undo/redo (it
  relied on `set_relationships_in_owner`, which clobbered concurrent siblings); it now leans on
  the already-surgical `remove_multi` plus the scoped `restore`.
- `UndoableSetRelationshipUseCase` / `UndoableMoveRelationshipUseCase` no longer take a
  whole-store savepoint. They capture the affected junction row's prior value and restore just
  that row (a surgical inverse). `WriteRelUoW<RF>` gains a `get_relationship` method to read the
  pre-image.

### Behavioral changes

- **Cross-trunk isolation**: undoing an operation on one undoable trunk no longer reverts other
  trunks or non-undoable data (settings, caches, etc.). This is the headline fix.
- **Precise restore events**: restore now emits `Created`/`Updated`/`Removed` for exactly the
  affected ids (three-way diff of snapshot vs live), instead of a `Created` event for *every*
  entity in the store. Set/move-relationship undo emits a scoped `Updated` for the touched row
  instead of a whole-store `AllEvent::Reset`. UIs that relied on the old "everything changed"
  storm to force a full refresh may need to listen to the precise events.
- **Performance**: capture stays O(1); restore is now O(subtree) plus an O(n) scan per weak
  *backward* relationship (only when one exists), instead of an O(store) replace.
- **Unchanged**: snapshots remain in-memory only (`store_snapshot` is still `#[serde(skip)]`);
  id counters are still preserved across restore.

### How to upgrade

1. **Regenerate affected files**: `snapshot.rs`, `hashmap_store.rs`, entity repository files
   (`*_repository.rs`), entity unit-of-work files (`*_units_of_work.rs`), the use-case traits
   file, and the generated `create.rs`, `set_relationship.rs`, `move_relationship.rs` use cases.
2. **Custom feature use cases**: no API change. Code following the documented pattern
   (`self.snap = uow.snapshot(ids); … uow.restore(&snap)` on undo) becomes scoped automatically
   as long as it passes the correct `ids` — undo no longer reverts unrelated data.
3. **Hand-written `WriteRelUoW` implementations** (rare): add the new `get_relationship` method
   (delegate to the entity repository's `get_relationship`).
4. **If you relied on undo reverting the whole store** (the old behaviour): that no longer
   happens. For an explicit whole-database rollback, use `create_savepoint` /
   `restore_to_savepoint` (or `restore_store`) directly — do not route it through undo.

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
