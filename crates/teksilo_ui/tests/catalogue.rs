//! The message catalogue and the code that reads it, kept in step.
//!
//! `tr!` validates at compile time that a key it names exists, so a missing key
//! cannot ship. Nothing checks the other direction: a key nobody reads compiles
//! perfectly and quietly becomes a lie about what the application says. Fifteen had
//! collected by the time this was written, including two `&Undo` rows superseded by
//! two others, and four `MessageBox` button labels that teksilo's own presets
//! supply.
//!
//! This walks the source rather than the binary, so it is a text check, which is
//! all it can be: `tr!(key())` is a macro call, and by the time anything is
//! reflectable the key is gone.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn catalogue() -> String {
    std::fs::read_to_string(crate_dir().join("locales/en-US/main.ftl"))
        .expect("the en-US catalogue")
}

/// Every message key the catalogue defines, with the line it is on.
///
/// A key is a line starting in the first column with an identifier and an `=`. An
/// attribute line starts with a dot and a continuation with whitespace, so neither
/// is mistaken for one.
fn keys(source: &str) -> Vec<(String, usize)> {
    source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            if line.starts_with([' ', '\t', '#', '.', '[']) {
                return None;
            }
            let (name, _) = line.split_once('=')?;
            let name = name.trim_end();
            let is_key = !name.is_empty()
                && name.starts_with(|c: char| c.is_ascii_lowercase())
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            is_key.then(|| (name.to_string(), index + 1))
        })
        .collect()
}

/// Every `.rs` file under `src/`, concatenated.
fn sources() -> String {
    fn walk(dir: &Path, out: &mut String) {
        for entry in std::fs::read_dir(dir).expect("readable source directory") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push_str(&std::fs::read_to_string(&path).expect("readable source"));
                out.push('\n');
            }
        }
    }
    let mut out = String::new();
    walk(&crate_dir().join("src"), &mut out);
    out
}

/// Every key is read by something.
#[test]
fn the_catalogue_carries_nothing_the_application_never_says() {
    let source = sources();
    let unused: Vec<String> = keys(&catalogue())
        .into_iter()
        .filter(|(key, _)| !source.contains(&format!("tr!({}(", key.replace('-', "_"))))
        .map(|(key, line)| format!("{key} (line {line})"))
        .collect();
    assert!(
        unused.is_empty(),
        "these keys are defined and never read; delete them or use them:\n  {}",
        unused.join("\n  ")
    );
}

/// No key is defined twice.
///
/// Fluent takes the first and drops the rest with a parse warning nothing surfaces,
/// so a second definition is an edit that silently does nothing.
#[test]
fn no_key_is_defined_twice() {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut duplicates = Vec::new();
    for (key, line) in keys(&catalogue()) {
        if let Some(first) = seen.get(&key) {
            duplicates.push(format!("{key} (lines {first} and {line})"));
        } else {
            seen.insert(key, line);
        }
    }
    assert!(
        duplicates.is_empty(),
        "duplicate keys:\n  {}",
        duplicates.join("\n  ")
    );
}

/// The catalogue is not empty, which is the failure mode that would make both
/// tests above pass without checking anything.
#[test]
fn the_catalogue_is_found_and_read() {
    assert!(
        keys(&catalogue()).len() > 200,
        "the catalogue should hold every string the application says"
    );
}
