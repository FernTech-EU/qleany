//! What the selected file would contain, or what would change in it.
//!
//! Two panes behind one toggle. The difference is the default, because it is the
//! question a generation run actually asks; the source is there for reading a file
//! whole, which is what a new file needs.

use teksilo::prelude::*;
use teksilo::text_document::TextDocument;
use teksilo::widgets::{LogView, PlainTextEditor};

use crate::generate::GenerateViewModel;

/// A colour per kind of diff line.
///
/// `LogView` classifies line by line, which is exactly the shape a unified diff
/// has: the marker is the first character, and the two file headers are the only
/// lines where that character means something else.
fn diff_colour(line: &str, theme: &Theme) -> Option<Color> {
    if line.starts_with("+++") || line.starts_with("---") {
        // The headers name the two sides. They start with the same characters as an
        // added and a removed line, so they are matched first or every diff opens
        // with one green line and one red one that mean nothing.
        return Some(TextRole::Secondary.resolve(&theme.colors));
    }
    if line.starts_with("@@") {
        return Some(TextRole::Accent.resolve(&theme.colors));
    }
    match line.chars().next() {
        Some('+') => Some(TextRole::Success.resolve(&theme.colors)),
        Some('-') => Some(TextRole::Error.resolve(&theme.colors)),
        _ => None,
    }
}

/// The preview for one file.
///
/// Built fresh each time the selection or the toggle changes, which is what the
/// page's rebuild bindings arrange: both panes read a whole file into a document,
/// and keeping two documents alive per file for a list of several hundred is the
/// memory this screen is careful about everywhere else.
pub fn preview(vm: &GenerateViewModel, ctx: &BuildContext) -> Box<dyn Widget> {
    let Some(file) = vm.selected_file().get() else {
        return Box::new(crate::shared::list_or_empty::empty_state(
            tr!(generate_no_selection()),
            tr!(generate_no_selection_hint()),
        ));
    };

    if vm.view_diff().get() {
        return diff_pane(vm, file, ctx.theme().clone());
    }
    code_pane(vm, file)
}

/// The unified difference against what is on disk.
fn diff_pane(vm: &GenerateViewModel, file: frontend::EntityId, theme: Theme) -> Box<dyn Widget> {
    let text = match vm.diff_of(file) {
        // US-GEN-08: a failed preview says why, in the pane, rather than rendering
        // as an empty one that reads as "this file is identical".
        Err(message) => return Box::new(message_pane(tr!(status_error(message = message)))),
        Ok(text) if text.trim().is_empty() => {
            return Box::new(message_pane(tr!(generate_no_differences())));
        }
        Ok(text) => text,
    };

    let view = LogView::new().severity_highlighter(move |line| diff_colour(line, &theme));
    let handle = view.handle();
    handle.append(&text);
    Box::new(view)
}

/// The generated source, read only.
fn code_pane(vm: &GenerateViewModel, file: frontend::EntityId) -> Box<dyn Widget> {
    let Some(code) = vm.code_of(file) else {
        // The rendering pass has not reached this file yet. Saying so beats an empty
        // pane, which reads as an empty file.
        return Box::new(message_pane(tr!(generate_step_rendering())));
    };

    let document = TextDocument::new();
    if let Err(e) = document.set_plain_text(&code) {
        log::error!("could not show the generated code: {e}");
        return Box::new(message_pane(tr!(status_error(message = e.to_string()))));
    }
    // Read only, with a gutter: this is generated output, and the one thing a user
    // does with it is find a line.
    Box::new(teksilo::widgets::CodeEditor::read_only(document).gutter(true))
}

fn message_pane(message: LocalizedString) -> impl Widget {
    teksilo::widgets::Padding::symmetric(16.0, 24.0)
        .child(teksilo::widgets::TextWidget::new(message).color(TextRole::Secondary))
}

/// Kept for the day a preview needs a plain-text pane that is not code.
#[allow(dead_code)]
fn plain(document: TextDocument) -> PlainTextEditor {
    PlainTextEditor::read_only(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two file headers of a unified diff start with the same characters as an
    /// added and a removed line. Matching them first is what stops every diff from
    /// opening with a green line and a red one that mean nothing.
    #[test]
    fn diff_headers_are_not_mistaken_for_added_and_removed_lines() {
        let theme = crate::style::light();
        let header_old = diff_colour("--- a/src/main.rs", &theme);
        let header_new = diff_colour("+++ b/src/main.rs", &theme);
        let removed = diff_colour("-let x = 1;", &theme);
        let added = diff_colour("+let x = 2;", &theme);

        assert_eq!(header_old, header_new, "both headers read the same");
        assert_ne!(header_old, removed);
        assert_ne!(header_new, added);
        assert_ne!(added, removed);
    }

    #[test]
    fn a_hunk_header_is_its_own_colour() {
        let theme = crate::style::light();
        let hunk = diff_colour("@@ -1,4 +1,6 @@", &theme);
        assert!(hunk.is_some());
        assert_ne!(hunk, diff_colour("+added", &theme));
    }

    #[test]
    fn an_unchanged_line_is_left_alone() {
        let theme = crate::style::light();
        assert_eq!(diff_colour(" context line", &theme), None);
        assert_eq!(diff_colour("", &theme), None);
    }
}
