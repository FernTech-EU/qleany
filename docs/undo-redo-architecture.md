# Entity Tree and Undo-Redo Architecture

Undo-redo systems are harder than they first appear. A robust implementation must handle complex scenarios like nested entities, cascading changes, and maintaining data integrity during undos and redos. Qleany's undo-redo architecture is designed to simplify these challenges by enforcing clear rules on how entities relate to each other in the context of undo-redo operations.

Entities in Qleany form a tree structure based on strong (ownership) relationships. This tree organization directly influences how undo-redo works across your application.

## Quick Reference

Before diving into the details, here is a summary of the two approaches Qleany supports:

| Aspect            | Approach A: Document-Scoped                           | Approach B: Panel-Scoped                                            |
|-------------------|-------------------------------------------------------|---------------------------------------------------------------------|
| Stack lifecycle   | Created when document opens, destroyed when it closes | Created when panel gains focus, cleared on focus loss or after undo |
| History depth     | Unlimited                                             | One command                                                         |
| Redo behavior     | Full redo history until new action                    | Single-use, lost on focus change                                    |
| Deletion handling | Optional stack-based or soft-delete with toast        | Soft-delete with timed toast                                        |
| User expectation  | "Undo my last change to this document"                | "Undo my immediate mistake"                                         |
| Best for          | IDEs, creative suites, document editors               | Form-based apps, simple tools                                       |

Qleany itself uses Approach B. Skribisto uses Approach A.

## My Recommendations

Do not use a single, linear undo-redo stack for the entire application except in the most basic cases. As Admiral Ackbar said: "It's a trap!" 

Think about interactions from the user's perspective: they expect undo and redo to apply to specific contexts rather than globally. A monolithic stack leads to confusion and unintended consequences. If a user is editing a document and undoes an action, they do not expect that to also undo changes in unrelated settings or other documents.

Instead, each context should have its own undo-redo stack. The question is how to define "context." Qleany supports two approaches, described below, suited to different application types. Both use the same generated infrastructure; they differ only in when and where stacks are created and destroyed.

For destructive operations such as deleting entities, Qleany supports cascading deletions and their undoing. If you delete a parent entity, all its strongly-owned children are also deleted, and you can undo that. At first, I used a database savepoint to be restored on undo, but the savepoint impacted non-undoable data as well, leading to confusion and unexpected behavior. Now, the `create`, `createOrphans`, `remove`, `setRelationshipsIds` and `moveRelationshipIds` commands use cascading snapshots of the individual tables to restore the database state before the operation.

Yet this behavior may be not what the user expects. Instead, you can use soft-deletion with timed recovery, described in the Soft Deletion section below.

## Two Approaches to Undo-Redo

### Approach A: Document-Scoped Stack

The stack is created when a document, workspace, or undoable trunk is loaded and destroyed when it closes. All UI panels editing entities within that trunk share the same stack.

This approach provides full undo history across the entire document. When the user presses Ctrl+Z, the application undoes the most recent change to the document regardless of which panel made it. This matches the behavior of professional tools like Qt Creator, Blender, and Adobe applications.

Redo works symmetrically: the user can redo any undone action until they perform a new action, which clears the redo stack.

**Lifecycle.** Create the stack when the document opens. Destroy it when the document closes. All panels resolve the same `stack_id` by looking up the document they are editing.

**User expectation.** "Undo my last change to this document."

**Best suited for.** Complex applications, professional tools, creative suites, IDEs, and any application where users expect deep undo history and work on persistent documents over extended sessions.

### Approach B: Panel-Scoped Stack, Length 1

The stack is created when a panel becomes active and cleared or destroyed when the panel loses focus or after a single undo executes. Each panel manages its own short-lived stack holding at most one command.

This approach provides immediate mistake recovery without maintaining history. When the user presses Ctrl+Z, the application undoes only their most recent action in that panel. After one undo, the stack is empty. This matches modern application patterns where undo is an "oops" button rather than a time-travel mechanism.

Redo is effectively single-use in this approach. After undoing, the user can redo immediately, but switching focus or performing any new action clears the redo slot. This is an acceptable trade-off for the simplicity gained.

**Lifecycle.** Create the stack when the panel gains focus. Clear or destroy it when the panel loses focus or after undo executes.

**User expectation.** "Undo my immediate mistake."

**Best suited for.** Simpler applications, form-based interfaces, and applications where deep undo history would cause more confusion than benefit.

