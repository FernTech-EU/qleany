//! Help, About.

use teksilo::core::modal::{ModalCloseBehavior, ModalPresentation, ModalRequest};
use teksilo::prelude::*;
use teksilo::widgets::{
    Button, DialogContent, Expand, HStack, Link, ModalContainer, TextWidget, VStack,
};

/// Where the published documentation lives. The Home screen's cards are built from
/// the same address; a second copy here would be a second thing to move.
const DOCS: &str = "https://qleany-docs.pages.dev";

/// The repository.
const REPOSITORY: &str = "https://github.com/jacquetc/qleany";

/// The version this binary was built from.
///
/// Read from Cargo at compile time rather than from the manifest at run time: the
/// question "what am I running" is about the binary.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Show it.
///
/// `DialogContent` rather than `MessageBox`: this is information with two links in
/// it, not a question, and a message box's buttons are answers. Presented the way
/// the wizard is, because `Dialog` is a trigger button and About is a menu row.
pub fn present(ctx: &mut EventContext) {
    ctx.present_modal(
        ModalRequest::deferred(move |tree| tree.add(ModalContainer::new(content())))
            .presentation(ModalPresentation::Auto)
            .close_behavior(ModalCloseBehavior::default())
            .title(tr!(about_title()).resolve_now()),
    );
}

fn content() -> DialogContent {
    let body = teksu!(
        VStack {
            spacing: 8.0
            TextWidget::new(tr!(about_version(version = VERSION))) {
                color: TextRole::Secondary
            }
            TextWidget::new(tr!(about_made_by()))
            TextWidget::new(tr!(about_licence())) {
                style: TextStyleRole::Small
                color: TextRole::Secondary
            }
            Link::new(tr!(about_docs_link())) {
                on_activate_fn: |_c| open(DOCS)
            }
            Link::new(tr!(about_repository_link())) {
                on_activate_fn: |_c| open(REPOSITORY)
            }
        }
    );

    DialogContent::new()
        .title(tr!(about_title()))
        .body(body)
        // One button, and it only closes: there is nothing here to confirm, and
        // Escape does the same.
        .footer(teksu!(
            HStack {
                spacing: 8.0
                Expand::horizontal
                Button::new(tr!(common_close())) {
                    on_activate_fn: |c| c.dismiss_modal()
                }
            }
        ))
}

fn open(url: &str) {
    if let Err(e) = open::that_detached(url) {
        log::warn!("could not open {url}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The version is the binary's, not the manifest's: "what am I running" is a
    /// question about this build.
    #[test]
    fn the_version_is_the_crates_own() {
        assert!(!VERSION.is_empty());
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            tr!(about_version(version = VERSION)).resolve_now(),
            format!("Version {VERSION}")
        );
    }

    /// Both links are absolute and point outside the app: a relative one would open
    /// as a local file.
    #[test]
    fn both_links_are_addresses() {
        for url in [DOCS, REPOSITORY] {
            assert!(url.starts_with("https://"), "{url}");
        }
    }
}
