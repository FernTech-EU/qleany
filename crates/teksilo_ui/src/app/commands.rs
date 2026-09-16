//! Where every named command is consumed.
//!
//! One `Action` per intent, all registered `_global`. The `_global` is not a
//! precaution: intents walk from the source widget to the root, and the title-bar
//! menu renders in an **overlay** that is a sibling of `App` rather than a
//! descendant, so a plain `register_action` would never be on that path and every
//! menu row would silently do nothing.

use teksilo::core::BindingLevel;
use teksilo::prelude::*;

use crate::about;
use crate::app_ids::{AppIds, Screen};
use crate::demo::{self, DemoViewModel};
use crate::edit::UndoViewModel;
use crate::entities::EntitiesViewModel;
use crate::intents::name;
use crate::manifest::{ManifestViewModel, with_unsaved_settled};
use crate::new_manifest::{self, NewManifestViewModel};
use crate::shell::menus::MenuParts;

/// The handles the actions need. Threaded in rather than looked up, because
/// `ctx.app_state::<T>()` is one process-wide slot and this is per-window state.
pub struct CommandDeps {
    pub ids: AppIds,
    pub parts: MenuParts,
    pub manifest: ManifestViewModel,
    pub entities: EntitiesViewModel,
    pub undo: UndoViewModel,
    pub new_manifest: NewManifestViewModel,
    pub demo: DemoViewModel,
}

/// Register everything. Called from `App::build`, on every build: an action
/// registration is scoped to one build cycle, like a subscription.
pub fn register(ctx: &mut BuildContext, deps: &CommandDeps) {
    register_navigation(ctx, deps);
    register_theme(ctx, deps);
    register_manifest(ctx, deps);
    register_entities(ctx, deps);
    register_undo(ctx, deps);
    register_demo(ctx, deps);
    register_help(ctx, deps);
}

/// Help, About, and the guard that stands between unsaved work and Quit.
fn register_help(ctx: &mut BuildContext, deps: &CommandDeps) {
    ctx.register_action_global(Action::new(name::ABOUT).on_invoke(|_i, c| about::present(c)));

    // US-SAFE: quitting with unsaved work asks first.
    //
    // The window's close guard vetoes and fires this same intent, so the menu row,
    // Ctrl+Q and the window's own close button are one path rather than three, and
    // the button most users press is the one that would otherwise be unguarded.
    // `close_window_forced` is the second half of that: it bypasses the guard that
    // opened this dialog rather than re-triggering it.
    let vm = deps.manifest.clone();
    ctx.register_action_global(Action::new(name::QUIT).on_invoke(move |_i, c| {
        with_unsaved_settled(
            &vm,
            c,
            tr!(quit_unsaved_title()),
            tr!(quit_unsaved_message()),
            |ctx| ctx.close_window_forced(),
        );
    }));
}

/// The demo generator.
///
/// Guarded the same way Quit is, and for the same reason: the demo loads a manifest
/// of its own into the one store this application has, so starting it with unsaved
/// work discards that work. The Slint UI did it without asking.
fn register_demo(ctx: &mut BuildContext, deps: &CommandDeps) {
    let manifest = deps.manifest.clone();
    let vm = deps.demo.clone();
    ctx.register_action_global(Action::new(name::RUN_DEMO).on_invoke(move |_i, c| {
        let vm = vm.clone();
        with_unsaved_settled(
            &manifest,
            c,
            tr!(demo_unsaved_title()),
            tr!(demo_unsaved_message()),
            move |ctx| demo::present(&vm, ctx),
        );
    }));
}

/// Undo and redo, on the active screen's stack.
///
/// Ctrl+Shift+Z as well as Ctrl+Y: the first is what a user coming from a Mac or
/// from most Linux applications reaches for, the second what a user coming from
/// Windows does, and a redo that only answers one of them reads as broken.
fn register_undo(ctx: &mut BuildContext, deps: &CommandDeps) {
    let vm = deps.undo.clone();
    ctx.register_action_global(
        Action::new(name::UNDO)
            .enabled_when(deps.parts.can_undo.clone())
            .on_invoke(move |_i, c| vm.undo(c)),
    );
    let vm = deps.undo.clone();
    ctx.register_action_global(
        Action::new(name::REDO)
            .enabled_when(deps.parts.can_redo.clone())
            .on_invoke(move |_i, c| vm.redo(c)),
    );

    ctx.register_shortcut_global(
        Shortcut::new(name::UNDO)
            .intent(name::UNDO)
            .primary(KeyStroke::ctrl(Key::Character('z')))
            .enabled_when(deps.parts.can_undo.clone())
            .build(),
    );
    ctx.register_shortcut_global(
        Shortcut::new(name::REDO)
            .intent(name::REDO)
            .primary(KeyStroke::ctrl_shift(Key::Character('z')))
            .secondary(KeyStroke::ctrl(Key::Character('y')))
            .enabled_when(deps.parts.can_redo.clone())
            .build(),
    );
}