## Entity Properties

With the approach chosen, configure your entities using these properties relevant to undo-redo:

| Property              | Type | Default | Effect                                                   |
|-----------------------|------|---------|----------------------------------------------------------|
| `undoable`            | bool | false   | Adds undo/redo support to the entity's controller        |
| `single_model`        | bool | false   | Generates `Single{Entity}` wrapper for QML (C++/Qt only) |

## Undo-Redo Rules

The undo-redo system follows strict inheritance rules through the entity tree:

1. **A non-undoable entity cannot have an undoable entity as parent** (strong relationship)
2. **All children of an undoable entity must also be undoable**
3. **Weak relationships (references) can point to any entity** regardless of undo status

These rules ensure that when you undo an operation on a parent entity, all its strongly-owned children can be consistently rolled back.

> *Qleany enforces these rules, among other rules, at loading time generation time and in the GUI.*
>
> Type `qleany check --rules` to see the full list of rules.
> 
> **What happens if you violate these rules?** Undo/redo stacks will become inconsistent. For example, if you place non-undoable persistent settings as a child of an undoable entity, those settings could be unexpectedly undone by cascade when the user undoes the parent. You do not want application settings disappearing because the user undid an unrelated action.
>
> Follow these rules strictly. If data should not participate in undo (like settings), place it in a separate non-undoable trunk — do not nest it under undoable entities.
>


## Entity Tree Configurations

Depending on your application's complexity, you can organize your entity tree in three ways.

### Configuration 1: No Undo-Redo

For simple applications where undo-redo is not needed, all entities are non-undoable.

```
Root (undoable: false)
├── Settings
├── Project
│   ├── Document
│   └── Asset
└── Cache
```

```yaml
entities:
  - name: Root
    inherits_from: EntityBase
    undoable: false
    fields:
      - name: settings
        type: entity
        entity: Settings
        relationship: one_to_one
        strong: true
      - name: projects
        type: entity
        entity: Project
        relationship: ordered_one_to_many
        strong: true
```

Even without user-facing undo-redo, the undo system must be initialized internally as it is used for transaction management.

### Configuration 2: Single Undoable Trunk

For applications where all user data should support undo-redo, the root is non-undoable with a single undoable trunk beneath it.

```
Root (undoable: false)
└── Workspace (undoable: true)     ← All user data under this trunk
    ├── Project (undoable: true)
    │   ├── Document (undoable: true)
    │   └── Asset (undoable: true)
    └── Tag (undoable: true)
```

```yaml
entities:
  - name: Root
    inherits_from: EntityBase
    undoable: false
    fields:
      - name: workspace
        type: entity
        entity: Workspace
        relationship: one_to_one
        strong: true

  - name: Workspace
    inherits_from: EntityBase
    undoable: true
    fields:
      - name: projects
        type: entity
        entity: Project
        relationship: ordered_one_to_many
        strong: true
      - name: tags
        type: entity
        entity: Tag
        relationship: one_to_many
        strong: true

  - name: Project
    inherits_from: EntityBase
    undoable: true
    fields:
      - name: documents
        type: entity
        entity: Document
        relationship: ordered_one_to_many
        strong: true
```

With Approach A, create one stack when the Workspace loads. All panels share this stack, and the user has full undo history across the entire workspace.

With Approach B, each panel creates and manages its own stack independently. The user has immediate undo within each panel, with deletions handled via toast notifications.

### Configuration 3: Multiple Trunks

For applications that need both undoable user data and non-undoable system data, or for multi-document applications where each document should have independent undo history, the root has multiple trunks.

```
Root (undoable: false)
├── System (undoable: false)       ← Non-undoable trunk
│   ├── Settings (undoable: false)
│   ├── RecentFiles (undoable: false)
│   └── SearchResults (undoable: false)
│
└── Workspace (undoable: true)     ← Undoable trunk
    ├── Event (undoable: true)
    │   └── Attendee (undoable: true)
    └── Calendar (undoable: true)
```

For multi-document applications:

```
Root (undoable: false)
├── System (undoable: false)
├── Document A (undoable: true)    ← Stack A
├── Document B (undoable: true)    ← Stack B
└── Document C (undoable: true)    ← Stack C
```

