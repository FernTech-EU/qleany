# Regeneration Workflow

This document explains how Qleany handles file generation and what happens when you regenerate code.

The GUI is a convenient way to generate files selectively.

## The Golden Rule

**Generated files are overwritten when you regenerate them.** Qleany does not merge changes or preserve modifications.

This is intentional. The workflow assumes you control what gets regenerated.

The GUI is helping you by checking the "in temp" checkbox by default to avoid accidental overwrites.

## Before You Generate

**Commit to Git first.** This isn't optional advice. It's how the tool is meant to be used. If something goes wrong, you can recover. If you accidentally overwrite modified files, you can restore them. Yes, it happened to me, and it was painful.

## Controlling What Gets Generated

### In the UI

The Generate tab shows all files that would be generated. You select which ones to actually write:

1. Click **List Files** to populate the file list
2. Use group checkboxes to select/deselect categories
3. Uncheck any files you've modified and want to keep
4. Click **Generate (N)** to write only selected files

### In the CLI

Inside the project folder, run:
```bash
# See what would change (only modified and new files are shown by default)
qleany list files

# Filter by status
qleany list files --modified       # only modified (-M)
qleany list files --new            # only new (-N)
qleany list files --unchanged      # only unchanged (-U)
qleany list files --all-status     # all statuses

# Filter by nature (infrastructure, aggregate, scaffold)
qleany list files --infra          # infrastructure only (-i)
qleany list files --aggregates     # aggregate only (-g)
qleany list files --scaffolds      # scaffold only (-s)
qleany list files --all-natures    # all natures

# Show everything (all statuses + all natures)
qleany list files --all

# Show output as a tree
qleany list files --format tree

# Show a unified diff for a specific file
qleany diff src/entities.rs

# Generate modified and new files (default)
qleany generate

# Generate to temp folder first (safe)
qleany generate --temp

# Dry run — see what would be written without writing
qleany generate --dry-run

# Generate only files for a specific feature, entity, or group
qleany generate feature MyFeature
qleany generate entity Car
qleany generate group "features"   # `qleany list groups` lists them all

# Generate a specific file by path
qleany generate file src/entities.rs

# Generate all files (all statuses + all natures)
qleany generate --all

# Then compare and merge manually
diff -r ./temp/crates ./crates

# or for VS Code users:
code --diff ./temp/file ./file
```

## What Happens When You Regenerate

- **Selected files are overwritten** — Your modifications are lost
- **Unselected files are untouched** — Even if the manifest changed
- **No files are deleted** — If you rename an entity, the old files remain; clean them up manually
- **By default, only modified and new files are written** — Use `--all` to include unchanged files of all natures

From the GUI (recommended), the "in temp" checkbox is checked by default to avoid accidental overwrites. Filter checkboxes let you control which files are visible by **status** (Modified, New, Unchanged) and **nature** (Infra, Aggregate, Scaffold).

In the CLI, `qleany generate` only writes files whose generated code differs from what's on disk (status `[M]` modified or `[N]` new). Use `--all` to force-write everything (all statuses and all natures), or `--dry-run` to preview without writing. You can also combine status and nature filters independently: e.g. `--modified --infra` shows only modified infrastructure files.

## Files That Must Stay in Sync

When you add or remove an entity, certain files reference all entities and must be regenerated together. If you've modified one of these files, you'll need to manually merge the changes.

### Rust

These files contain references to all entities:

| File                                         | Contains                                        |
|----------------------------------------------|-------------------------------------------------|
| `common/event.rs`                            | Event enum variants for all entities            |
| `common/entities.rs`                         | All entity structs                              |
| `common/database/hashmap_store.rs`           | Store table fields for all entities             |
| `common/direct_access/repository_factory.rs` | Factory methods for all repositories            |
| `common/direct_access/setup.rs`              | Factory methods for all repositories            |
| `common/direct_access.rs`                    | Module declarations for all entity repositories |
| `direct_access/lib.rs`                       | Module declarations for all entity features     |

`hashmap_store.rs` additionally pairs with the per-entity files that read it, `common/direct_access/{entity}/{entity}_table.rs` and `common/direct_access/{entity}/{entity}_repository.rs`. The store's field names and the code reading them are generated from the same entity list, yet they sit in different file groups — `base` for the store, `entities` for its readers — so regenerating one group alone leaves the two out of step and the crate stops compiling. Regenerate both groups, or run `qleany generate` with no target.

### C++/Qt

| File                                             | Contains                                |
|--------------------------------------------------|-----------------------------------------|
| `common/database/db_builder.h`                   | Database table builder for all entities |
| `common/direct_access/repository_factory.h/.cpp` | Factory methods for all repositories    |
| `common/direct_access/event_registry.h`          | Event objects for all entities          |
| `common/entities/CMakeLists.txt`                 | Adds all entity source files to build   |
| `direct_access/CMakeLists.txt`                   | Adds all entity source files to build   |

If you modify one of these files and later add a new entity, you'll need to either:
- Regenerate the file and re-apply your modifications, or
- Manually add the new entity references yourself

## Using the Temp Folder

It's recommended to add the "temp/" folder to your .gitignore.

The safest workflow when you've modified generated files:

1. Check **in temp/** checkbox in the UI (or use `--temp` or `--output ./whatever/` in CLI)
2. Generate all files to the temp location
3. Compare temp output against your current files:
   ```bash
   # Use the built-in diff command for individual files
   qleany diff src/entities.rs

   # Or compare entire directories
   diff -r ./temp/crates ./crates

   # or for VS Code users:
   code --diff ./temp/file ./file
   ```
4. Manually merge changes you want to keep
5. Delete the temp folder

This manual merge is the cost of customization. For files you modify heavily, consider whether the customization belongs in a separate file that won't conflict with generation.

## Practical Guidelines

Every generated file has a **nature** that tells you how to treat it. You can filter by nature in both the CLI (`--infra`, `--aggregates`, `--scaffolds`) and the GUI (checkboxes).

### Infrastructure files — regenerate freely

These are pure plumbing with no business logic (nature: `Infrastructure`):

- Entity structs (`common/entities/`)
- DTOs (`dtos.rs`, `dtos.h`)
- Repository implementations
- Table/cache definitions
- Event classes
- Database helpers, undo/redo infrastructure

### Scaffold files — modify and protect

These are starting points for your custom code (nature: `Scaffold`). After first generation, you'll typically modify them and avoid regenerating:

- Use case implementations (your business logic)
- Use case unit-of-work trait definitions and implementations
- Controllers (if you add custom endpoints)
- Main entry point (`main.rs`, `main.cpp`)

### Aggregate files — handle with care

These reference all entities/features and must be regenerated when you add or remove entities (nature: `Aggregate`). Be aware they may need manual merging if you've modified them:

- Module declarations (`lib.rs`, feature exports)
- Factory classes
- Event registries
- CMakeLists.txt files that list all entity sources

## When You Rename an Entity

Qleany doesn't track renames. If you rename `Car` to `Vehicle`:

1. Update the manifest with the new name
2. Generate the new `Vehicle` files
3. **Manually delete** the old `Car` files
4. Update any code that referenced `Car`

The old files won't be removed automatically because Qleany never deletes files.

## When Templates Improve

When Qleany's templates are updated (new features, bug fixes, better patterns):

1. Generate to temp folder with the new version
2. Compare against your existing generated files
3. Decide which improvements to adopt
4. For files you haven't modified: regenerate directly
5. For files you've modified: merge manually or regenerate and re-apply your changes

The manifest remains your source of truth. The same manifest with improved templates produces better output.