/// The Mermaid export: the one entity action that is about the model as a whole
/// rather than about a selected row.
///
/// Registered here rather than on the screen because it is also a menu row, and the
/// menu renders in an overlay that is a sibling of `App`.
fn register_entities(ctx: &mut BuildContext, deps: &CommandDeps) {
    let vm = deps.entities.clone();
    ctx.register_action_global(
        Action::new(name::EXPORT_MERMAID)
            .enabled_when(deps.parts.manifest_open.clone())
            .on_invoke(move |_i, c| vm.export_to_mermaid(c)),
    );
}

/// Open, save, save as, close, and the developer shortcut to Qleany's own manifest.
fn register_manifest(ctx: &mut BuildContext, deps: &CommandDeps) {
    let open = deps.parts.manifest_open.clone();
    let can_save = deps.parts.can_save.clone();

    // US-SAFE: every path that replaces or discards the open manifest asks first,
    // not just Quit. The Slint UI guarded New, Open and Close from its Home screen
    // and this one did not, which made three of its own commands quietly
    // destructive. The question is the same one, so it comes from the same place.
    let wizard = deps.new_manifest.clone();
    let wizard_manifest = deps.manifest.clone();
    ctx.register_action_global(Action::new(name::NEW_MANIFEST).on_invoke(move |_i, c| {
        let wizard = wizard.clone();
        let wizard_manifest = wizard_manifest.clone();
        with_unsaved_settled(
            &wizard_manifest.clone(),
            c,
            tr!(new_manifest_unsaved_title()),
            tr!(new_manifest_unsaved_message()),
            move |ctx| new_manifest::present(&wizard, &wizard_manifest, ctx),
        );
    }));

    let vm = deps.manifest.clone();
    ctx.register_action_global(Action::new(name::OPEN_MANIFEST).on_invoke(move |_i, c| {
        let vm = vm.clone();
        with_unsaved_settled(
            &vm.clone(),
            c,
            tr!(open_manifest_unsaved_title()),
            tr!(open_manifest_unsaved_message()),
            move |ctx| vm.pick_open(ctx),
        );
    }));

    let vm = deps.manifest.clone();
    ctx.register_action_global(
        Action::new(name::SAVE_MANIFEST)
            .enabled_when(can_save.clone())
            .on_invoke(move |_i, _c| vm.save()),
    );

    let vm = deps.manifest.clone();
    ctx.register_action_global(
        Action::new(name::SAVE_MANIFEST_AS)
            .enabled_when(open.clone())
            .on_invoke(move |_i, c| vm.pick_save_as(c)),
    );

    let vm = deps.manifest.clone();
    ctx.register_action_global(
        Action::new(name::CLOSE_MANIFEST)
            .enabled_when(open.clone())
            .on_invoke(move |_i, c| {
                let vm = vm.clone();
                with_unsaved_settled(
                    &vm.clone(),
                    c,
                    tr!(close_manifest_unsaved_title()),
                    tr!(close_manifest_unsaved_message()),
                    move |_ctx| vm.close(),
                );
            }),
    );

    // The developer affordance loads the manifest in the working directory, which
    // for anyone running from a Qleany checkout is Qleany's own. Guarded like the
    // rest: it is a load, and a load replaces whatever is open.
    let vm = deps.manifest.clone();
    ctx.register_action_global(
        Action::new(name::OPEN_QLEANY_MANIFEST).on_invoke(move |_i, c| {
            let vm = vm.clone();
            with_unsaved_settled(
                &vm.clone(),
                c,
                tr!(open_manifest_unsaved_title()),
                tr!(open_manifest_unsaved_message()),
                move |_ctx| vm.open_path("qleany.yaml"),
            );
        }),
    );

    ctx.register_shortcut_global(
        Shortcut::new(name::OPEN_MANIFEST)
            .intent(name::OPEN_MANIFEST)
            .primary(KeyStroke::ctrl(Key::Character('o')))
            .build(),
    );
    ctx.register_shortcut_global(
        Shortcut::new(name::SAVE_MANIFEST)
            .intent(name::SAVE_MANIFEST)
            .primary(KeyStroke::ctrl(Key::Character('s')))
            .enabled_when(can_save)
            .build(),
    );
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
