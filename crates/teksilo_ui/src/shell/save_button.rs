//! The Save button that sits in the title bar, next to the hamburger.
//!
//! It owns no state. It renders two signals the app mirrors into it and fires an
//! intent, which is the rule for anything drawn outside `App`: the title bar
//! renders in chrome that is a sibling of the app tree, so a handler here cannot
//! reach a view-model without capturing one.
//!
//! `TitleBar` builds its slots exactly once and forbids `Rebuild`-level bindings in
//! them, so the icon swap is done *inside* this widget, which is free to rebuild
//! itself.

use teksilo::core::BindingLevel;
use teksilo::prelude::*;
use teksilo::widgets::{IconButton, IconButtonSize};

use crate::intents::AppIntent;

#[derive(Debug)]
pub struct SaveButton {
    /// True while there is something to write. Drives both the glyph and the
    /// enabled state, so the button cannot look pressable and do nothing.
    dirty: Signal<bool>,
    enabled: Signal<bool>,
    root_child: Option<WidgetId>,
}

impl SaveButton {
    pub fn new(dirty: Signal<bool>, enabled: Signal<bool>) -> Self {
        Self {
            dirty,
            enabled,
            root_child: None,
        }
    }
}

impl Widget for SaveButton {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        // `IconButton` takes its glyph at construction, so swapping the icon means
        // rebuilding this widget. Binding at `Rebuild` here is exactly what the
        // title bar cannot do in its own slot.
        self.dirty
            .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);
        let dirty = self.dirty.get();

        let icon = if dirty {
            crate::icons::action::save_unsaved()
        } else {
            crate::icons::action::save()
        };
        let tooltip = if dirty {
            tr!(titlebar_save())
        } else {
            tr!(titlebar_save_disabled())
        };

        let id = ctx.add(
            IconButton::new(icon)
                .size(IconButtonSize::Toolbar)
                .tooltip(tooltip)
                .enabled(self.enabled.clone())
                .on_activate_fn(|c| c.send_intent(AppIntent::SaveManifest.into_intent())),
        );
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
