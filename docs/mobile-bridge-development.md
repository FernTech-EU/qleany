# Mobile Bridge Development

Qleany can generate a **mobile bridge** crate that wraps your Rust backend for consumption by iOS (Swift) and Android (Kotlin) apps. The bridge uses [UniFFI](https://mozilla.github.io/uniffi-rs/) to produce a synchronous FFI surface, with platform-native async wrappers generated as Swift and Kotlin source files.

## Enabling Mobile Targets

Add `rust_ios` and/or `rust_android` to your manifest's `ui` section:

```yaml
ui:
  rust_cli: true
  rust_slint: true
  rust_ios: true       # generates mobile_bridge + Swift wrappers
  rust_android: true   # generates mobile_bridge + Kotlin wrappers
```

Either flag triggers generation of the `mobile_bridge` crate. Both flags together generate wrappers for both platforms.

Platform-specific build and integration guides are generated alongside the crate:
- **iOS**: `crates/mobile_bridge/README-iOS.md` (prerequisites, `cargo-swift` workflow, Xcode integration)
- **Android**: `crates/mobile_bridge/README-Android.md` (prerequisites, `cargo-ndk` workflow, Gradle setup, ProGuard rules)

## Generated Structure

```
crates/mobile_bridge/
├── Cargo.toml
├── uniffi.toml                       # UniFFI custom type mappings
├── src/
│   ├── lib.rs                        # Module declarations + re-exports
│   ├── mobile_types.rs               # Cross-module type re-exports
│   ├── backend.rs                    # MobileBackend lifecycle + listeners
│   ├── custom_types.rs               # MobileDateTime
│   ├── errors.rs                     # MobileError enum
│   ├── events.rs                     # MobileEventKind, dispatch, fan-out
│   ├── undo_redo_commands.rs         # Undo/redo wrappers
│   ├── {entity}_commands.rs          # Per-entity CRUD + relationships
│   └── {feature}_commands.rs         # Per-feature use case wrappers
├── tests/
│   └── integration_tests.rs
├── swift/
│   ├── MobileBackend+Async.swift     # Swift async/await wrappers
│   └── MobileBridgeTests.swift       # XCTest suite
├── kotlin/
│   ├── MobileBackendAsync.kt         # Kotlin suspend wrappers
│   └── MobileBridgeTest.kt           # JUnit test suite
├── README-iOS.md                     # iOS build guide
└── README-Android.md                 # Android build guide
```

## Architecture

The mobile bridge sits between platform code and the `frontend` crate:

```
     Swift / Kotlin (async)
              │
     ┌────────┴────────┐
     │  mobile_bridge   │   Synchronous UniFFI surface
     │  (cdylib/static) │
     └────────┬────────┘
              │
     ┌────────┴────────┐
     │    frontend      │   Commands, AppContext, FlatEvent
     └────────┬────────┘
              │
     ┌────────┴────────┐
     │  common, direct_ │   Entities, repos, database
     │  access, features│
     └─────────────────┘
```

**Key design decisions:**

- The Rust core stays fully synchronous. No async runtime.
- Platform-native async (`Task.detached` on iOS, `withContext(Dispatchers.IO)` on Android) wraps the synchronous calls.
- All operations go through `frontend::commands` to ensure undo/redo registration and event emission.

## MobileBackend

The entry point is the `MobileBackend` object. It owns the `AppContext` (database, event hub, undo/redo manager) and provides all operations as methods.

```swift
// Swift
let backend = MobileBackend()
let item = try await backend.createItemAsync(stackId: nil, dto: dto, ownerId: 1, index: 0)
backend.shutdown()
```

```kotlin
// Kotlin
val backend = MobileBackend()
scope.launch {
    val item = backend.createItemAsync(stackId = null, dto = dto, ownerId = 1uL, index = 0)
}
// Later, when done:
backend.shutdown()
```

### Lifecycle

1. `MobileBackend()`: creates database, event hub, starts dispatch thread
2. Use CRUD/feature/undo-redo methods
3. `shutdown()`: stops dispatch thread, releases resources

### Entity Operations

For each entity, the bridge exposes:
- `create_{entity}` / `create_{entity}_multi` (with owner)
- `create_orphan_{entity}` / `create_orphan_{entity}_multi`
- `get_{entity}` / `get_{entity}_multi` / `get_all_{entity}`
- `update_{entity}` / `update_{entity}_multi`
- `update_{entity}_with_relationships` / `update_{entity}_with_relationships_multi`
- `remove_{entity}` / `remove_{entity}_multi`
- `get_{entity}_relationship` / `get_{entity}_relationship_many`
- `get_{entity}_relationship_count` / `get_{entity}_relationship_in_range`
- `set_{entity}_relationship` / `move_{entity}_relationship`

Undoable entities accept an optional `stack_id` parameter.

### Undo/Redo

- `undo(stack_id)` / `redo(stack_id)`
- `can_undo(stack_id)` / `can_redo(stack_id)` (lightweight, UI-thread-safe)
- `create_new_stack()` / `delete_stack(stack_id)`
- `begin_composite(stack_id)` / `end_composite()` / `cancel_composite()`
- `clear_stack(stack_id)` / `clear_all_stacks()`
- `get_stack_size(stack_id)` (lightweight, UI-thread-safe)

### Feature Commands

Feature use case method names include the **feature prefix** to avoid collisions across features. For example, a `save` use case in the `handling_manifest` feature becomes `handling_manifest_save()`. The async wrappers follow the same convention: `handlingManifestSaveAsync()` in Swift/Kotlin. The DTOs and enums generated from a feature use case carry the same prefix, so that feature's `SaveDto` is exported as `MobileHandlingManifestSaveDto`. Types generated from an entity take only the `Mobile` prefix, with no feature name: `MobileTaskDto`, `MobileCreateTaskDto`.

Feature use cases follow one of five patterns depending on their properties:

| Pattern | Properties | Signature |
|---------|-----------|-----------|
| 1 | non-undoable, has DTO in | `fn {feature}_{uc}(dto) -> Result` |
| 2 | non-undoable, no DTO in | `fn {feature}_{uc}() -> Result` |
| 3 | undoable, has DTO in | `fn {feature}_{uc}(stack_id, dto) -> Result` |
| 4 | undoable, no DTO in | `fn {feature}_{uc}(stack_id) -> Result` |
| 5 | long operation | `start_{feature}_{uc}()`, `get_{feature}_{uc}_progress()`, `get_{feature}_{uc}_result()` |

Long operations can be cancelled with `cancel_operation(operation_id)`.

## Event System

### Event Listener

Register a callback to receive all events:

```swift
class MyListener: MobileEventListener {
    func onEvent(event: MobileEvent) {
        switch event.kind {
        case .itemCreated, .itemUpdated:
            refreshItems()
        case .undoPerformed, .redoPerformed:
            refreshAll()
        default: break
        }
    }
}
backend.setEventListener(listener: MyListener())
```

### Auto-Save Listener

Register a callback that fires on any entity mutation (create, update, remove). The typical pattern is to call a custom `Save` feature use case that serializes the in-memory state to disk (e.g. writing the manifest YAML, exporting to a file, or persisting to a platform store).

Since `on_save_needed` fires on every single mutation, **debouncing is essential**: you don't want to write to disk on every keystroke. A common approach is to set a dirty flag and flush after a short delay (e.g. 500ms to 2s of inactivity), or on app backgrounding.

```swift
class AutoSaver: MobileAutoSaveListener {
    private var saveWorkItem: DispatchWorkItem?
    private let backend: MobileBackend

    init(backend: MobileBackend) { self.backend = backend }

    func onSaveNeeded() {
        // Called on background thread: debounce before saving
        saveWorkItem?.cancel()
        let work = DispatchWorkItem { [weak self] in
            guard let self else { return }
            // Call your custom "Save" feature use case
            try? self.backend.handlingManifestSave(dto: MobileHandlingManifestSaveDto(manifestPath: currentManifestPath))
        }
        saveWorkItem = work
        DispatchQueue.global().asyncAfter(deadline: .now() + 1.0, execute: work)
    }
}
backend.setAutoSaveListener(listener: AutoSaver(backend: backend))
```

```kotlin
class AutoSaver(
    private val backend: MobileBackend,
    private val scope: CoroutineScope
) : MobileAutoSaveListener {
    private var saveJob: Job? = null

    override fun onSaveNeeded() {
        // Called on background thread: debounce before saving
        saveJob?.cancel()
        saveJob = scope.launch {
            delay(1000)
            withContext(Dispatchers.IO) {
                // Call your custom "Save" feature use case
                runCatching { backend.handlingManifestSave(MobileHandlingManifestSaveDto(manifestPath = currentManifestPath)) }
            }
        }
    }
}
backend.setAutoSaveListener(AutoSaver(backend, lifecycleScope))
```

The `Save` use case itself is a custom feature you define in your manifest. Qleany generates the controller, DTO, and UoW scaffolding. The implementation typically serializes entities back to whatever format your app uses (YAML manifest, JSON file, SQLite, etc.).

### Sole Consumer Constraint

Flume channels are MPMC (competing consumers). The `MobileBackend` takes the only receiver from the `EventHub`. Do **not** instantiate `EventHubClient` or call `start_event_loop()` alongside `MobileBackend`.

## Custom Types

### MobileDateTime

`chrono::DateTime<Utc>` is mapped to platform-native types via UniFFI:
- Swift: `Date` (via `timeIntervalSince1970`)
- Kotlin: `java.time.Instant` (via `toEpochMilli`)

### MobileError

All fallible methods return `MobileError`:
- `OperationFailed { message }`: wraps `anyhow::Error`
- `NotFound { entity, id }`: entity not found

## Async Wrappers

Every method that touches the database has a generated async variant. Feature use case async methods include the feature prefix in camelCase:
- Swift: `{featureUc}Async()` methods using `Task.detached` (e.g., `handlingManifestSaveAsync()`)
- Kotlin: `{featureUc}Async()` suspend functions using `withContext(Dispatchers.IO)` (e.g., `handlingManifestSaveAsync()`)

Lightweight methods (`can_undo`, `can_redo`, `get_stack_size`, `shutdown`, `cancel_operation`) stay synchronous. They are safe to call from the UI thread.

## Testing

Integration tests run without a device or emulator:

```bash
# Rust tests
cargo test -p mobile_bridge

# Swift tests (requires Xcode)
swift test

# Kotlin tests (requires Gradle)
./gradlew test
```

## In-Memory Constraint

The entire dataset must fit in memory. This is appropriate for document-oriented apps under ~100MB working set. For larger datasets, consider pagination via `get_*_relationship_in_range`.
