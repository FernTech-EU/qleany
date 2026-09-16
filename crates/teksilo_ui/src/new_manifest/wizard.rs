//! Presenting the new-manifest wizard.
//!
//! `Wizard` is a button that opens a modal, and this wizard is opened by a menu row
//! and by a button on Home, neither of which is that button. So the modal is
//! presented the way `Wizard` presents its own: a `Stepper` inside a
//! `ModalContainer`, handed to `EventContext::present_modal`.

use teksilo::core::modal::{ModalCloseBehavior, ModalPresentation, ModalRequest};
use teksilo::platform::file_dialog::{FileDialogRequest, FileDialogResult};
use teksilo::prelude::*;
use teksilo::widgets::stepper::FinishOutcome;
use teksilo::widgets::{
    FixedSize, FormLayout, ModalContainer, RadioTile, RadioTileGroup, Step, Stepper, TextInput,
    TextWidget, VStack,
};

use crate::manifest::ManifestViewModel;
use crate::new_manifest::NewManifestViewModel;
use crate::new_manifest::new_manifest_vm::{Template, offered_targets};
use crate::project::Language;
use crate::shared::form::{inline_checkbox, required_field_label};

/// How big the modal is. Four steps of a form, with room for the tallest of them.
const WIZARD_SIZE: (f32, f32) = (720.0, 480.0);

/// Open the wizard.
///
/// `manifest` is what a successful create loads through, so the rest of the app
/// hears about the new manifest the same way it hears about an opened one.
pub fn present(vm: &NewManifestViewModel, manifest: &ManifestViewModel, ctx: &mut EventContext) {
    let steps = vec![
        language_step(vm),
        names_step(vm),
        template_step(vm),
        targets_step(vm),
    ];

    let finish_vm = vm.clone();
    let finish_manifest = manifest.clone();
    ctx.present_modal(
        ModalRequest::deferred(move |tree| {
            let vm = finish_vm.clone();
            let manifest = finish_manifest.clone();
            let stepper = Stepper::new()
                .steps(steps)
                .back_label(tr!(wizard_back()))
                .next_label(tr!(wizard_next()))
                .finish_label(tr!(wizard_create()))
                .cancel(tr!(wizard_cancel()), |ctx, _ctrl| ctx.dismiss_modal())
                .on_finish(move |ctx, _ctrl| finish(&vm, &manifest, ctx));
            // The size is on the content, not on the request. An in-tree modal is
            // laid out to its content; `ModalRequest::size` is for the native-window
            // presentation, and a stepper left to its own devices collapses to the
            // height of its tallest row, with the buttons over the step.
            tree.add(ModalContainer::new(
                FixedSize::new()
                    .width(WIZARD_SIZE.0)
                    .height(WIZARD_SIZE.1)
                    .child(stepper),
            ))
        })
        .presentation(ModalPresentation::Auto)
        // Escape and the scrim dismiss without creating anything: the wizard has
        // written nothing until Create.
        .close_behavior(ModalCloseBehavior::default())
        .title(tr!(wizard_title()).resolve_now()),
    );
}

/// Create is pressed.
///
/// The save dialog runs *after* the finish returns, which is why this always
/// reports `Finished` and dismisses: a modal cannot wait on an asynchronous dialog,
/// and a wizard that stayed open behind a file picker would be two modals deep. A
/// create that then fails reports itself through the manifest view-model's error,
/// which the shell already shows.
fn finish(
    vm: &NewManifestViewModel,
    manifest: &ManifestViewModel,
    ctx: &mut EventContext,
) -> FinishOutcome {
    let vm = vm.clone();
    let manifest = manifest.clone();
    let request = FileDialogRequest::save_file()
        .title("Create a Qleany manifest")
        .add_filter("Qleany manifest", &["yaml", "yml"])
        .default_file_name("qleany.yaml");

    let _ = ctx.pick_file(request, move |result, _ectx| {
        let FileDialogResult::File(Some(path)) = result else {
            return;
        };
        match vm.create(&path.to_string_lossy()) {
            // The create writes the file; loading it is what fills the store and
            // tells every screen about it.
            Ok(written) => manifest.open_path(&written),
            Err(e) => log::error!("could not create the manifest: {e}"),
        }
    });
    FinishOutcome::Finished
}

