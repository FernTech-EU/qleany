//! The Project settings screen.

use teksilo::core::BindingLevel;
use teksilo::prelude::*;
use teksilo::widgets::{
    Card, ComboBox, FormLayout, Padding, ScrollArea, TextInput, VStack, ValidationState,
};

use crate::project::project_vm::{Language, ProjectViewModel};
use crate::shared::form::{field_label, heading, required_field_label};

/// How wide a field asks to be before the form starts sharing out what is left.
/// Enough for a reverse domain name without scrolling.
const FIELD_MIN_WIDTH: f32 = 320.0;

/// A named widget rather than a free function, for two reasons that both come back
/// to the same place: the view-model's effects have to be installed on every build,
/// and the prefix-path placeholder names the selected language's default while
/// `TextInput::placeholder` takes a value rather than a signal. Both need a `build`
/// of our own.
pub struct ProjectPage {
    vm: ProjectViewModel,
    root_child: Option<WidgetId>,
}

impl std::fmt::Debug for ProjectPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectPage").finish_non_exhaustive()
    }
}

impl ProjectPage {
    pub fn new(vm: ProjectViewModel) -> Self {
        Self {
            vm,
            root_child: None,
        }
    }
}

impl Widget for ProjectPage {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        // Bound on the view-model's own signal, not on a `.map(..)` of it: a derived
        // signal built inline is dropped at the end of the statement, and the
        // binding would then be registered against something that no longer exists.
        self.vm
            .language()
            .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);
        let language = self.vm.language_now();

        let combo = {
            let vm = self.vm.clone();
            ComboBox::from_items(Language::ALL, self.vm.language(), |l| l.label())
                .label(tr!(project_language()))
                .on_select(move |language, _c| vm.set_language(*language))
        };

        let form = FormLayout::new()
            .label_gap(12.0)
            .row_spacing(12.0)
            .label(tr!(project_title()))
            .line(required_field_label(tr!(project_language())), combo)
            .line(
                required_field_label(tr!(project_application_name())),
                text_field(
                    &self.vm,
                    self.vm.application_name(),
                    tr!(project_application_name()),
                    tr!(project_application_name_placeholder()),
                    self.vm.application_name_validation(),
                ),
            )
            .line(
                required_field_label(tr!(project_organisation_name())),
                text_field(
                    &self.vm,
                    self.vm.organisation_name(),
                    tr!(project_organisation_name()),
                    tr!(project_organisation_name_placeholder()),
                    self.vm.organisation_name_validation(),
                ),
            )
            .line(
                required_field_label(tr!(project_organisation_domain())),
                text_field(
                    &self.vm,
                    self.vm.organisation_domain(),
                    tr!(project_organisation_domain()),
                    tr!(project_organisation_domain_placeholder()),
                    self.vm.organisation_domain_validation(),
                ),
            )
            .line(
                field_label(tr!(project_prefix_path())),
                text_field(
                    &self.vm,
                    self.vm.prefix_path(),
                    tr!(project_prefix_path()),
                    // The placeholder names the default the backend will apply, so
                    // it has to follow the language chosen above. That is what the
                    // rebuild binding at the top of this function is for.
                    tr!(project_prefix_path_placeholder(
                        path = language.default_prefix_path()
                    )),
                    Signal::new(ValidationState::None),
                ),
            );

        let id = ctx.add(teksu!(
            ScrollArea {
                Padding::new(20.0, 24.0, 20.0, 24.0) {
                    VStack {
                        spacing: 16.0
                        child: heading(tr!(project_title()))
                        Card {
                            padding: 16.0
                            content: form
                        }
                    }
                }
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

/// One text row of the form.
///
/// Both commit hooks call the same method. Enter and blur are the two moments a
/// field is finished with, and the view-model's commit is a no-op when nothing
/// changed, so a user who tabs through the form without typing writes nothing and
/// pushes no undo entry.
fn text_field(
    vm: &ProjectViewModel,
    value: Signal<String>,
    label: LocalizedString,
    placeholder: LocalizedString,
    validation: Signal<ValidationState>,
) -> TextInput {
    let on_submit = vm.clone();
    let on_blur = vm.clone();
    TextInput::new(value)
        .label(label)
        .placeholder(placeholder)
        .validation(validation)
        .min_width(FIELD_MIN_WIDTH)
        .on_submit_fn(move |_c| on_submit.commit())
        .on_blur_fn(move |_c| on_blur.commit())
}
