//! The Undo and Redo buttons in the title bar.
//!
//! They own no state, like the Save button beside them: the title bar renders in
//! chrome that is a sibling of the app tree, so a handler here cannot reach a
//! view-model without capturing one. They render two signals and fire two intents.

use teksilo::prelude::*;
use teksilo::widgets::{HStack, IconButton, IconButtonSize};

use crate::intents::AppIntent;

#[derive(Debug)]
pub struct UndoButtons {
    can_undo: Signal<bool>,
    can_redo: Signal<bool>,
    root_child: Option<WidgetId>,
}

impl UndoButtons {
    pub fn new(can_undo: Signal<bool>, can_redo: Signal<bool>) -> Self {
        Self {
            can_undo,
            can_redo,
            root_child: None,
        }
    }
}

impl Widget for UndoButtons {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        // No rebuild binding: the glyphs never change, only whether the buttons are
        // live, and `enabled` is reactive on its own. That is the difference from
        // the Save button, which swaps its icon.
        let id = ctx.add(teksu!(
            HStack {
                spacing: 2.0
                IconButton::new(crate::icons::action::undo()) {
                    size: IconButtonSize::Toolbar
                    tooltip: tr!(titlebar_undo())
                    enabled: self.can_undo.clone()
                    on_activate_fn: |c| c.send_intent(AppIntent::Undo.into_intent())
                }
                IconButton::new(crate::icons::action::redo()) {
                    size: IconButtonSize::Toolbar
                    tooltip: tr!(titlebar_redo())
                    enabled: self.can_redo.clone()
                    on_activate_fn: |c| c.send_intent(AppIntent::Redo.into_intent())
                }
            }
        ));
        self.root_child = Some(id);
        vec![id]
    }

    fn layout_response(&self, proposal: SizeProposal, ctx: &LayoutContext) -> LayoutResponse {
        self.root_child
            .and_then(|id| ctx.child_size(id, proposal))
            .map(LayoutResponse::from)
            .unwrap_or_else(|| proposal.resolve(0.0, 0.0).into())
    }
}
