//! The navigation rail: one row per screen, down the left edge.
//!
//! A named widget rather than a free function, because the selected row is drawn in
//! a different `ButtonVariant` and a variant is a value rather than a signal. The
//! rail therefore rebuilds when the screen changes, which is what a `Rebuild`-level
//! binding on `screen` buys.

use teksilo::core::BindingLevel;
use teksilo::prelude::*;
use teksilo::widgets::{Button, ButtonVariant, IconLocation, IconWidget, Padding, VStack};

use crate::app_ids::{AppIds, Screen};
use crate::intents::name;

/// Width of the rail, in logical pixels. Wide enough for "User Interface" beside a
/// 20 dp glyph without wrapping, which is what the Slint rail was too narrow for.
const RAIL_WIDTH: f32 = 172.0;

pub struct NavRail {
    ids: AppIds,
    /// A manifest is open. Every screen but Home is unreachable without one.
    manifest_open: Signal<bool>,
    /// The manifest has a critical validation error. Generate is unreachable then,
    /// because generating from a broken manifest produces broken code.
    check_critical: Signal<bool>,
    root_child: Option<WidgetId>,
}

impl std::fmt::Debug for NavRail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavRail").finish_non_exhaustive()
    }
}

impl NavRail {
    pub fn new(ids: AppIds, manifest_open: Signal<bool>, check_critical: Signal<bool>) -> Self {
        Self {
            ids,
            manifest_open,
            check_critical,
            root_child: None,
        }
    }
}

/// Everything that differs between the six rows.
struct Row {
    screen: Screen,
    label: LocalizedString,
    icon: fn() -> IconWidget,
    intent: &'static str,
}

fn rows() -> [Row; 6] {
    [
        Row {
            screen: Screen::Home,
            label: tr!(nav_home()),
            icon: crate::icons::nav::home,
            intent: name::SHOW_HOME,
        },
        Row {
            screen: Screen::Project,
            label: tr!(nav_project()),
            icon: crate::icons::nav::project,
            intent: name::SHOW_PROJECT,
        },
        Row {
            screen: Screen::Entities,
            label: tr!(nav_entities()),
            icon: crate::icons::nav::entities,
            intent: name::SHOW_ENTITIES,
        },
        Row {
            screen: Screen::Features,
            label: tr!(nav_features()),
            icon: crate::icons::nav::features,
            intent: name::SHOW_FEATURES,
        },
        Row {
            screen: Screen::UserInterface,
            label: tr!(nav_user_interface()),
            icon: crate::icons::nav::user_interface,
            intent: name::SHOW_USER_INTERFACE,
        },
        Row {
            screen: Screen::Generate,
            label: tr!(nav_generate()),
            icon: crate::icons::nav::generate,
            intent: name::SHOW_GENERATE,
        },
    ]
}

/// Whether a row is reachable right now.
///
/// A plain function over the two gating signals rather than a method, so the rule
/// is unit-testable without a widget tree.
pub fn row_enabled(screen: Screen, manifest_open: bool, check_critical: bool) -> bool {
    match screen {
        Screen::Home => true,
        Screen::Generate => manifest_open && !check_critical,
        _ => manifest_open,
    }
}

impl Widget for NavRail {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        // All three at `Rebuild`, not just the screen. A row's `enabled` prop is
        // reactive for painting, but the accessible node it publishes is written
        // when the row is built, so a rail that only rebuilt on a screen change
        // would keep telling assistive technology, and every automation probe, that
        // a row is still disabled after the manifest opened.
        //
        // Bound on the source signals, not on a derived one built inline: a
        // `.map(..)` temporary is dropped at the end of the statement, so the
        // binding would be registered against something that no longer exists and
        // the rail would never rebuild at all.
        self.ids
            .screen
            .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);
        self.manifest_open
            .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);
        self.check_critical
            .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);
        let current = self.ids.screen.get();

        let mut column = VStack::new().spacing(2.0);
        for row in rows() {
            let selected = row.screen == current;
            let enabled = self
                .manifest_open
                .zip(&self.check_critical)
                .map(move |(open, critical)| row_enabled(row.screen, *open, *critical));
            let intent = row.intent;
            column = column.child(
                Button::new(row.label)
                    .icon((row.icon)(), IconLocation::Leading)
                    // Filled marks the current screen; Ghost keeps the rest quiet.
                    .variant(if selected {
                        ButtonVariant::Filled
                    } else {
                        ButtonVariant::Ghost
                    })
                    .enabled(enabled)
                    .on_activate_fn(move |c| c.send_intent(Intent::new(intent))),
            );
        }

        let id = ctx.add(Padding::new(8.0, 8.0, 8.0, 8.0).child(column));
        self.root_child = Some(id);
        vec![id]
    }

    fn layout_response(&self, proposal: SizeProposal, ctx: &LayoutContext) -> LayoutResponse {
        let inner = self
            .root_child
            .and_then(|id| ctx.child_size(id, proposal))
            .map(|s| s.height)
            .unwrap_or(0.0);
        proposal.resolve(RAIL_WIDTH, inner).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_is_always_reachable() {
        assert!(row_enabled(Screen::Home, false, false));
        assert!(row_enabled(Screen::Home, false, true));
    }

    #[test]
    fn the_other_screens_need_an_open_manifest() {
        for screen in [
            Screen::Project,
            Screen::Entities,
            Screen::Features,
            Screen::UserInterface,
            Screen::Generate,
        ] {
            assert!(!row_enabled(screen, false, false), "{screen:?}");
            assert!(row_enabled(screen, true, false), "{screen:?}");
        }
    }

    /// A critical validation error blocks Generate and nothing else: the user has to
    /// stay able to reach the screens that let them fix the problem.
    #[test]
    fn a_critical_error_blocks_only_generate() {
        assert!(!row_enabled(Screen::Generate, true, true));
        assert!(row_enabled(Screen::Entities, true, true));
        assert!(row_enabled(Screen::Project, true, true));
    }
}