```yaml
entities:
  - name: EntityBase
    only_for_heritage: true
    fields:
      - name: id
        type: uinteger
      - name: created_at
        type: datetime
      - name: updated_at
        type: datetime
  - name: Root
    inherits_from: EntityBase
    undoable: false
    fields:
      - name: system
        type: entity
        entity: System
        relationship: one_to_one
        strong: true
      - name: workspace
        type: entity
        entity: Workspace
        relationship: one_to_one
        strong: true

  - name: System
    inherits_from: EntityBase
    undoable: false
    fields:
      - name: settings
        type: entity
        entity: Settings
        relationship: one_to_one
        strong: true
      - name: recentFiles
        type: entity
        entity: RecentFile
        relationship: ordered_one_to_many
        strong: true
      - name: searchResults
        type: entity
        entity: SearchResult
        relationship: one_to_many
        strong: true

  - name: Settings
    inherits_from: EntityBase
    undoable: false
    fields:
      - name: theme
        type: string
      - name: language
        type: string

  - name: SearchResult
    inherits_from: EntityBase
    undoable: false
    fields:
      - name: query
        type: string
      - name: matchedItem
        type: entity
        entity: Event
        relationship: many_to_one

  - name: Workspace
    inherits_from: EntityBase
    undoable: true
    fields:
      - name: events
        type: entity
        entity: Event
        relationship: ordered_one_to_many
        strong: true
      - name: calendars
        type: entity
        entity: Calendar
        relationship: one_to_many
        strong: true

  - name: Event
    inherits_from: EntityBase
    undoable: true
    single_model: true
    fields:
      - name: title
        type: string
      - name: attendees
        type: entity
        entity: Attendee
        relationship: one_to_many
        strong: true
        list_model: true
```

With Approach A, each document gets its own stack. Ctrl+Z in Document A's editor undoes only Document A's changes. This provides natural contextual undo at the document level.

With Approach B, the multi-document structure is less relevant since each panel manages its own immediate-undo stack regardless of which document it edits.

Here is the section, written to sit between Configuration 3 and Cross-Trunk References:

---

## Breaking the Mold

The three configurations above are the patterns I recommend and use myself. They are not the only ones the infrastructure supports.

Qleany's generated code does not enforce a single Root entity. It does not enforce tree-structured ownership at all. The repository layer provides `createOrphans` alongside `create`. The undo/redo system keys its stacks by integer ID, not by position in a tree. The snapshot/restore system captures whatever entity graph it finds. Nothing checks that your entities form a coherent tree at runtime.

This means you can do things the configurations above don't show:

**Multiple independent roots.** You can create several root-like entities, each owning a separate subtree with its own undo stack. Think of a multi-workspace IDE where each workspace is truly independent — its own entities, its own undo history, no shared state. This works. I haven't needed it in Skribisto or Qleany, but the infrastructure won't stop you.

**Flat orphan entities.** You can skip the tree model entirely and use `createOrphans` for everything, managing relationships through weak references. For a simple utility with a handful of entities and no undo/redo, this is less ceremony than setting up a Root → Workspace hierarchy you don't need.

**Hybrid approaches.** A tree for your main domain model, orphan entities for transient data that doesn't belong in the tree. The infrastructure doesn't care.

So why do I recommend the tree model so insistently?

Because the tree model gives you things for free that you must handle manually without it. Cascade deletion follows ownership: delete a parent, all strongly-owned children are deleted. Snapshot/restore captures the full subtree: undo a deletion, everything comes back including nested children and their junction relationships. Undo stack scoping maps naturally to tree branches: one stack per document, one stack per workspace.

Without the tree, you take on these responsibilities yourself. Orphan entities have no owner to cascade from, you must track and delete them explicitly. A parent's snapshot does not capture entities outside a tree, you must manage their lifecycle in your use case logic. Undo stack assignment becomes your problem rather than a natural consequence of the data structure.

None of this is impossible. It's just work that the tree model handles for you.

> If you deviate from the prescribed configurations, the undo/redo rules from the previous section still apply. A non-undoable entity should not be strongly owned by an undoable entity, regardless of your tree topology. The infrastructure won't warn you. The undo stacks will just become inconsistent, and you'll spend an afternoon figuring out why.

My advice: start with the tree model. If you later find it too rigid for a specific part of your application, relax it locally — use orphans for that part, keep the tree for the rest. Don't start with a flat model and try to add structure later. It's easier to remove structure than to add it.

---

## Cross-Trunk References

