# Contributing to Qleany

Thank you for your interest in contributing to Qleany! This document provides guidelines and information for contributors.

## Code of Conduct

Please be respectful and constructive in all interactions. We aim to maintain a welcoming environment for everyone.

## How to Contribute

### Reporting Issues

- Check existing issues before creating a new one
- Provide a clear description of the problem
- Include steps to reproduce, expected behavior, and actual behavior
- Mention your environment (OS, Rust version, etc.)

### Suggesting Features

- Open an issue describing the feature and its use case
- Explain why this would be valuable for Qleany users
- Be open to discussion about alternative approaches

### Submitting Code

1. Fork the repository
2. Create a feature branch from `main`
3. Make your changes
4. Ensure your code follows the project's style
5. Test your changes
6. Submit a pull request

## Regenerating Qleany's own backend

Qleany generates itself from the `qleany.yaml` at the repo root, so a template
change can be applied to this repo too. That is worth doing — it is how the
generator dogfoods its own output — but a blanket `qleany generate` **destroys
this repository**, and the reasons are not obvious.

### Never run a bare `qleany generate` here

Every generated file in this repo reports as `[M]` modified, because the header
records the generator version and that moves with every release. `generate`
defaults to modified + new, so a bare run rewrites essentially everything —
including the 45 **Scaffold** files, which hold every hand-written use-case body
in the project. Use `generate file <path>`, a nature filter, or the read-only
`generate --all --temp`.

### What must never be regenerated

| Path | Why |
|------|-----|
| `crates/*/src/use_cases/*_uc.rs`, `*/units_of_work/*_uow.rs` | Scaffold. These are Qleany's implementation; the generated form is `unimplemented!()`. |
| `Cargo.toml` (root) | Marked Aggregate, but hand-maintained: the generated form resets the version to `0.0.1`, the licence to `MIT OR Apache-2.0`, and drops the real package names and crates.io metadata. |
| `crates/*/Cargo.toml` | Same: the generated form carries only the skeleton dependencies, dropping `tera`, `rayon`, `similar`, `heck`, `include_dir` and the rest. |

### What must be re-applied after regenerating

These live inside generated files, so regeneration drops them. After a regen,
check each one is still present:

| File | Hand-written addition |
|------|-----------------------|
| `crates/common/src/lib.rs` | `pub mod enum_variant_parser;` · `pub mod generator;` |
| `crates/handling_manifest/src/lib.rs` | `#![recursion_limit = "256"]` — the JSON-schema literal needs it |
| `crates/{handling_manifest,rust_file_generation,cpp_qt_file_generation,file_generation_shared_steps}/src/use_cases.rs` | `mod common;` — each feature's shared helpers |
| `crates/handling_manifest/src/dtos.rs` | `CheckRuleDto` |
| `crates/handling_manifest/src/handling_manifest_controller.rs` | `get_check_rules()` and its `use crate::CheckRuleDto;` |
| `crates/{rust,cpp_qt}_file_generation/src/*_controller.rs` | `let uc = Generate{Rust,CppQt}CodeUseCase::new(…)` without `mut`: those two use cases are hand-tightened to `execute(&self)`, which the template cannot know |

### The procedure

```bash
git commit                                   # non-negotiable; regen overwrites
cargo build --workspace                      # build the generator you will run

qleany generate --all --temp                 # read-only: writes only ./temp/
diff -r temp/crates crates | less            # review before writing anything

qleany generate -M -i                        # modified Infrastructure
qleany generate -M -g                        # modified Aggregate
git checkout -- Cargo.toml crates/*/Cargo.toml   # undo the manifest clobber
# …re-apply the table above…
cargo fmt --all
cargo check --workspace && cargo test --workspace && ./run_tests.sh
```

Finally, confirm it is a fixed point — regenerate a second time and expect no
diff. If the second pass changes anything, a template is not stable and that is
a bug worth fixing before landing.

## Developer Certificate of Origin

This project uses the [Developer Certificate of Origin (DCO)](DCO.md).

By contributing to this repository, you agree to the DCO. You **must sign off your commits** to indicate your agreement:

```bash
git commit -s -m "Your commit message"
```

This adds a `Signed-off-by: Your Name <your.email@example.com>` line to your commit, certifying that you wrote or have the right to submit the code under the project's license (MPL-2.0).

### Setting up automatic sign-off

You can configure Git to always set your identity for your commits for this repository:

```bash
git config user.name "Your Name"
git config user.email "your.email@example.com"
```

Then use `git commit -s` for each commit, or create a Git alias:

```bash
git config --global alias.cs "commit -s"
```

### What if I forgot to sign off?

You can amend your last commit:

```bash
git commit --amend -s
```

For multiple commits, you may need to rebase:

```bash
git rebase --signoff HEAD~N
```

(Replace `N` with the number of commits to sign off)

## License

By contributing to Qleany, you agree that your contributions will be licensed under the [Mozilla Public License 2.0](LICENSE).

## Questions?

If you have questions about contributing, feel free to open an issue for discussion.
