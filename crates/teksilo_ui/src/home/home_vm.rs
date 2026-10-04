//! What the Home screen knows.
//!
//! Almost nothing of its own: the manifest actions belong to
//! [`crate::manifest::ManifestViewModel`], and Home only decides what to offer and
//! whether the developer-only block is shown. The documentation table lives here
//! rather than in the view so it can be asserted without a widget tree.

use teksilo::prelude::*;

/// One documentation card.
pub struct DocLink {
    pub label: LocalizedString,
    pub blurb: LocalizedString,
    /// The page's file name. The full URL is [`DocLink::url`].
    pub page: &'static str,
}

impl DocLink {
    /// The address the card opens.
    pub fn url(&self) -> String {
        format!("{DOCS_BASE}/{}", self.page)
    }
}

/// Where the published documentation lives. Every card is built from this, so
/// a card cannot quietly point somewhere else.
const DOCS_BASE: &str = "https://qleany-docs.pages.dev";

/// The nine cards, in the order the Slint UI showed them.
pub fn doc_links() -> Vec<DocLink> {
    vec![
        DocLink {
            label: tr!(doc_introduction()),
            blurb: tr!(doc_introduction_blurb()),
            page: "introduction.html",
        },
        DocLink {
            label: tr!(doc_quick_start_cpp_qt()),
            blurb: tr!(doc_quick_start_cpp_qt_blurb()),
            page: "quick-start-cpp-qt.html",
        },
        DocLink {
            label: tr!(doc_quick_start_rust()),
            blurb: tr!(doc_quick_start_rust_blurb()),
            page: "quick-start-rust.html",
        },
        DocLink {
            label: tr!(doc_design_philosophy()),
            blurb: tr!(doc_design_philosophy_blurb()),
            page: "design-philosophy.html",
        },
        DocLink {
            label: tr!(doc_undo_redo()),
            blurb: tr!(doc_undo_redo_blurb()),
            page: "undo-redo-architecture.html",
        },
        DocLink {
            label: tr!(doc_how_operations_flow()),
            blurb: tr!(doc_how_operations_flow_blurb()),
            page: "how-operations-flow.html",
        },
        DocLink {
            label: tr!(doc_manifest_reference()),
            blurb: tr!(doc_manifest_reference_blurb()),
            page: "manifest-reference.html",
        },
        DocLink {
            label: tr!(doc_qml_integration()),
            blurb: tr!(doc_qml_integration_blurb()),
            page: "qml-integration.html",
        },
        DocLink {
            label: tr!(doc_troubleshooting()),
            blurb: tr!(doc_troubleshooting_blurb()),
            page: "troubleshooting.html",
        },
    ]
}

/// Whether the developer-only block is shown.
///
/// An explicit environment variable rather than the Slint UI's heuristic of looking
/// for a `.git` directory three levels above the executable: that fired or did not
/// fire depending on where the binary happened to sit, which is a surprising way
/// for a test affordance to appear.
pub fn developer_mode() -> bool {
    std::env::var("QLEANY_DEV").is_ok_and(|v| v != "0" && !v.is_empty())
}

#[derive(Clone)]
pub struct HomeViewModel {
    developer: Signal<bool>,
}

impl std::fmt::Debug for HomeViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HomeViewModel").finish_non_exhaustive()
    }
}

impl Default for HomeViewModel {
    fn default() -> Self {
        Self::new()
    }
}

impl HomeViewModel {
    pub fn new() -> Self {
        Self {
            developer: Signal::new(developer_mode()),
        }
    }

    pub fn developer(&self) -> Signal<bool> {
        self.developer.clone()
    }

    /// Hand a documentation URL to the OS default browser.
    pub fn open_link(&self, url: &str) {
        if let Err(e) = open::that_detached(url) {
            log::warn!("could not open {url}: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_nine_documentation_cards() {
        assert_eq!(doc_links().len(), 9);
    }

    /// Every card points at the docs site, and none of them at a bare path that
    /// would open as a local file.
    #[test]
    fn every_card_points_at_the_docs_site() {
        for link in doc_links() {
            assert!(
                link.url().starts_with(DOCS_BASE),
                "{} points at {}",
                link.label.resolve_now(),
                link.url()
            );
        }
    }

    #[test]
    fn every_card_url_is_distinct() {
        let mut urls: Vec<String> = doc_links().iter().map(|l| l.url()).collect();
        urls.sort_unstable();
        let before = urls.len();
        urls.dedup();
        assert_eq!(before, urls.len(), "two cards share a URL");
    }
}
