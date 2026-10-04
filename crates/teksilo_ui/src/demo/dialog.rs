//! The demo dialog: one modal with three faces.
//!
//! A `Stepper` would be the wrong shape here even though the Slint UI called this a
//! wizard. There is one page of choices, then a page that runs, then a page that
//! reports; nothing is revisited, and Back has no meaning once a project is on
//! disk. So this is a dialog whose body is swapped by the phase, not a stepper.

use teksilo::core::BindingLevel;
use teksilo::core::modal::{ModalCloseBehavior, ModalPresentation, ModalRequest};
use teksilo::core::styles::PanelVariant;
use teksilo::prelude::*;
use teksilo::widgets::{
    Button, ButtonVariant, DialogContent, Expand, FixedSize, HStack, IconButton, IconButtonSize,
    ModalContainer, Panel, ProgressBar, RadioTile, RadioTileGroup, TextInput, TextWidget, VStack,
};

use crate::demo::DemoViewModel;
use crate::demo::demo_vm::{DEFAULT_DESTINATION, Phase, ui_line};
use crate::project::Language;
use crate::shared::form::field_label;

/// How big the modal is: wide enough for the next-step command, tall enough for the
/// summary, which is the longest of the three faces.
const DIALOG_SIZE: (f32, f32) = (640.0, 460.0);

/// Open it.
pub fn present(vm: &DemoViewModel, ctx: &mut EventContext) {
    vm.reset();
    let vm = vm.clone();
    ctx.present_modal(
        ModalRequest::deferred(move |tree| {
            tree.add(ModalContainer::new(
                FixedSize::new()
                    .width(DIALOG_SIZE.0)
                    .height(DIALOG_SIZE.1)
                    .child(DemoDialog::new(vm.clone())),
            ))
        })
        .presentation(ModalPresentation::Auto)
        // US-DEMO-03: neither the scrim nor Escape may interrupt a run. A close
        // behaviour is fixed when the modal is presented and cannot be made
        // reactive, so the dialog closes itself through its own button, which is
        // the one thing that can be disabled while a run is in flight.
        .close_behavior(ModalCloseBehavior::Manual)
        .title(tr!(demo_title()).resolve_now()),
    );
}

/// The modal's body, rebuilt when the phase changes.
struct DemoDialog {
    vm: DemoViewModel,
    root_child: Option<WidgetId>,
}

impl std::fmt::Debug for DemoDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DemoDialog").finish_non_exhaustive()
    }
}

impl DemoDialog {
    fn new(vm: DemoViewModel) -> Self {
        Self {
            vm,
            root_child: None,
        }
    }
}

impl Widget for DemoDialog {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        // Two source signals, not one derived from them: a `.map(..)` temporary is
        // dropped at the end of the statement and the binding would be registered
        // against something that no longer exists. The error is in the list because
        // a failed run comes back to the form and the sentence saying why is the
        // only thing that changed.
        self.vm
            .phase()
            .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);
        self.vm
            .error()
            .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);

        let phase = self.vm.phase().get();
        let content = DialogContent::new()
            .title(tr!(demo_title()))
            .body(match phase {
                Phase::Form => Box::new(form(&self.vm)) as Box<dyn Widget>,
                Phase::Running => Box::new(running(&self.vm)),
                Phase::Done => Box::new(summary(&self.vm)),
            })
            .footer(footer(&self.vm, phase));

        let id = ctx.add(content);
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

/// Face one: what to generate, and where.
fn form(vm: &DemoViewModel) -> impl Widget {
    let selected = vm.language_index();
    let mut languages = RadioTileGroup::new(selected.clone()).label(tr!(demo_language()));
    for (index, language) in Language::ALL.into_iter().enumerate() {
        languages = languages.tile(
            RadioTile::new()
                .selection(index, selected.clone())
                .title(language.label())
                .description(match language {
                    Language::Rust => tr!(demo_language_rust_blurb()),
                    Language::CppQt => tr!(demo_language_cpp_qt_blurb()),
                }),
        );
    }

    let browse = vm.clone();
    let destination = teksu!(
        HStack {
            spacing: 8.0
            Expand {
                child: TextInput::new(vm.destination()) {
                    label: tr!(demo_destination())
                    placeholder: lit!(DEFAULT_DESTINATION.to_string())
                }
            }
            Button::new(tr!(demo_browse())) {
                on_activate_fn: move |c| browse.browse(c)
            }
        }
    );

    let mut column = teksu!(
        VStack {
            spacing: 12.0
            TextWidget::new(tr!(demo_blurb())) {
                color: TextRole::Secondary
            }
            child: languages
            child: field_label(tr!(demo_destination()))
            child: destination
            TextWidget::new(tr!(demo_destination_hint())) {
                style: TextStyleRole::Small
                color: TextRole::Secondary
            }
        }
    );

    // US-DEMO-06: the reason sits with the field the user has to change, not in a
    // message box they dismiss before reading the path.
    if let Some(error) = vm.error().get() {
        column = column.child(TextWidget::new(error).color(TextRole::Error));
    }
    column
}

