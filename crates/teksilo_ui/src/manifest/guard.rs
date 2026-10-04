//! Standing between unsaved work and whatever would discard it.
//!
//! Two things in this application throw an open manifest away: quitting, and
//! running the demo, which loads a manifest of its own into the same store. Both
//! ask the same question, so both ask it through here rather than each growing its
//! own dialog.

use teksilo::prelude::*;
use teksilo::widgets::{MessageBox, MessageBoxButtons, StandardButton};

use crate::manifest::ManifestViewModel;

/// Deal with unsaved work, then do the thing.
///
/// `then` runs immediately when there is nothing to lose, which is the common case
/// and must not cost a dialog. Otherwise the user answers first, and Cancel means
/// `then` never runs at all.
///
/// `SaveDiscardCancel` rather than a yes/no: "do you want to save" with two answers
/// makes one of them stand for both "no" and "stop", and a user who meant to stop
/// loses the work either way. Teksilo's preset makes Cancel the escape, so the
/// answer that keeps everything is the one Escape takes.
pub fn with_unsaved_settled(
    vm: &ManifestViewModel,
    ctx: &mut EventContext,
    title: LocalizedString,
    message: LocalizedString,
    then: impl Fn(&mut EventContext) + 'static,
) {
    if !vm.can_save().get() {
        then(ctx);
        return;
    }
    let vm = vm.clone();
    MessageBox::question(title)
        .text(message)
        .buttons(MessageBoxButtons::SaveDiscardCancel)
        .on_result(move |result, ctx| {
            if may_proceed(&vm, result.button) {
                then(ctx);
            }
        })
        .present(ctx);
}

fn may_proceed(vm: &ManifestViewModel, choice: StandardButton) -> bool {
    match choice {
        StandardButton::Save => vm.try_save().is_ok(),
        StandardButton::Discard => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Fixture;
    use frontend::handling_manifest::dtos::CreateLanguage;

    #[test]
    fn failed_save_keeps_the_manifest_open_and_blocks_discard() {
        let f = Fixture::new(CreateLanguage::Rust);
        // A directory in place of the file fails even when tests run as root.
        std::fs::remove_file(&f.path).unwrap();
        std::fs::create_dir(&f.path).unwrap();
        assert!(!may_proceed(&f.manifest, StandardButton::Save));
        assert!(f.manifest.is_open().get());
        assert!(f.ids.workspace_id.get().is_some());
        assert!(f.manifest.error().get().is_some());
        assert!(!may_proceed(&f.manifest, StandardButton::Cancel));
        assert!(may_proceed(&f.manifest, StandardButton::Discard));
    }

    #[test]
    fn successful_save_allows_the_pending_action() {
        let f = Fixture::new(CreateLanguage::Rust);
        assert!(may_proceed(&f.manifest, StandardButton::Save));
        assert!(f.path.is_file());
        assert!(f.manifest.is_saved().get());
        assert!(f.manifest.error().get().is_none());
    }
}