/// Step 1: the target language.
fn language_step(vm: &NewManifestViewModel) -> Step {
    let vm = vm.clone();
    Step::new(tr!(wizard_step_language())).content(move || {
        // The group writes the view-model's own signal, so there is nothing to
        // observe and nothing to mirror: `chosen_language` reads the same signal
        // the radio wrote, and step 4 reconciles its ticks when it is built.
        let selected = vm.language_index();
        RadioTileGroup::new(selected.clone())
            .label(tr!(wizard_step_language()))
            .tile(
                RadioTile::new()
                    .selection(0, selected.clone())
                    .title(tr!(wizard_language_rust()))
                    .description(tr!(wizard_language_rust_blurb())),
            )
            .tile(
                RadioTile::new()
                    .selection(1, selected.clone())
                    .title(tr!(wizard_language_cpp_qt()))
                    .description(tr!(wizard_language_cpp_qt_blurb())),
            )
    })
}

/// Step 2: the two names. The only step that gates Next.
fn names_step(vm: &NewManifestViewModel) -> Step {
    let gate = vm.names_are_valid();
    let vm = vm.clone();
    Step::new(tr!(wizard_step_names()))
        .complete_when(gate)
        .content(move || {
            FormLayout::new()
                .label_gap(12.0)
                .row_spacing(12.0)
                .label(tr!(wizard_step_names()))
                .line(
                    required_field_label(tr!(wizard_application_name())),
                    TextInput::new(vm.application_name())
                        .label(tr!(wizard_application_name()))
                        .placeholder(tr!(wizard_application_name_placeholder()))
                        .validation(vm.application_name_validation()),
                )
                .line(
                    required_field_label(tr!(wizard_organisation_name())),
                    TextInput::new(vm.organisation_name())
                        .label(tr!(wizard_organisation_name()))
                        .placeholder(tr!(wizard_organisation_name_placeholder()))
                        .validation(vm.organisation_name_validation()),
                )
        })
}

/// Step 3: what the manifest starts as.
fn template_step(vm: &NewManifestViewModel) -> Step {
    let vm = vm.clone();
    Step::new(tr!(wizard_step_template())).content(move || {
        let selected = vm.template_index();
        let mut group = RadioTileGroup::new(selected.clone()).label(tr!(wizard_step_template()));
        for (index, template) in Template::ALL.into_iter().enumerate() {
            group = group.tile(
                RadioTile::new()
                    .selection(index, selected.clone())
                    .title(template.title())
                    .description(template.description()),
            );
        }
        group
    })
}

/// Step 4: which frontends to scaffold.
///
/// A named widget rather than a closure, because a `Step`'s content factory runs
/// once when the stepper is built rather than each time the step is shown: a
/// closure reading the language would read it before the user had chosen one, and
/// a C++/Qt run would offer the Rust frontends.
fn targets_step(vm: &NewManifestViewModel) -> Step {
    let vm = vm.clone();
    Step::new(tr!(wizard_step_targets())).content(move || TargetsStep::new(vm.clone()))
}

struct TargetsStep {
    vm: NewManifestViewModel,
    root_child: Option<WidgetId>,
}

impl std::fmt::Debug for TargetsStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TargetsStep").finish_non_exhaustive()
    }
}

impl TargetsStep {
    fn new(vm: NewManifestViewModel) -> Self {
        Self {
            vm,
            root_child: None,
        }
    }
}

impl Widget for TargetsStep {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        // Bound on the source signal, not on a `.map(..)` temporary, which would be
        // dropped at the end of the statement and never fire again.
        self.vm.language_index().bind_to(
            ctx.self_id(),
            ctx.binding_registry(),
            teksilo::core::BindingLevel::Rebuild,
        );

        let language = self.vm.chosen_language();
        let mut column = VStack::new().spacing(8.0);
        for target in offered_targets(language) {
            let ticked = Signal::new(self.vm.has_target(target));
            let vm = self.vm.clone();
            column = column.child(inline_checkbox(target.label(), ticked, move |on, _c| {
                vm.set_target(target, on)
            }));
        }
        if language == Language::CppQt {
            column = column.child(
                TextWidget::new(tr!(wizard_targets_cpp_note()))
                    .style(TextStyleRole::Small)
                    .color(TextRole::Secondary),
            );
        }
        let column = column.child(
            TextWidget::new(tr!(wizard_targets_footnote()))
                .style(TextStyleRole::Small)
                .color(TextRole::Secondary),
        );

        let id = ctx.add(column);
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
