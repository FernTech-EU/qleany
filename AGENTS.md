<!-- BEGIN teksilo -->
# Teksilo — read this before writing GUI code in this app

This app depends on **Teksilo**, a pure-Rust desktop GUI framework: a retained
widget tree, SwiftUI-style layout negotiation, `Signal`/`Prop` reactivity,
AccessKit accessibility, wgpu rendering.

`cargo teksilo` is a cargo subcommand that answers for the exact Teksilo
version **this app resolved** — read from its dependency graph, never from
whatever is newest. Install it at that version:

```bash
cargo install cargo-teksilo --version <the teksilo version this app pins>
```

**There is a floor: this tool's first release is 0.13.0.** It is newer than the
framework it serves, so an app on teksilo 0.12 or older has no matching tool —
that version of it was never published, and no `--version 0.12.x` will resolve.
Such an app has to move to teksilo 0.13 first. Until it does, this briefing
describes a command that cannot run here: use `cargo doc -p teksilo-widgets`
and the compiler, and say the version is unserved rather than answering from
memory.

It refuses to answer when its own minor or major differs from the app's
Teksilo, and warns when only the patch differs. That is deliberate:
`SplitView` was deleted outright in favour of `Splitter` between two minors,
and a wrong answer reads exactly like a right one.

## Commands

- `cargo teksilo symbol <Name>`: public API from resolved crate sources.
- `cargo teksilo search "<query>"`: guides and examples; `--json` for structured hits.
- `cargo teksilo show <path>`: full document; `--lines A-B` for a 1-based range.
- `cargo teksilo init --agent codex --agent claude -y`: install the harness and
  selected instructions, creating directories without detection markers.
- `cargo teksilo agent install <agents...>`: instructions only. `--user` supports
  claude, vibe, and opencode. `agent list` shows targets and installation state.
- `cargo teksilo probe install`: install the automation harness in scripts/teksilo_probe/.
- `cargo teksilo model fetch`: explicitly download the search model. Search uses
  BM25 until cached; init and search never download weights.
- `cargo teksilo status --json`: project version and tooling state.

Use `--force` to replace conflicting generated files, `--quiet` to suppress
informational output, and `--verbose` for diagnostics. Shared instructions
outside the managed region are preserved.

## The rule that matters most

**Read a search result with `cargo teksilo show <path>`. Do not fetch it from
GitHub.** The path a hit prints is real in the *framework* repository and
absent from this one, so both tempting moves are wrong: opening it locally
finds nothing, and `blob/main/` — or the published book — serves `main`, which
is a different Teksilo from the one this app pinned. `show` is offline and
version-matched.

## Working in this app

1. **Conceptual question** ("which data model do I want?", "how does
   drag-and-drop escalate to the OS?") → `cargo teksilo search "<question>"`,
   then `cargo teksilo show <path>` to read the hit in full rather than
   working from its snippet.
2. **Before using a type** → `cargo teksilo symbol <Name>`.
3. **Compile:** `cargo check -p <app-crate>`. If it fails twice on the same
   item, re-extract that type's API before a third attempt — the mental model
   is wrong, not the compiler.
4. **Verify behaviour** where it matters: headless widget tests for layout and
   state logic, a probe for "the user clicks this and that happens on screen".

If `cargo teksilo` is not installed, fall back to
`cargo doc -p teksilo-widgets --no-deps --open`, or docs.rs at the pinned
version, and to the compiler. Both are slower and the second is
version-approximate — say so rather than guessing.
<!-- END teksilo -->
