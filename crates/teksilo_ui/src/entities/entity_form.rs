//! The form for the selected entity.

use teksilo::prelude::*;
use teksilo::widgets::{Checkbox, ComboBox, FormLayout, TextInput, VStack};

use frontend::EntityId;

use crate::entities::EntitiesViewModel;
use crate::models::WorkspaceEntitiesRow;
use crate::shared::form::{field_label, required_field_label};

/// What the "no parent" row of the Inherits from combo stands for.
///
/// A sentinel row rather than an empty selection: a combo with nothing selected
/// looks like a combo nobody has filled in, and "None" is a real answer here.
const NO_PARENT: EntityId = 0;

/// The form. Rebuilt by [`crate::entities::EntitiesPage`] whenever the selection or
/// the heritage flag changes, which is what lets the three concrete-only rows appear
/// and disappear.
pub fn entity_form(vm: &EntitiesViewModel) -> impl Widget {
    let concrete = vm.concrete().get();

    let name = {
        let on_submit = vm.clone();
        let on_blur = vm.clone();
        TextInput::new(vm.name())
            .label(tr!(entities_name()))
            .placeholder(tr!(entities_name_placeholder()))
            .validation(vm.name_validation())
            .on_submit_fn(move |_c| on_submit.commit())
            .on_blur_fn(move |_c| on_blur.commit())
    };

    let heritage = {
        let vm = vm.clone();
        Checkbox::new(vm.only_for_heritage())
            .label(tr!(entities_only_for_heritage()))
            .caption(tr!(entities_only_for_heritage_hint()))
            .labelled_externally()
            // `on_change`, never an effect over the bound signal: a refresh writes
            // that signal too, so an effect would push an undo entry every time the
            // list reloaded.
            .on_change(move |value, _c| vm.set_only_for_heritage(value))
    };

    let mut form = FormLayout::new()
        .label_gap(12.0)
        .row_spacing(12.0)
        .label(tr!(entities_title()))
        .line(required_field_label(tr!(entities_name())), name)
        .line(field_label(tr!(entities_only_for_heritage())), heritage);

    // A heritage-only entity is never instantiated, so it has nothing to inherit
    // from, no row to bind a single to, and no edit to undo. The rows are hidden
    // rather than disabled: a disabled control invites the user to work out why.
    if concrete {
        form = form
            .line(field_label(tr!(entities_inherits_from())), parent_combo(vm))
            .line(field_label(tr!(entities_single_model())), {
                let vm = vm.clone();
                Checkbox::new(vm.single_model())
                    .label(tr!(entities_single_model()))
                    .labelled_externally()
                    .on_change(move |value, _c| vm.set_single_model(value))
            })
            .line(field_label(tr!(entities_undoable())), {
                let vm = vm.clone();
                Checkbox::new(vm.undoable())
                    .label(tr!(entities_undoable()))
                    .labelled_externally()
                    .on_change(move |value, _c| vm.set_undoable(value))
            });
    }

    teksu!(
        VStack {
            spacing: 12.0
            child: form
        }
    )
}

/// The Inherits from combo: "None" first, then the heritage-only entities.
///
/// `ComboBox::from_items` selects by **value**, so the selection has to be a row
/// rather than an id, and the sentinel needs a row of its own.
fn parent_combo(vm: &EntitiesViewModel) -> impl Widget {
    let mut options = vec![WorkspaceEntitiesRow {
        id: NO_PARENT,
        name: tr!(common_none()).resolve_now(),
        ..Default::default()
    }];
    options.extend(vm.heritage_options());

    let current = vm.inherits_from().unwrap_or(NO_PARENT);
    let selected = Signal::new(
        options
            .iter()
            .find(|row| row.id == current)
            .or_else(|| options.first())
            .cloned(),
    );

    let on_select = vm.clone();
    ComboBox::from_items(options, selected, |row| lit!(row.name.clone()))
        .label(tr!(entities_inherits_from()))
        .on_select(move |row, _c| {
            on_select.set_inherits_from((row.id != NO_PARENT).then_some(row.id));
        })
}
