use crate::cli::{DocsArgs, DocsTarget, OutputContext};
use frontend::AppContext;
use include_dir::{Dir, include_dir};
use std::rc::Rc;

static DOCS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/docs/");

static README: &str = include_str!("../../README.md");

fn print_md(content: &str, raw: bool) {
    if raw {
        println!("{}", content);
    } else {
        let skin = termimad::MadSkin::default();
        skin.print_text(content);
    }
    println!();
}

pub fn execute(
    _app_context: &Rc<AppContext>,
    args: &DocsArgs,
    output: &OutputContext,
) -> anyhow::Result<()> {
    let raw = args.md;

    // No target → show help listing and return
    let target = match args.target.as_ref() {
        Some(t) => t.clone(),
        None => {
            if !output.quiet {
                let listing = "\
**Available documentation commands:**

|:-|:-|:-|
|`qleany docs all`||All documentations|
|`qleany docs introduction`|`intro`|Introduction (README)|
|`qleany docs quick-start-rust`|`start-rust`|Quick Start — Rust|
|`qleany docs quick-start-cpp-qt`|`start-cpp`|Quick Start — C++/Qt|
|`qleany docs manifest-reference`|`manifest`|Manifest Reference|
|`qleany docs design-philosophy`|`design`|Design Philosophy|
|`qleany docs how-operations-flow`|`flow`|How Operations Flow|
|`qleany docs regeneration-workflow`|`regen`|Regeneration Workflow|
|`qleany docs undo-redo-architecture`|`undo`|Undo/Redo Architecture|
|`qleany docs qml-integration`|`qml`|QML Integration|
|`qleany docs generated-code-cpp-qt`|`cpp`|Generated Code — C++/Qt|
|`qleany docs generated-code-rust`|`rust`|Generated Code — Rust|
|`qleany docs api-reference-cpp-qt`|`api-cpp`|API Reference — C++/Qt|
|`qleany docs api-reference-rust`|`api-rust`|API Reference — Rust|
|`qleany docs migration-guide`|`mig`|Migration Guide|
|`qleany docs troubleshooting`|`trouble`|Troubleshooting|
|`qleany docs mobile-bridge-development`|`mobile`|Mobile Bridge Development|
|-|-|-|

Use `--md` to output raw Markdown instead of terminal-formatted text.
";
                print_md(listing, raw);
            }
            return Ok(());
        }
    };

    // Handle Intro specially (README.md is not in docs/)
    if matches!(target, DocsTarget::Introduction | DocsTarget::All) {
        if !output.quiet {
            print_md(README, raw);
        }
        if matches!(target, DocsTarget::Introduction) {
            return Ok(());
        }
    }

    let files: Vec<&str> = match target {
        DocsTarget::All => vec![
            "quick-start-rust.md",
            "quick-start-cpp-qt.md",
            "manifest-reference.md",
            "design-philosophy.md",
            "how-operations-flow.md",
            "regeneration-workflow.md",
            "undo-redo-architecture.md",
            "qml-integration.md",
            "generated-code-cpp-qt.md",
            "generated-code-rust.md",
            "api-reference-cpp-qt.md",
            "api-reference-rust.md",
            "migration-guide.md",
            "troubleshooting.md",
            "mobile-bridge-development.md",
        ],
        DocsTarget::Introduction => unreachable!(),
        DocsTarget::ManifestReference => vec!["manifest-reference.md"],
        DocsTarget::DesignPhilosophy => vec!["design-philosophy.md"],
        DocsTarget::HowOperationsFlow => vec!["how-operations-flow.md"],
        DocsTarget::UndoRedoArchitecture => vec!["undo-redo-architecture.md"],
        DocsTarget::GeneratedCodeCppQt => vec!["generated-code-cpp-qt.md"],
        DocsTarget::GeneratedCodeRust => vec!["generated-code-rust.md"],
        DocsTarget::ApiReferenceCppQt => vec!["api-reference-cpp-qt.md"],
        DocsTarget::ApiReferenceRust => vec!["api-reference-rust.md"],
        DocsTarget::QuickStartCppQt => vec!["quick-start-cpp-qt.md"],
        DocsTarget::QuickStartRust => vec!["quick-start-rust.md"],
        DocsTarget::QmlIntegration => vec!["qml-integration.md"],
        DocsTarget::MigrationGuide => vec!["migration-guide.md"],
        DocsTarget::Troubleshooting => vec!["troubleshooting.md"],
        DocsTarget::RegenerationWorkflow => vec!["regeneration-workflow.md"],
        DocsTarget::MobileBridgeDevelopment => vec!["mobile-bridge-development.md"],
    };

    for filename in files {
        match DOCS_DIR.get_file(filename) {
            Some(file) => {
                let content = file.contents_utf8().unwrap_or("[binary content]");
                if !output.quiet {
                    print_md(content, raw);
                }
            }
            None => {
                output.warn(&format!("Documentation file not found: {}", filename));
            }
        }
    }

    Ok(())
}