Non-undoable entities can hold weak references (many_to_one, many_to_many) to undoable entities. This is useful for search results, recent items, or bookmarks that point to user data without owning it.

```yaml
- name: SearchResult
  undoable: false
  fields:
    - name: matchedEvent
      type: entity
      entity: Event
      relationship: many_to_one
```

The reverse is also true: undoable entities can reference non-undoable entities, such as referencing a Settings entity for default values.

## Soft Deletion

Definition: deletions are handled outside the undo stack using soft-deletion with timed hard-deletion.

To implement soft deletion, add an `activated` boolean field to your entities. When "deleting" an entity, set this flag to false instead of removing it from the database. Your UI filters out entities where `activated` is false, effectively hiding them from the user.

For immediate recovery, display a toast notification with an "Undo" action for a few seconds after deletion, typically three seconds. Maintain a timer for each soft-deleted entity. If the user clicks "Undo" within the timeout window, restore the entity by setting `activated` back to true and cancel the timer. If the timeout expires, perform the hard-delete.

This pattern is time-bounded rather than focus-bounded. The user can switch panels, notice the toast still visible, and click "Undo" within the window. It matches user expectations from applications like Gmail, Slack, and Notion.

For longer-term recovery, you can implement a trash bin with a dedicated entity that holds references to soft-deleted items:

| Id | trashed_date | entity_type | entity_id |
|----|--------------|-------------|-----------|
| 1  | 2024-01-01   | Document    | 42        |
| 2  | 2024-01-02   | Car         | 7         |

Users can then restore items from the trash bin or permanently delete them. Permanently emptying the trash clears all undo-redo stacks, which is acceptable since permanent deletion is a non-undoable action from the user's perspective as well.

For Approach A, you may alternatively implement deletion undo through the stack if your application requires full undo history for deletions, but the soft-deletion pattern remains simpler and avoids cascade-reversal complexity.

> Note : Soft deletion isn't baked-in to Qleany's generated code. You must implement the `activated` field, filtering logic, toast UI, and timer management yourself. I only provide this pattern as a recommended best practice. To only display non-deleted entities, you can use QAbstractProxyModel in C++/Qt or filter models in QML.

## Choosing Your Approach

The key questions to ask are: Do you have data that should not participate in undo? Do users expect deep history or just immediate mistake recovery? Will users work on multiple independent documents simultaneously?

| Application Type   | Entity Configuration     | Recommended Approach |
|--------------------|--------------------------|----------------------|
| Simple utility     | No undo-redo             | Neither              |
| Form-based app     | Single undoable trunk    | Approach B           |
| Document editor    | Single undoable trunk    | Approach A           |
| Multi-document IDE | Multiple undoable trunks | Approach A           |
| Creative suite     | Multiple undoable trunks | Approach A           |

Settings, preferences, search results, and caches belong in non-undoable trunks. User-created content belongs in undoable trunks. Temporary UI state belongs outside the entity tree entirely or in non-undoable trunks.

## Naming, identifying and bounding an entry

Everything above is about *which stack*. These are about *which entry*, and they exist
because an application that offers one Undo command over the whole app needs answers the
manager could not give.

**A toast's Undo must name its operation.** `undo()` pops whatever is on top, and between
offering the button and the user clicking it anything may have pushed — an autosave, a second
window, a background job. The button then reverses that instead, silently, while the user
believes they took back what the toast named. So each push mints a sequence:

```rust
manager.add_command_to_stack(Box::new(uc), stack_id)?;
let seq = manager.last_pushed_seq();          // the entry it landed in
…
match manager.undo_if_head(stack_id, seq)? {
    UndoStatus::Undone     => { /* done */ }
    UndoStatus::Superseded => { /* say so; do not undo something else */ }
    UndoStatus::Empty      => { /* nothing left */ }
}
```

A *merged* push reports the sequence of the entry it merged into, which is what a caller
holding it wants: for a burst of coalesced typing, the burst is the operation.

**A menu row should say what it will undo.** `UndoRedoCommand::label()` returns an
`UndoLabel { subject, action }` — a machine key, never a sentence, because generated code
knows nothing about locales. The application maps `("binder_item", "remove")` onto its own
translation. Read the head's with `undo_label(stack_id)` / `redo_label(stack_id)`. A group
names itself with `begin_composite_labeled(…)`: its constituents describe its parts, so a
menu built from them says *"Undo update"* for what the user did as one act.

