//! The light/dark toggle in the title bar.
//!
//! Same shape as [`crate::shell::save_button::SaveButton`]: no state of its own,
//! one signal in, one intent out. The glyph advertises the action rather than the
//! state, so while the app is light it shows a moon.

use teksilo::core::BindingLevel;
use teksilo::prelude::*;
use teksilo::widgets::{IconButton, IconButtonSize};

use crate::intents::AppIntent;

#[derive(Debug)]
pub struct ThemeButton {
    dark: Signal<bool>,
    root_child: Option<WidgetId>,
}

impl ThemeButton {
    pub fn new(dark: Signal<bool>) -> Self {
        Self {
            dark,
            root_child: None,
        }
    }
}

impl Widget for ThemeButton {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        self.dark
            .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);
        let dark = self.dark.get();

        let (icon, tooltip) = if dark {
            (
                crate::icons::theme::to_light(),
                tr!(titlebar_theme_to_light()),
            )
        } else {
            (
                crate::icons::theme::to_dark(),
                tr!(titlebar_theme_to_dark()),
            )
        };

        let id = ctx.add(
            IconButton::new(icon)
                .size(IconButtonSize::Toolbar)
                .tooltip(tooltip)
                .on_activate_fn(|c| c.send_intent(AppIntent::ToggleTheme.into_intent())),
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
