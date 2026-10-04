//! The window root: title bar over the app body.

use teksilo::core::WidgetTree;
use teksilo::prelude::*;
use teksilo::tokens::VAlignment;
use teksilo::widgets::menu::NativeMenuMode;
use teksilo::widgets::{
    Center, CollapsePolicy, DeadZone, Expand, HStack, IconButtonSize, MenuBar, TextWidget,
    TitleBar, VStack,
};

use crate::app::App;
use crate::app_ids::AppIds;
use crate::check::{CheckBadge, CheckViewModel};
use crate::edit::UndoViewModel;
use crate::manifest::ManifestViewModel;
use crate::session::Session;
use crate::shell::TITLE_BAR_HEIGHT;
use crate::shell::menus::{self, MenuParts};
use crate::shell::save_button::SaveButton;
use crate::shell::theme_button::ThemeButton;
use crate::shell::undo_buttons::UndoButtons;

/// Build the window's widget tree.
///
/// The title bar is conditional. `DecorationsMode::CustomChrome` falls back to the
/// native frame on a window manager that does not advertise `_NET_WM_MOVERESIZE`,
/// and there `title_bar_host()` is `None`. Mounting a `TitleBar` unconditionally
/// would draw a second bar under the real one, so the `None` arm puts the same
/// controls in an ordinary strip and lets the OS draw the frame.
pub fn build_root(
    tree: &mut WidgetTree,
    session: Session,
    ids: AppIds,
    unsaved: Signal<bool>,
    manifest: ManifestViewModel,
) -> WidgetId {
    // Validation is shell state, not a screen: the badge is in the title bar and
    // the navigation rail reads its verdict to decide whether Generate is reachable.
    let check = CheckViewModel::new(session.app_ctx.clone(), manifest.is_open());
    // Undo is shell state too: it follows the active screen, and both the title bar
    // and the Edit menu read it.
    let undo = UndoViewModel::new(session.app_ctx.clone(), ids.clone());
    let parts = MenuParts {
        manifest_open: manifest.is_open(),
        can_save: manifest.can_save(),
        can_undo: undo.can_undo(),
        can_redo: undo.can_redo(),
        undo_label: undo.undo_label(),
        redo_label: undo.redo_label(),
        dark: Signal::new(false),
        check_critical: check.critical(),
    };

    let menubar = MenuBar::from_model(menus::build_menu(&parts))
        // On macOS the same model is mirrored into the global bar at the top of the
        // screen, so the in-window hamburger would be a duplicate. Inert elsewhere.
        .native_on_macos(NativeMenuMode::Suppress)
        .collapse_policy(CollapsePolicy::Always)
        .hamburger_size(IconButtonSize::Toolbar);

    let save = SaveButton::new(
        manifest.is_saved().map(|saved| !*saved),
        parts.can_save.clone(),
    );
    let theme = ThemeButton::new(parts.dark.clone());
    let badge = CheckBadge::new(check.clone());
    let undo_buttons = UndoButtons::new(undo.can_undo(), undo.can_redo());

    let body = App::new(
        session,
        ids,
        parts.clone(),
        manifest.clone(),
        check,
        undo,
        unsaved,
    );

    match tree.title_bar_host() {
        Some(host) => {
            let leading = teksu!(
                HStack {
                    spacing: 4.0
                    alignment: VAlignment::Center
                    child: menubar
                    child: save
                    child: badge
                }
            );
            let trailing = teksu!(
                HStack {
                    spacing: 4.0
                    alignment: VAlignment::Center
                    child: undo_buttons
                    child: theme
                }
            );
            // The centre slot lives inside the bar's drag region, which is published
            // to the OS as the window caption. The OS owns caption pixels, so a bare
            // widget there would never see a click. `DeadZone` carves this back out.
            // `Center` on both axes: without it the label sits at the drag
            // region's top-left, which in a 34 dp bar reads as a stray word rather
            // than a window title.
            let centre = teksu!(
                Expand::horizontal {
                    Center {
                        DeadZone {
                            TextWidget::new(tr!(app_name())) {
                                text: manifest.title().map(|t| t.resolve_now())
                            }
                        }
                    }
                }
            );
            let bar = TitleBar::new(host)
                .height(TITLE_BAR_HEIGHT)
                .leading(leading)
                .center(centre)
                .trailing(trailing);
            tree.add(teksu!(
                VStack {
                    spacing: 0.0
                    child: bar
                    Expand {
                        child: body
                    }
                }
            ))
        }
        None => {
            let strip = teksu!(
                HStack {
                    spacing: 4.0
                    alignment: VAlignment::Center
                    child: menubar
                    child: save
                    child: badge
                    Expand::horizontal
                    child: undo_buttons
                    child: theme
                }
            );
            tree.add(teksu!(
                VStack {
                    spacing: 0.0
                    child: strip
                    Expand {
                        child: body
                    }
                }
            ))
        }
    }
}
