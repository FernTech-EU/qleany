//! The User Interface screen.

use teksilo::core::BindingLevel;
use teksilo::prelude::*;
use teksilo::widgets::{Card, Checkbox, FormLayout, Padding, ScrollArea, VStack};

use crate::project::Language;
use crate::shared::form::{field_label, heading};
use crate::user_interface::UserInterfaceViewModel;

/// A named widget rather than a free function: which checkboxes exist depends on
/// the project's language, and a set of rows that appears and disappears is a
/// different tree rather than a different value.
pub struct UserInterfacePage {
    vm: UserInterfaceViewModel,
    root_child: Option<WidgetId>,
}

impl std::fmt::Debug for UserInterfacePage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserInterfacePage").finish_non_exhaustive()
    }
}

impl UserInterfacePage {
    pub fn new(vm: UserInterfaceViewModel) -> Self {
        Self {
            vm,
            root_child: None,
        }
    }
}

impl Widget for UserInterfacePage {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        self.vm.wire(ctx);
        // Bound on the source signal, not on a `.map(..)` of it, which would be
        // dropped at the end of the statement and never fire again.
        self.vm
            .language()
            .bind_to(ctx.self_id(), ctx.binding_registry(), BindingLevel::Rebuild);

        let language = self.vm.language().get().unwrap_or_default();
        let mut form = FormLayout::new()
            .label_gap(12.0)
            .row_spacing(12.0)
            .label(section_title(language));

        for target in self.vm.targets() {
            let target = *target;
            let vm = self.vm.clone();
            form = form.line(
                field_label(target.label()),
                Checkbox::new(self.vm.flag(target))
                    .label(target.label())
                    .labelled_externally()
                    // `on_change`, not an effect over the bound signal: a reload
                    // writes that signal too, and an effect would push an undo entry
                    // every time the manifest reloaded.
                    .on_change(move |value, _c| vm.set(target, value)),
            );
        }

        let id = ctx.add(teksu!(
            ScrollArea {
                Padding::new(20.0, 24.0, 20.0, 24.0) {
                    VStack {
                        spacing: 16.0
                        child: heading(tr!(ui_title()))
                        child: heading(section_title(language))
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

/// What the set of targets is called, which is the only thing the language changes
/// besides the rows themselves.
fn section_title(language: Language) -> LocalizedString {
    match language {
        Language::Rust => tr!(ui_section_rust()),
        Language::CppQt => tr!(ui_section_cpp_qt()),
    }
}
