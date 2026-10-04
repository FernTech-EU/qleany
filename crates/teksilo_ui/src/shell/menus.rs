//! The menu model the hamburger opens, and that macOS mirrors into its global bar.
//!
//! Declared as data rather than built as widgets: one `MenuModel` drives the
//! in-window `MenuBar` and the native NSMenu, so the two cannot drift. Every entry
//! fires an intent and binds an `enabled` signal; none of them calls a view-model,
//! because the bar renders in an overlay that is a sibling of `App`.

use teksilo::prelude::*;
use teksilo::widgets::menu::{MenuEntry, MenuModel, StandardMenu};

use crate::intents::name;

/// Everything the menu needs to decide what is reachable right now.
#[derive(Clone)]
pub struct MenuParts {
    /// A manifest is open. Gates everything that reads or writes one.
    pub manifest_open: Signal<bool>,
    /// There is something to write.
    pub can_save: Signal<bool>,
    /// The active screen's undo stack has history.
    pub can_undo: Signal<bool>,
    pub can_redo: Signal<bool>,
    /// What Undo and Redo would take back. A `LocalizedString` that observes the
    /// stack rather than a `Signal`, because a menu model is built once and
    /// `MenuEntry::new` takes its title by value.
    pub undo_label: LocalizedString,
    pub redo_label: LocalizedString,
    /// The resolved theme, so the two View rows can be a radio pair.
    pub dark: Signal<bool>,
    /// The manifest has at least one critical validation error, which is what puts
    /// the Generate screen out of reach.
    pub check_critical: Signal<bool>,
}

impl std::fmt::Debug for MenuParts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuParts").finish_non_exhaustive()
    }
}

pub fn build_menu(parts: &MenuParts) -> MenuModel {
    let light = parts.dark.map(|d| !*d);
    let dark = parts.dark.clone();

    MenuModel::new()
        // macOS moves About and Quit into the application menu and takes them out
        // of the in-window bar. `install_native_menu` on the builder is the other
        // half; without it this is inert rather than wrong.
        .standard_menu(
            StandardMenu::app()
                .about(tr!(native_menu_about(app = "Qleany")))
                .quit(tr!(native_menu_quit(app = "Qleany")))
                .quit_intent(name::QUIT),
        )
        .menu(tr!(menu_file()), |m| {
            m.item(MenuEntry::new(tr!(menu_new_manifest())).intent(name::NEW_MANIFEST))
                .item(MenuEntry::new(tr!(menu_open_manifest())).intent(name::OPEN_MANIFEST))
                .separator()
                .item(
                    MenuEntry::new(tr!(menu_save_manifest()))
                        .intent(name::SAVE_MANIFEST)
                        .enabled(parts.can_save.clone()),
                )
                .item(
                    MenuEntry::new(tr!(menu_save_manifest_as()))
                        .intent(name::SAVE_MANIFEST_AS)
                        .enabled(parts.manifest_open.clone()),
                )
                .item(
                    MenuEntry::new(tr!(menu_close_manifest()))
                        .intent(name::CLOSE_MANIFEST)
                        .enabled(parts.manifest_open.clone()),
                )
                .separator()
                .item(MenuEntry::new(tr!(menu_run_demo())).intent(name::RUN_DEMO))
                .separator()
                .item(MenuEntry::new(tr!(menu_quit())).intent(name::QUIT))
        })
        .menu(tr!(menu_edit()), |m| {
            m.item(
                MenuEntry::new(parts.undo_label.clone())
                    .intent(name::UNDO)
                    .enabled(parts.can_undo.clone()),
            )
            .item(
                MenuEntry::new(parts.redo_label.clone())
                    .intent(name::REDO)
                    .enabled(parts.can_redo.clone()),
            )
        })
        .menu(tr!(menu_view()), |m| {
            m.item(
                MenuEntry::new(tr!(menu_theme_light()))
                    .intent(name::TOGGLE_THEME)
                    .checked(light),
            )
            .item(
                MenuEntry::new(tr!(menu_theme_dark()))
                    .intent(name::TOGGLE_THEME)
                    .checked(dark),
            )
            .separator()
            .item(MenuEntry::new(tr!(nav_home())).intent(name::SHOW_HOME))
            .item(
                MenuEntry::new(tr!(nav_project()))
                    .intent(name::SHOW_PROJECT)
                    .enabled(parts.manifest_open.clone()),
            )
            .item(
                MenuEntry::new(tr!(nav_entities()))
                    .intent(name::SHOW_ENTITIES)
                    .enabled(parts.manifest_open.clone()),
            )
            .item(
                MenuEntry::new(tr!(nav_features()))
                    .intent(name::SHOW_FEATURES)
                    .enabled(parts.manifest_open.clone()),
            )
            .item(
                MenuEntry::new(tr!(nav_user_interface()))
                    .intent(name::SHOW_USER_INTERFACE)
                    .enabled(parts.manifest_open.clone()),
            )
            .item(
                MenuEntry::new(tr!(nav_generate()))
                    .intent(name::SHOW_GENERATE)
                    .enabled(parts.manifest_open.clone()),
            )
        })
        .menu(tr!(menu_help()), |m| {
            m.item(MenuEntry::new(tr!(menu_about())).intent(name::ABOUT))
        })
}
