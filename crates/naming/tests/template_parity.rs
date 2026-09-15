//! Keeps the generated projects' copy of the pluralization rules in sync.
//!
//! `crates/naming` is the source of truth, but it cannot be the only copy:
//! `macros_direct_access.tera` emits the same rules into every generated
//! project's `crates/macros` proc-macro crate, and that crate is standalone --
//! it cannot depend on `qleany-naming`. So the code is duplicated on purpose.
//!
//! Both copies are delimited by the same pair of marker comments and are meant
//! to be character-for-character identical, so this test is a plain string
//! comparison rather than a set-of-words comparison. That matters: an earlier
//! version compared only the rule *tables*, which meant deleting a whole rule
//! branch -- or adding a new helper function, as `last_word_start` later was --
//! left the test green while the two copies genuinely disagreed.

use std::fs;
use std::path::{Path, PathBuf};

/// The source of truth, relative to the repository root.
const LIB_RELATIVE: &str = "crates/naming/src/lib.rs";

/// The generated projects' copy, relative to the repository root.
const TEMPLATE_RELATIVE: &str =
    "crates/rust_file_generation/src/use_cases/common/templates/macros/macros_direct_access.tera";

const BEGIN: &str = "// --- BEGIN canonical pluralizer ---";
const END: &str = "// --- END canonical pluralizer ---";

#[test]
fn the_generated_projects_copy_is_character_identical() {
    let lib = read_source(LIB_RELATIVE);
    let template = read_source(TEMPLATE_RELATIVE);

    let authoritative = region(&lib, LIB_RELATIVE);
    let copied = region(&template, TEMPLATE_RELATIVE);

    if authoritative == copied {
        return;
    }

    let mut report = String::from(
        "The canonical pluralizer has drifted between its two copies.\n\n\
         Source of truth : ",
    );
    report.push_str(LIB_RELATIVE);
    report.push_str("\nGenerated copy  : ");
    report.push_str(TEMPLATE_RELATIVE);
    report.push_str(
        "\n\nEvery generated project ships the second copy inside its own proc-macro\n\
         crate, which cannot depend on `qleany-naming`. If the two disagree, a\n\
         generated project's macros will name a collection differently from the\n\
         store field the templates declare, and it will not compile.\n\n\
         Fix: edit the block in the source of truth, then copy it across verbatim,\n\
         marker comments included.\n\n\
         First differing lines:\n",
    );

    let mine: Vec<&str> = authoritative.lines().collect();
    let theirs: Vec<&str> = copied.lines().collect();
    let mut shown = 0;
    for index in 0..mine.len().max(theirs.len()) {
        let a = mine.get(index).copied();
        let b = theirs.get(index).copied();
        if a == b {
            continue;
        }
        report.push_str(&format!(
            "\n  line {}:\n    truth    : {}\n    template : {}\n",
            index + 1,
            a.unwrap_or("<missing>"),
            b.unwrap_or("<missing>"),
        ));
        shown += 1;
        if shown == 5 {
            report.push_str("\n  ... further differences suppressed.\n");
            break;
        }
    }

    panic!("{report}");
}

/// Reads a file that must exist, relative to the repository root.
///
/// A missing file means the layout moved; failing loudly here is the point,
/// because a silently-skipped parity test is worse than no parity test.
fn read_source(relative: &str) -> String {
    let path = repository_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "cannot read {}: {error}. If this file moved, update \
             crates/naming/tests/template_parity.rs to match.",
            path.display()
        )
    })
}

/// Extracts the canonical block, markers included.
fn region<'a>(source: &'a str, origin: &str) -> &'a str {
    let start = source.find(BEGIN).unwrap_or_else(|| {
        panic!("{origin} no longer contains the marker line `{BEGIN}`");
    });
    let end = source.find(END).unwrap_or_else(|| {
        panic!("{origin} no longer contains the marker line `{END}`");
    });
    assert!(
        start < end,
        "{origin} has the canonical pluralizer's markers in the wrong order"
    );
    &source[start..end + END.len()]
}

/// `crates/naming` -> the repository root.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("CARGO_MANIFEST_DIR should be <root>/crates/naming")
        .to_path_buf()
}
