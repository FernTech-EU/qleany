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
        .on_result(move |result, ctx| match result.button {
            StandardButton::Save => {
                vm.save();
                then(ctx);
            }
            StandardButton::Discard => then(ctx),
            _ => {}
        })
        .present(ctx);
}
