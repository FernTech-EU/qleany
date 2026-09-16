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
use crate::session::Session;
use crate::shell::TITLE_BAR_HEIGHT;
use crate::shell::menus::{self, MenuParts};
use crate::shell::save_button::SaveButton;
use crate::shell::theme_button::ThemeButton;

/// Build the window's widget tree.
///
/// The title bar is conditional. `DecorationsMode::CustomChrome` falls back to the
/// native frame on a window manager that does not advertise `_NET_WM_MOVERESIZE`,
/// and there `title_bar_host()` is `None`. Mounting a `TitleBar` unconditionally
/// would draw a second bar under the real one, so the `None` arm puts the same
/// controls in an ordinary strip and lets the OS draw the frame.
pub fn build_root(tree: &mut WidgetTree, session: Session, ids: AppIds) -> WidgetId {
    let parts = MenuParts {
        manifest_open: Signal::new(false),
        can_save: Signal::new(false),
        can_undo: Signal::new(false),
        can_redo: Signal::new(false),
        dark: Signal::new(false),
        check_critical: Signal::new(false),
    };

    let menubar = MenuBar::from_model(menus::build_menu(&parts))
        // On macOS the same model is mirrored into the global bar at the top of the
        // screen, so the in-window hamburger would be a duplicate. Inert elsewhere.
        .native_on_macos(NativeMenuMode::Suppress)
        .collapse_policy(CollapsePolicy::Always)
        .hamburger_size(IconButtonSize::Toolbar);

    let save = SaveButton::new(parts.can_save.clone(), parts.can_save.clone());
    let theme = ThemeButton::new(parts.dark.clone());

    let body = App::new(session, ids, parts.clone());

    match tree.title_bar_host() {
        Some(host) => {
            let leading = teksu!(
                HStack {
                    spacing: 4.0
                    alignment: VAlignment::Center
                    child: menubar
                    child: save
                }
            );
            let trailing = teksu!(
                HStack {
                    spacing: 4.0
                    alignment: VAlignment::Center
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
                            TextWidget::new(tr!(app_name()))
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
                    Expand::horizontal
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
