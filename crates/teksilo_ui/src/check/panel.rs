//! The validation badge, and the panel it opens.
//!
//! The badge sits in the title bar beside the save button and is visible whenever a
//! manifest is open, so "is this manifest valid" never has to be asked. The panel is
//! what it opens: every problem, criticals first.

use teksilo::core::BindingLevel;
use teksilo::core::overlay::OverlayPlacement;
use teksilo::prelude::*;
use teksilo::widgets::{
    Divider, Expand, HStack, IconButton, IconButtonSize, Padding, PopoverIconButton, ScrollArea,
    TextWidget, VStack,
};

use crate::check::{CheckStatus, CheckViewModel};
use crate::shared::pane::{FixedHeight, FixedWidth};

/// How wide the panel is. Wide enough for a full sentence from the rule catalogue
/// without wrapping every line of it.
const PANEL_WIDTH: f32 = 420.0;

/// How tall it may grow before it scrolls.
const PANEL_MAX_HEIGHT: f32 = 380.0;

pub struct CheckBadge {
    vm: CheckViewModel,
    root_child: Option<WidgetId>,
}

impl std::fmt::Debug for CheckBadge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckBadge").finish_non_exhaustive()
    }
}

impl CheckBadge {
    pub fn new(vm: CheckViewModel) -> Self {
        Self {
            vm,
            root_child: None,
        }
    }
}

/// The glyph, its colour and its tooltip for a status.
fn appearance(
    status: CheckStatus,
) -> (
    fn() -> teksilo::widgets::IconWidget,
    TextRole,
    LocalizedString,
) {
    match status {
        CheckStatus::Critical => (
            crate::icons::action::check_critical,
            TextRole::Error,
            tr!(check_tooltip_critical()),
        ),
        CheckStatus::Warning => (
            crate::icons::action::check_warning,
            TextRole::Warning,
            tr!(check_tooltip_warning()),
        ),
        CheckStatus::Ok | CheckStatus::None => (
            crate::icons::action::check_ok,
            TextRole::Success,
            tr!(check_tooltip_ok()),
        ),
    }
}

impl Widget for CheckBadge {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        // The glyph is chosen at build time, as the save button's is: `IconButton`
        // takes its icon by value, so swapping it means rebuilding. Bound on the
        // source signal, never on a `.map(..)` temporary, which would be dropped at
        // the end of this statement.
        self.vm
            .status()
            .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);
        let status = self.vm.status().get();
        let (icon, role, tooltip) = appearance(status);

        let button = IconButton::new(icon())
            .size(IconButtonSize::Toolbar)
            .icon_role(role)
            .tooltip(tooltip);

        // The popover owns whether it is open: it already closes on Escape and on a
        // click outside, which is two thirds of what the panel has to do, and a
        // second flag beside it would be a second thing to keep in step.
        let popover = PopoverIconButton::new(button).placement(OverlayPlacement::Below);
        let open = popover.open_signal();
        let popover = popover.content(panel_body(&self.vm, open.clone()));

        // The count beside the glyph rather than drawn over it: a number inside a
        // 16 dp icon is unreadable, and the panel is where the detail belongs.
        let count = TextWidget::new(lit!(String::new()))
            .text(self.vm.badge())
            .style(TextStyleRole::Tiny)
            .color(role);

        let id = ctx.add(teksu!(
            HStack {
                spacing: 2.0
                child: popover
                child: count
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

/// Every problem, criticals first.
fn panel_body(vm: &CheckViewModel, open: Signal<bool>) -> impl Widget {
    let problems = vm.problems().get();

    let mut rows = VStack::new().spacing(8.0);
    if problems.is_empty() {
        rows = rows.child(TextWidget::new(tr!(check_all_clear())).color(TextRole::Success));
    }
    for (critical, message) in problems {
        let (icon, role) = if critical {
            (crate::icons::action::check_critical(), TextRole::Error)
        } else {
            (crate::icons::action::check_warning(), TextRole::Warning)
        };
        rows = rows.child(
            HStack::new()
                .spacing(8.0)
                .child(icon.color(role))
                // The backend's own sentence from the rule catalogue, not a summary
                // of it: the user has to be able to find the rule that failed.
                .child(TextWidget::new(lit!(message))),
        );
    }

    let recheck = vm.clone();
    let header = teksu!(
        Padding::new(8.0, 12.0, 8.0, 12.0) {
            HStack {
                spacing: 8.0
                TextWidget::new(tr!(check_panel_title())) {
                    style: TextStyleRole::BodyBold
                }
                Expand::horizontal
                IconButton::new(crate::icons::action::refresh()) {
                    size: IconButtonSize::Toolbar
                    tooltip: tr!(check_recheck())
                    on_activate_fn: move |_c| recheck.run()
                }
                IconButton::new(crate::icons::action::close()) {
                    size: IconButtonSize::Toolbar
                    tooltip: tr!(common_close())
                    on_activate_fn: move |_c| open.set(false)
                }
            }
        }
    );

    let body = FixedHeight::new(
        PANEL_MAX_HEIGHT,
        teksu!(
            ScrollArea {
                Padding::new(12.0, 12.0, 12.0, 12.0) {
                    child: rows
                }
            }
        ),
    );

    FixedWidth::new(
        PANEL_WIDTH,
        teksu!(
            VStack {
                spacing: 0.0
                child: header
                Divider::horizontal
                child: body
            }
        ),
    )
}