**Some writes must happen and must not be history.** A buffer mirrored to the store on a
timer, a cache line, an index rebuilt on open. `stack_id: None` does not opt out — it means
stack `0` — so the doctrine above ("a throwaway stack, cleared after execution") relies on a
clear that nobody remembers. `UNTRACKED_STACK_ID` drops the command at the door instead, so
there is nothing left to clear. Opening a composite on it is an error.

**History needs a ceiling.** `set_undo_limit(Some(n))` drops the oldest entries past `n`;
`None` is the old unbounded behaviour. Worth setting: every entry can pin an
`EntityTreeSnapshot`, so a day-long session has no bound at all. For scale, LibreOffice
defaults to 100 steps and caps at 1000.

**And a way to end a merge run.** Merging looks only at the shape of two commands — adjacent,
close in time — and cannot see that something unrelated happened between them. A caller that
knows a dividing line was crossed calls `seal_head(stack_id)`, and the next command starts a
new entry.

**Events.** `UndoRedoEvent::StackChanged` fires when history is *created* (a push, a merge) or
cleared, not only when it is consumed, and every variant carries the stack id in
`Event::data`. Without both, a UI has to poll, and a process holding one stack per open
document cannot tell whose history moved. The hub is injected into the manager by
`AppContext::new`, not by the first undo — inject it later than that and the push events, the
whole reason the variant exists, are emitted into nothing.

**A record-nothing path reports nothing.** `last_pushed_seq()` is `None` after any call that
did not create an entry: an untracked write, a command folded into a still-open composite, a
group that closed empty or whose stack had gone, a cancelled group. This matters more than it
looks. A stale register does not merely misreport — the number it returns names an *earlier*
entry that is still the head, so `undo_if_head` finds a match and undoes it. The mechanism
built to stop a toast reversing the wrong thing would be the thing doing it.

**A composite owns its stack only while it is open.** `begin_composite` refuses a stack other
than the one already in progress, so `end_composite`, `cancel_composite` and
`clear_all_stacks` all release the target. Forget it in any one of them and that manager is
pinned to a single stack for compositing forever after — which, in a process holding one
stack per open document, means every document after the first quietly loses grouping.

## Snapshots

Delete a `Calendar` and you don't just delete one row. You delete its `CalendarEvent`s, their `Reminder`s, and every junction table entry connecting those events to `Tag`s. Now undo that. You need to put back the entire tree, exactly as it was, relationships and all. That's what snapshots do.

Before a destructive command runs, the use case walks the ownership tree downward and serializes everything it finds into an `EntityTreeSnapshot` — entity rows, junction table entries, ordering data. On `undo()`, the snapshot is replayed. The tree reappears as if nothing happened.

How expensive is this? A Calendar with 200 events, each having 2 reminders and 3 tags, produces a snapshot of 1,801 rows. One calendar row, 200 event rows, 200 ordering entries, 400 reminder rows, 400 reminder junction entries, 600 event-tag junction entries. All serialized into memory before the deletion even starts. Weak references (the Tag entities themselves) are not captured — only the junction entries pointing to them.

The generated CRUD use cases handle all of this for you. `create` snapshots after insertion so undo can delete. `remove` snapshots before deletion so undo can restore. `setRelationshipIds` and `moveRelationshipIds` snapshot affected relationships before modification. `update` is scalar-only — it cannot change relationships, so it just stores a before/after pair of scalar fields. Cheaper. `updateWithRelationships` writes both scalars and junction tables; its undo stores a before/after pair of full entities (including relationship data).

**Your feature use cases get none of this.** The snapshot methods are available on the unit of work (`snapshotCalendar(ids)`, `restoreCalendar(snap)`), but nobody calls them for you. If your feature use case is undoable and you skip the snapshot calls, undo will do nothing. Silently. I've been there. Non-undoable use cases and long operations don't need snapshots — the transaction rollback handles failures.

Keep snapshot cost in mind when setting `undoable: true` on entities that accumulate large numbers of children. Single-entity updates are free. Deleting a parent with thousands of children is proportional to the subtree size. If that's your situation, either make it non-undoable or accept the latency.

## Savepoints

In the land of persistence, this is the nuclear option. Be cautious.

A savepoint captures the state of the **entire database** at a given point in time, without any distinction between undoable and non-undoable entities. Nice in theory, less nice with an undo/redo system.