/// Face two: it is running, and nothing can stop it.
fn running(vm: &DemoViewModel) -> impl Widget {
    teksu!(
        VStack {
            spacing: 12.0
            Expand::vertical
            TextWidget::new(tr!(demo_running_title())) {
                style: TextStyleRole::BodyBold
            }
            TextWidget::new(lit!(String::new())) {
                // The step message, straight from the pipeline. Bound rather than
                // read: the whole point of this face is that it changes while it is
                // on screen.
                text: vm.message()
                color: TextRole::Secondary
            }
            ProgressBar::new(0.0) {
                value: vm.progress()
            }
            Expand::vertical
        }
    )
}

/// Face three: what you got, and what to do with it.
fn summary(vm: &DemoViewModel) -> impl Widget {
    let language = vm.language();
    let mut includes = VStack::new()
        .spacing(4.0)
        .child(TextWidget::new(tr!(demo_includes_title())).style(TextStyleRole::SmallBold));
    for bullet in [
        tr!(demo_includes_crud()),
        tr!(demo_includes_undo()),
        tr!(demo_includes_events()),
        tr!(demo_includes_relationships()),
        tr!(demo_includes_tests()),
        ui_line(language),
    ] {
        includes = includes.child(
            TextWidget::new(bullet)
                .style(TextStyleRole::Small)
                .color(TextRole::Secondary),
        );
    }

    teksu!(
        VStack {
            spacing: 12.0
            TextWidget::new(tr!(demo_success())) {
                style: TextStyleRole::BodyBold
                color: TextRole::Success
            }
            TextWidget::new(vm.summary_stats())
            Panel::new() {
                variant: PanelVariant::Sunken
                padding: 12.0
                child: includes
            }
            TextWidget::new(tr!(demo_next_step())) {
                style: TextStyleRole::SmallBold
            }
            child: command_strip(vm)
        }
    )
}

/// The command, in monospace, with the button that copies it.
fn command_strip(vm: &DemoViewModel) -> impl Widget {
    teksu!(
        HStack {
            spacing: 8.0
            Expand {
                child: Panel::new() {
                    variant: PanelVariant::Sunken
                    padding: 8.0
                    child: TextWidget::new(lit!(vm.next_command())) {
                        style: TextStyleRole::Mono
                    }
                }
            }
            child: CopyButton::new(vm.clone())
        }
    )
}

/// The copy button, which changes glyph once it has copied.
///
/// A widget of its own rather than part of the summary, because the glyph is a
/// value rather than a signal: rebuilding is the only way to change it, and
/// rebuilding the whole summary to swap one icon would take the text selection in
/// the command beside it with it.
struct CopyButton {
    vm: DemoViewModel,
    root_child: Option<WidgetId>,
}

impl std::fmt::Debug for CopyButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CopyButton").finish_non_exhaustive()
    }
}

impl CopyButton {
    fn new(vm: DemoViewModel) -> Self {
        Self {
            vm,
            root_child: None,
        }
    }
}

impl Widget for CopyButton {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        self.vm
            .copied()
            .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);
        let copied = self.vm.copied().get();

        let vm = self.vm.clone();
        // The tooltip is the accessible name, so it says what happened rather than
        // only what the button does: a screen reader gets the confirmation the
        // glyph gives everyone else.
        let button = IconButton::new(if copied {
            crate::icons::action::check_ok()
        } else {
            crate::icons::action::copy()
        })
        .size(IconButtonSize::Toolbar)
        .tooltip(if copied {
            tr!(demo_copied())
        } else {
            tr!(demo_copy())
        })
        .on_activate_fn(move |c| vm.copy_next_command(c));

        let id = ctx.add(button);
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

/// The buttons, which differ by phase.
///
/// While a run is in flight the Close button is present and disabled rather than
/// absent: a button that vanishes reads as a bug, and a disabled one says that
/// there is a way out, just not yet.
fn footer(vm: &DemoViewModel, phase: Phase) -> impl Widget {
    let mut row = HStack::new().spacing(8.0).child(Expand::horizontal());

    match phase {
        Phase::Form => {
            let start = vm.clone();
            row = row
                .child(Button::new(tr!(common_cancel())).on_activate_fn(|c| c.dismiss_modal()))
                .child(
                    Button::new(tr!(demo_generate()))
                        .variant(ButtonVariant::Filled)
                        .on_activate_fn(move |_c| start.start()),
                );
        }
        Phase::Running => {
            row = row.child(Button::new(tr!(common_close())).enabled(false));
        }
        Phase::Done => {
            let open = vm.clone();
            row = row
                .child(Button::new(tr!(common_close())).on_activate_fn(|c| c.dismiss_modal()))
                .child(
                    Button::new(tr!(demo_open_folder()))
                        .variant(ButtonVariant::Filled)
                        .on_activate_fn(move |_c| open.open_result_folder()),
                );
        }
    }
    row
}
