![PyPI - Version](https://img.shields.io/pypi/v/qleany)
[![license](https://img.shields.io/badge/license-Mozilla_Public_License--2.0-blue?style=flat-square)](#license)
![quality](https://img.shields.io/github/actions/workflow/status/ferntech-eu/qleany/ci.yml)
![GitHub commit activity](https://img.shields.io/github/commit-activity/y/ferntech-eu/qleany)

# Qleany

**Define your entities in YAML. Get a complete, tested architecture in C++20/Qt6 or Rust: controllers, repositories, undo/redo, reactive models, and ready-to-compile UIs.**

> **No framework. No runtime. No Qleany dependencies in your code.**
>
> The generated code is yours: plain C++ classes and Rust structs using standard libraries (Qt, QCoro for C++). Modify it, extend it, delete Qleany afterward. You're not adopting a framework that will haunt your codebase for years or burn you when the maintainer moves on: because the generated code carries no Qleany dependency at all.

## Try it now

Run:
`pipx install qleany && qleany demo`

And follow the instructions.

Deps: `clang-format` for C++/Qt, or `cargo-fmt` for Rust.

Or type `qleany` to launch the GUI and click on the green "Run demo" button. 

![Screenshot](docs/screenshot_1.png)

## Intro

Qt provides excellent widgets and signals, but little guidance on organizing a 30,000-line application. Rust's GUI ecosystem is growing fast, but there's nothing to help you structure what sits behind the UI. Qleany fills that gap. Write a YAML manifest describing your entities, relationships, and features. Qleany generates the rest: the database layer, the repository infrastructure, the event system, the controller wiring, and — if you need it — a multi-stack undo/redo system with cascade snapshot/restore for entity trees. For C++/Qt, it also generates reactive QML models that update themselves, and JavaScript mock controllers so your UI developer can work without waiting for the backend.

Once the code is generated, your work is two things: fill in the use case bodies where the TODOs are, and build your UI. The rest is done. Think of it as getting the framework without adopting one.

Qleany is not a scaffolding tool, it's an architecture materializer.

For a 17-entity project, that's roughly 410 files in C++/Qt or 175 in Rust, all compiling, all internally consistent, with a generated test suite that validates the infrastructure before you write a single line of business logic. The generated code is deliberately straightforward, readable and modifiable by a developer with a few years of experience, not a showcase of advanced language features.

Qleany follows Package by Feature (Vertical Slice Architecture) principles. Define your entities and features once, generate consistent architecture across Rust and C++/Qt with baked-in (empty) UIs. Qleany's own Slint-based tool is built using the same patterns it generates.

## Key Features

- **Complete CRUD infrastructure**: Controllers, DTOs, use cases, repositories per entity
- **Undo/redo system** (optional): Command-based with multi-stack scoping, composite grouping, and failure strategies; async execution with QCoro coroutines in C++/Qt, synchronous in Rust; cascade snapshot/restore for entity trees
- **GUI skeleton generation**: Ready-to-compile frontend code for QtQuick, QtWidgets, Slint, or CLI
- **Mobile bridge**: UniFFI-based iOS (Swift) and Android (Kotlin) support with generated async wrappers, event callbacks, and platform READMEs
- **Models**: C++/Qt only: auto-updating list models and single-entity wrappers with event-driven refresh
- **Reactive QML models**: Same than above, but with QML integration, so they can be used directly in the UI without manual wiring (C++/Qt)
- **QML mocks**: JavaScript stubs that simulate async behavior, enabling UI development without a backend (C++/Qt)
- **Relationship management**: Uniform junction tables with ordering, two-layer caching, bidirectional navigation, and cascade deletion
- **Event system**: Thread-safe, decoupled communication between features
- **Event Buffer**: Send events only if the command succeeds
- **Generated test suite**: Junction table operations, undo/redo behavior, and async integration tests

## AI ready

Qleany generates a consistent, well-structured codebase that coding LLMs can navigate without guesswork and ships two CLI commands to make that easy:

- `qleany docs all --md > qleany_docs.md`: full reference as a single Markdown file you can drop into your LLM's context. Narrow it with `qleany docs manifest`, `qleany docs undo-redo-architecture`, etc.
- `qleany prompt`: generate contextualized, guardrailed prompts for coding tasks against your manifest:

```bash
qleany prompt --context              # project context for .claude/CLAUDE.md and friends
qleany prompt --list                 # list use cases (and which are unimplemented)
qleany prompt --use-case feature:my_feature_name
```

Not interested in LLMs? Ignore this section — `prompt` just prints text, and nothing else in Qleany depends on it.

## Documentation

| Document                                                           | Purpose                                                                        |
|--------------------------------------------------------------------|--------------------------------------------------------------------------------|
| [Quick Start - Rust](docs/quick-start-rust.md)                     | Step-by-step tutorial building a complete application                          |
| [Quick Start - C++/Qt](docs/quick-start-cpp-qt.md)                 | Step-by-step tutorial building a complete application                          |
| [Manifest Reference](docs/manifest-reference.md)                   | Entity options, field types, relationships, features and use cases             |
| [Design Philosophy](docs/design-philosophy.md)                     | Clean Architecture background, package by feature, Rust module structure       |
| [How Operations Flow](docs/how-operations-flow.md)                 | How data flows through the application, events                                 |
| [Regeneration Workflow](docs/regeneration-workflow.md)             | How file generation works, what gets overwritten, files that must stay in sync |
| [Undo-Redo Architecture](docs/undo-redo-architecture.md)           | Entity tree structure, undoable vs non-undoable, configuration patterns        |
| [Migration Guide](docs/migration-guide.md)                         | Manifest changes and migration strategies                                      |
| [QML Integration](docs/qml-integration.md)                         | Reactive models, mocks, and event system for C++/Qt                            |
| [Generated Infrastructure - C++/Qt](docs/generated-code-cpp-qt.md) | Database layer, repositories, and file organization details                    |
| [Generated Infrastructure - Rust](docs/generated-code-rust.md)     | Database layer, repositories, and file organization details                    |
| [API Reference - C++/Qt](docs/api-reference-cpp-qt.md)             | Entity controller, feature controller, and custom UoW macros                   |
| [API Reference - Rust](docs/api-reference-rust.md)                  | Entity controller, feature controller, and custom UoW proc macros              |
| [Mobile Bridge Development](docs/mobile-bridge-development.md)     | UniFFI bridge for iOS (Swift) and Android (Kotlin)                             |
| [Troubleshooting](docs/troubleshooting.md)                         | Common issues and how to fix them                                              |

New to Qleany? Start with the [Quick Start Guide - C++/Qt](docs/quick-start-cpp-qt.md) or [Quick Start Guide - Rust](docs/quick-start-rust.md). Then return here for reference.


---

## Is Qleany the Right Fit?

### When Qleany Makes Sense

**Data-centric applications** that will grow in complexity over time. Think document editors, project management tools, creative applications, or anything where users manipulate structured data and expect undo/redo to work reliably. This applies equally to desktop and mobile: a note-taking app on Plasma Mobile has the same architectural needs as one on desktop Linux.

**Complex CLI tools in Rust**: tools like `git` that manage structured data, have multiple subcommands, and need consistent internal architecture. Qleany itself is built this way: type `qleany -h` to see a CLI interface backed by the same architecture that powers its Slint GUI.

**Applications targeting multiple platforms**: if you're building for desktop Linux and want to support Plasma Mobile or Ubuntu Touch with the same codebase, Qleany's generated backend works identically across all of them. Write your business logic once, swap UI frontends as needed.

**Mobile apps with a Rust backend**: if you are targeting iOS (Swift) or Android (Kotlin), enable `rust_ios` and/or `rust_android` in your manifest. Qleany generates a complete UniFFI bridge with platform-native async wrappers. See the [Mobile Bridge Development](docs/mobile-bridge-development.md) guide. This solution is especially valuable if you want to share the backend between desktop and mobile.

**Applications needing multiple Qt frontends**: if you need QtQuick, QtWidgets (or any combination of them simultaneously), Qleany generates a ready-to-compile backend architecture that any of these frontends can consume. The generated controllers, repositories, and event system work identically regardless of which UI toolkit you choose.

**Solo developers or small teams** without established architectural patterns. Qt provides excellent widgets and signals, but little guidance on organizing a 30,000-line application (or I couldn't find it). Qleany gives you that structure immediately, with patterns validated through real-world use in Skribisto.

**Projects that will grow incrementally**: the manifest-driven approach means you can define a new entity, regenerate the architecture, and immediately have a working controller, repository, DTOs, and use cases. The consistency this brings across your codebase is hard to achieve manually.

### When to Reconsider

For **simple utilities or single-purpose tools**, Qleany introduces more infrastructure than you need. If your application doesn't have complex entity relationships, doesn't need undo/redo, and won't grow significantly, a hand-written architecture may serve you better. Yet since you only have to fill a few blank spots, using Qleany can save you time.

If you're working with a **team that already has established patterns**, introducing Qleany means everyone needs to learn its conventions. The generated code is readable and follows clear patterns, but it represents a specific way of doing things. Discuss with your team before adopting it. Do not antagonize existing workflows. A more professional approach may be to present Qleany's patterns with some open-minded senior devs of your team. Even if they don't want to use Qleany - **which is fairly expected** - they may appreciate some of its ideas and adapt them to their existing architecture. They may even want to use Qleany for prototyping or side projects, or scaffold new subsystems of an existing project without disrupting the main architecture.

Qleany **targets native applications**. If you're building for the web, using Electron, this isn't the right tool. Similarly, if you need high-throughput server-side processing, the patterns here are optimized for user interaction, not request-per-second performance. 

### Special Considerations

For Flutter or React Native, the Rust backend option can still be an interesting choice, but the C++/Qt generation is not recommended.

You can also have a Rust backend and a C++/Qt frontend in the same codebase, using cxx-qt as a bridge. This is only an idea. Qleany does not support this.

### The Practical Test

If your project matches the profile, start by **generating the architecture for a small subset of your entities** and spend time reading through the generated code. Understand how the controllers wire to use cases, how the event system propagates changes, how the undo commands work. This investment of a few hours will tell you whether the patterns feel natural to your way of thinking.

The "generate and disappear" philosophy means you're not locked in. If you decide halfway through that you'd prefer a different approach, the generated code is yours to modify or replace.

---

## Why Qleany

I wrote [Skribisto](https://github.com/jacquetc/skribisto), a novel-writing application in Qt. Four times. In different languages. Each time, I hit the same wall: spaghetti code and structural dead-ends that made adding features painful and eventually impossible without rewriting half the codebase.

After the third rewrite, I studied architecture patterns seriously. Clean Architecture (Robert C. Martin) clicked: the separation of concerns, the dependency rules, the testability. But implementing it by hand meant writing the same boilerplate over and over: repositories, DTOs, use cases, controllers. So I wrote templates. The templates grew into a generator. The generator needed a manifest file.

**Qleany v0** was Python/Jinja2 generating C++/Qt code following pure Clean Architecture. It worked, but the tradeoffs were hard to miss: a 17-entity project produced 1700+ files across 500 folders. Some of my early design choices were dubious in hindsight.

**Qleany v1** is a ground-up rewrite in Rust, aiming to fix those problems while adopting a more robust and easier-to-maintain language. Less sophisticated, more pragmatic, architecture. It adopts Package by Feature (a.k.a. Vertical Slice Architecture) instead of strict layer separation. Same Clean Architecture principles (SOLID), but organized by what the code does rather than what layer it belongs to. The same manifest now generates both C++/Qt and Rust code.

This is the tool I needed when I started Skribisto. If it saves someone else from their fourth rewrite, it's done its job.

---

## Target Platforms

| Language | Standard    | internal database | Frontend Options                  |
|----------|-------------|-------------------|-----------------------------------|
| C++      | C++20 / Qt6 | SQLite            | QtQuick, QtWidgets                |
| Rust     | Rust 2024   | in-memory HashMap | CLI, Slint, iOS (Swift), Android (Kotlin) |

**Supported deployment targets for C++/Qt:**
- Desktop Linux (KDE Plasma, GNOME, etc.)
- Windows, macOS (Qt's cross-platform support)
- Mobile (Android, iOS) via Qt's mobile support

**Supported deployment targets for Rust:**
- All the usual Rust targets (Linux, Windows, macOS, etc.)
- iOS and Android via the generated `mobile_bridge` crate (UniFFI)

The generated backend is platform-agnostic. Your business logic, repositories, and controllers work identically whether you're building a desktop app, a mobile app, or both from the same codebase. Only the UI layer differs.

Also, the internal storage choice (SQLite for C++/Qt, in-memory HashMap store for Rust) is abstracted behind repositories. You can swap out the storage implementation if needed.

**Rust frontend examples:**
- **Slint UI**: [qleany/crates/slint_ui](https://github.com/ferntech-eu/qleany/tree/generator_in_rust/crates/slint_ui) (Qleany's current GUI frontend)
- **Tauri/React**: [qleany/crates/qleany-app](https://github.com/ferntech-eu/qleany/tree/885c3ac6fdf6f115aed2c5a30fd26b81e331b4dd/crates/qleany-app) (abandoned prototypes but still working references)

I'm no web developer, and Tauri/React is not my forte. But if you want to build a web-based frontend with Rust backend generated by Qleany, this is a starting point.

---

## Where to Get Qleany

| Source | Status                                                                                     |
|--------|--------------------------------------------------------------------------------------------|
| GitHub Releases | [See here](https://github.com/ferntech-eu/qleany/releases/latest)                             |
| Cargo | `cargo install --git https://github.com/ferntech-eu/qleany qleany` or `cargo binstall qleany` |
| PyPI (pip) with pipx | `pipx install qleany`  from [Pypi](https://pypi.org/project/qleany/)                       |

Or build from source (see below).

---

## License

Qleany (the generator) is licensed under MPL-2.0. See the [LICENSE](LICENSE) file for details. It is compatible with
both open source and proprietary projects.

**Generated code**: This license does not cover the code generated by Qleany. You are free to use, modify,
and distribute generated code under any license of your choice, including proprietary licenses.

For more details, see this fine [summary](https://choosealicense.com/licenses/mpl-2.0/)

---

## Building and Running

### Prerequisites

- Rust (install via [rustup](https://rustup.rs/))

### Building Qleany

```bash
git clone https://github.com/ferntech-eu/qleany
cd qleany
cargo build --release
```

### Running the UI

```bash
cargo run --release
```

The Slint-based UI provides:
- Form-based manifest editing
- Entity and relationship management
- Feature/use case orchestration
- Selective file generation
- Code preview and diff before generation

For more details, see the [Quick Start Guide - C++/Qt](docs/quick-start-cpp-qt.md) or [Quick Start Guide - Rust](docs/quick-start-rust.md).

### CLI Usage

```bash

# Show help
qleany -h

# Create, generate a demo project in a qleany-demo folder
qleany demo

# Show an option help
qleany generate -h

# show the list of available documentation
qleany docs -h

# show all documentation in Markdown format
qleany docs all --md

# new qleany.yaml manifest, questions will guide you through the process
qleany new

# Generate modified and new files (default)
qleany generate (or gen)

# Generate to ./temp/ folder (recommended)
qleany generate --temp

# Dry run (list files that would be generated without writing)
qleany generate --dry-run

# Dry run for a specific entity
qleany generate --dry-run entity MyEntity

# Generate specific feature
qleany generate feature my_feature_name

# List files that would be generated  (only modified and new by default)
qleany list

# Filter by status
qleany list --modified          # only modified files (-M)
qleany list --new               # only new files (-N)
qleany list --unchanged         # only unchanged files (-U)
qleany list --all-status        # all statuses

# Filter by nature
qleany list --infra             # infrastructure files only (-i)
qleany list --aggregates        # aggregate files only (-g)
qleany list --scaffolds         # scaffold files only (-s)
qleany list --all-natures       # all natures

# Show everything
qleany list --all               # equivalent to --all-status --all-natures

# List features that would be generated
qleany list features

# Verify the manifest
qleany check

# Display the enforced rules
qleany check --rules

# Diff the generated code for a file against the existing code
qleany diff file_path/file.rs

# Create a context for LLM-based code generation
qleany prompt --context

# List the use cases and the unimplemented ones
qleany prompt --list

# generate a task prompt tailored for a use case
qleany prompt --use-case feature:my_feature_name 
```

---

## Reference Implementation

[Skribisto](https://github.com/jacquetc/skribisto) (develop branch) is a novel-writing application built with Qleany-generated C++/Qt code. It demonstrates:

- Full entity hierarchy (Root → Work → Binder → BinderItem → Content)
- Complex relationships (ordered children, many-to-many tags)
- Feature orchestration (LoadWork, SaveWork with file format transformation)
- Reactive QML UI with auto-updating models
- Undo/redo across structural and content operations

Skribisto serves as both proof-of-concept and template source for C++/Qt generation.

---

## Migration from v0

Qleany v0 (Python/Jinja2), the prototype, generated pure Clean Architecture with strict layer separation. A 17-entity project produced 1700+ files across 500 folders.

v1 generates Package by Feature with pragmatic organization. The same project produces ~600 files across ~80 folders with better discoverability. Its manifest version begins with version 2.

**Breaking changes:**
- Manifest format changed (schema version 2)
- Output structure reorganized by feature
- Reactive models are new (list models, singles)

Bottom line: from v0 to v1, there is no automated migration path. You must regenerate from your manifest and manually port any custom code.

Starting from the newer version 2 of the manifest (i.e., Qleany v1), the new architecture offers a smoother transition to future versions, doing it automatically at load time, or using `qleany upgrade`.

See the [Migration Guide](docs/migration-guide.md) for more details.

---

## Contributing

To contribute:
1. Open an issue to discuss changes
2. Ensure changes work for both Rust and C++/Qt
3. Remember to sign off your commits (`commit -s`)

Please read the [CONTRIBUTING.md](CONTRIBUTING.md) file.

## Support

**GitHub Issues** is the only support channel: [github.com/ferntech-eu/qleany/issues](https://github.com/ferntech-eu/qleany/issues)

Qleany is a project licensed under MPL-2.0. It is actively used in Skribisto's development, among other projects from FernTech, so improvements flow from real-world needs. Bug reports and contributions are welcome.

FernTech offers professional support for Qleany. 

Contact: cyril.jacquet@ferntech.eu

## AI use

Qleany isn't a tool made by AI. It is a human-driven tool using templates and smart (I can hope) algorithms.

In this era where too much code comes from AI, and too much "slop" code, I feel that I must be honest with my use of these semi-smart tools. I am an IT professional for 14 years (started in 2011). I was Linux sysadmin, DevOps engineer, and now a senior C++ and Rust developer/tech lead/architect. I learned my trade before all this AI stuff. For me, LLMs are capricious smart tools. My take: never trust a LLM, always check the answers because they tend to trick you, LLMs never learn from their mistakes, unlike a human. It's a tool, not a crutch.

In Qleany, AI was used in only three cases:
- basic auto-completion, thanks to the LLM integrated into JetBrains IDEs (especially in very repetitive patterns), less "magical" than GitHub Copilot, but still helpful.
- English sentences in the documentation were smoothed with the AI, nothing more. And it helped to create tables with all these asterisks. I wrote this documentation.
- The AI added some comments and inline documentation, especially in the C++ undo redo system. It was fun.

That's it. I honestly feel that Qleany is the work of a human being (me), not a machine.

That being said, while Qleany wasn't born as an AI tool, today's reality is that LLMs are here to stay, and they can be useful for developers. So I made sure that Qleany's CLI has a `prompt` command that generates contextualized prompts for coding tasks, based on the manifest and the current state of the codebase. This way, you can use Qleany to get better results from your coding LLMs, with safer guardrails and more relevant prompts.


## About

Qleany is developed and maintained by FernTech.

Copyright (c) 2025-2026 FernTech
Licensed under [MPL-2.0](LICENSE)