Why did I implement it? At first, I thought about using savepoints instead of snapshots to undo cascade-deletions. Simpler logic, no tree walking. However, I quickly ran into the problem: non-undoable entities get reverted to an earlier state too. Application settings, caches, anything stored in the same database. I switched to the snapshot system, which is more complex but gives precise control over what gets undone and what doesn't.

Why keep it? If you are not using the undo/redo system, if you have a basic application with orphan entities and no undoable/non-undoable distinction, a savepoint can be a quick way to revert the entire database to a previous state. A very specific situation.

My recommendation: keep your finger away from the big red button.


## Command Composition

Command composition groups multiple operations into a single undo/redo unit. The user presses Ctrl+Z once and all grouped operations are undone together.

### Two-Tier Architecture

The undo/redo system has two tiers with different composition capabilities:

**Tier 1 — QML-facing APIs are always pre-composed and atomic.** Each controller method exposed to QML (a `Q_INVOKABLE` returning `QCoro::QmlTask`) is a complete operation: one call equals one undo unit. The monolithic `create` (which bundles `createOrphan` + relationship attachment with snapshot-based undo) is the correct design for QML. QML developers get one call that does the right thing. `QmlTask::then(QJSValue)` returns `void` — there is no way to chain calls, pass values between steps, or intercept commands from JavaScript.

**Tier 2 — C++ feature controllers can compose operations.** When writing a custom feature use case (like `import_inventory` or `load_work`), use `executeDeferredCommand` to run multiple entity operations and collect the resulting commands. Assemble them into a `GroupCommand` and push the group as a single undo unit. The QML caller sees this as one atomic operation.

### Rust: `begin_composite` / `end_composite`

In Rust, controllers execute synchronously. Command composition uses a bracket pattern on `UndoRedoManager`:

```rust
undo_redo_manager.begin_composite(Some(stack_id))?;

// All commands added here go into the composite
workspace_controller::create(..., undo_redo_manager, Some(stack_id), ...)?;
feature_controller::update(..., undo_redo_manager, Some(stack_id), ...)?;

undo_redo_manager.end_composite();
// Single undo() now reverses both operations
```

`begin_composite` returns `Result<()>` — it fails if a composite is already in progress for a different stack. The `end_composite` call finalizes the composite and pushes it as a single command.

If something goes wrong mid-composite, call `cancel_composite()` instead of `end_composite()`. This will undo any sub-commands that were already executed within the composite (in reverse order), then discard the composite entirely. The undo stacks remain clean.

The `begin_composite` / `end_composite` pattern works because Rust controller calls are blocking — each completes before the next starts. The `UndoRedoManager` routes commands to the in-progress composite instead of the main stack.

### C++/Qt: `executeDeferredCommand` + `GroupCommand`

In C++/Qt, the QCoro coroutine model means each `co_await` suspends the coroutine and returns to the event loop. A global `beginGroupCommand` / `endGroupCommand` flag on `UndoRedoSystem` would be unsafe — between suspension points, other coroutines can run and their commands would be routed to the wrong group.

Instead, C++/Qt uses an explicit command-collection pattern: the `executeDeferredCommand` helper executes a use case on a background thread (same as normal commands) and returns both the result and the command object without pushing to any stack. The caller collects commands locally and assembles a `GroupCommand`.

The three helpers in `controller_command_helpers.h`:

| Helper | Purpose |
|--------|---------|
| `executeDeferredCommand<T>(...)` | Execute use case, return `{result, command}` |
| `executeDeferredCommandVoid(...)` | Same for void use cases, return `{success, command}` |
| `rollbackDeferredCommands(commands)` | Undo a list of commands in reverse order |

### C++/Qt: Full Implementation Example

This example shows a feature controller method that imports cars with their tag relationships as a single undo unit. If any step fails, previous steps are rolled back.

