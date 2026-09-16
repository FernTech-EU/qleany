//! Where every named command is consumed.
//!
//! One `Action` per intent, all registered `_global`. The `_global` is not a
//! precaution: intents walk from the source widget to the root, and the title-bar
//! menu renders in an **overlay** that is a sibling of `App` rather than a
//! descendant, so a plain `register_action` would never be on that path and every
//! menu row would silently do nothing.

use teksilo::core::BindingLevel;
use teksilo::prelude::*;

use crate::app_ids::{AppIds, Screen};
use crate::intents::name;
use crate::shell::menus::MenuParts;

/// The handles the actions need. Threaded in rather than looked up, because
/// `ctx.app_state::<T>()` is one process-wide slot and this is per-window state.
pub struct CommandDeps {
    pub ids: AppIds,
    pub parts: MenuParts,
}

/// Register everything. Called from `App::build`, on every build: an action
/// registration is scoped to one build cycle, like a subscription.
pub fn register(ctx: &mut BuildContext, deps: &CommandDeps) {
    register_navigation(ctx, deps);
    register_theme(ctx, deps);
}

/// The six screen intents, plus Ctrl+1 to Ctrl+6.
fn register_navigation(ctx: &mut BuildContext, deps: &CommandDeps) {
    for (intent, screen, digit) in [
        (name::SHOW_HOME, Screen::Home, '1'),
        (name::SHOW_PROJECT, Screen::Project, '2'),
        (name::SHOW_ENTITIES, Screen::Entities, '3'),
        (name::SHOW_FEATURES, Screen::Features, '4'),
        (name::SHOW_USER_INTERFACE, Screen::UserInterface, '5'),
        (name::SHOW_GENERATE, Screen::Generate, '6'),
    ] {
        // The same rule the rail draws itself from, so a shortcut can never reach a
        // screen the rail says is closed.
        let enabled = deps
            .parts
            .manifest_open
            .zip(&deps.parts.check_critical)
            .map(move |(open, critical)| crate::app::nav::row_enabled(screen, *open, *critical));

        ctx.register_shortcut_global(
            Shortcut::new(intent)
                .intent(intent)
                .primary(KeyStroke::ctrl(Key::Character(digit)))
                .enabled_when(enabled.clone())
                .build(),
        );

        let target = deps.ids.screen.clone();
        ctx.register_action_global(Action::new(intent).enabled_when(enabled).on_invoke(
            move |_i, _c| {
                target.set_if_changed(screen);
            },
        ));
    }
}

/// The light/dark toggle, fired by the title-bar button and by both View rows.
fn register_theme(ctx: &mut BuildContext, deps: &CommandDeps) {
    // Seed from the live theme, so the glyph matches what is on screen from the
    // first frame rather than from the first toggle.
    deps.parts
        .dark
        .set_if_changed(crate::style::is_dark(ctx.theme()));

    let dark = deps.parts.dark.clone();
    ctx.register_action_global(Action::new(name::TOGGLE_THEME).on_invoke(move |_i, c| {
        let next = !dark.get();
        dark.set(next);
        c.set_theme(crate::style::for_dark(next));
    }));
}

/// Keep a binding on the screen signal so the body re-renders when navigation
/// changes it. Separate from `register` because it belongs to the widget's own
/// build, not to the command surface.
pub fn bind_screen(ctx: &mut BuildContext, ids: &AppIds) {
    ids.screen
        .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);
}