```cpp
QCoro::Task<bool> InventoryManagementController::importInventory(
    const ImportInventoryDto &importInventoryDto)
{
    namespace Helpers = Common::ControllerHelpers;
    namespace UndoRedo = Common::UndoRedo;

    auto onError = [this](const QString &cmd, const QString &msg) {
        if (m_featureEventRegistry)
            m_featureEventRegistry->publishError(cmd, msg);
    };

    // Collect executed commands for GroupCommand assembly or rollback
    QList<std::shared_ptr<UndoRedo::UndoRedoCommand>> executed;

    // ── Step 1: Create car entities ──────────────────────────────

    auto carUow = std::make_unique<CarUnitOfWork>(*m_dbContext, m_eventRegistry);
    auto createCarsUC = std::make_shared<CreateCarsUseCase>(std::move(carUow));

    auto [cars, createCmd] = co_await Helpers::executeDeferredCommand<QList<CarDto>>(
        u"Create cars"_s,
        createCarsUC,
        kDefaultCommandTimeoutMs,
        onError,
        importInventoryDto.cars(),
        importInventoryDto.ownerId(),
        -1 /* append */);

    if (!createCmd)
    {
        // Step 1 failed — nothing to roll back
        co_return false;
    }
    executed.append(createCmd);

    // ── Step 2: Set tag relationships (depends on step 1 results) ─

    for (const auto &car : cars)
    {
        if (car.tagIds().isEmpty())
            continue;

        auto tagUow = std::make_unique<CarUnitOfWork>(*m_dbContext, m_eventRegistry);
        auto setTagsUC = std::make_shared<SetRelationshipIdsUseCase>(std::move(tagUow));

        auto [ok, tagCmd] = co_await Helpers::executeDeferredCommandVoid(
            u"Set car tags"_s,
            setTagsUC,
            kDefaultCommandTimeoutMs,
            onError,
            car.id(),
            CarRelationshipField::Tags,
            car.tagIds());

        if (!tagCmd)
        {
            // Step 2 failed — roll back all previous steps
            co_await Helpers::rollbackDeferredCommands(executed);
            co_return false;
        }
        executed.append(tagCmd);
    }

    // ── All steps succeeded — register as a single undo unit ─────

    auto group = std::make_shared<UndoRedo::GroupCommand>(u"Import inventory"_s);
    for (auto &cmd : executed)
        group->addCommand(cmd);

    // Push to the undo stack without executing — children already ran
    m_undoRedoSystem->manager()->pushCommand(group, m_undoRedoStackId);

    co_return true;
}
```

Key points:

- **Each `executeDeferredCommand` runs on a background thread** via `QtConcurrent::run`, identical to the normal command path. The UI stays responsive.
- **Results flow between steps.** Step 2 uses `cars` from step 1 to set relationships. This is natural with sequential `co_await`.
- **Rollback on failure.** If any step fails, `rollbackDeferredCommands` undoes all previous steps in reverse order. Each undo also runs on a background thread.
- **No global state.** The `executed` list is local to the coroutine. Multiple feature controllers can run grouped operations concurrently without interference.
- **The `GroupCommand` is pushed without executing.** The children were already executed by `executeDeferredCommand`. When the user undoes, `GroupCommand::asyncUndo()` undoes all children in reverse order. Redo replays them forward.
- **QML sees one atomic operation.** The foreign controller wraps this method as a single `Q_INVOKABLE` returning `QCoro::QmlTask`. From QML, it is indistinguishable from any other controller call.

### How `GroupCommand` Handles Undo/Redo

After `pushCommand(group, stackId)`, the `GroupCommand` sits on the undo stack. Here is what happens on undo and redo:

1. **User presses Undo** — `UndoRedoStack::undo()` pops the `GroupCommand`, calls `GroupCommand::asyncUndo()`.
2. **`asyncUndo()`** iterates children in reverse order: for each child, connects to its `finished` signal, calls `child->asyncUndo()`.
3. **Each child's undo** runs via `QtConcurrent::run` (the use case's `undo()` function). When done, the child emits `finished`.
4. **`GroupCommand` advances** to the next child (reverse). After all children complete, emits its own `finished`.
5. **Stack moves** the `GroupCommand` to the redo stack.

Redo is the mirror image: children are re-executed in forward order via `asyncRedo()`.

If any child's undo fails, `GroupCommand`'s failure strategy kicks in. The default is `StopOnFailure` — it stops and reports failure. You can set `RollbackAll` (undo all successfully undone children back to their pre-undo state) or `ContinueOnFailure` via `GroupCommandBuilder`:

```cpp
auto group = GroupCommandBuilder(u"Import inventory"_s)
    .onFailure(FailureStrategy::RollbackAll)
    .build();
```

---

For implementation details of the undo/redo system including command infrastructure, async execution, and composite commands, see [Generated Infrastructure - C++/Qt](generated-code-cpp-qt.md) or [Generated Infrastructure - Rust](generated-code-rust.md).
