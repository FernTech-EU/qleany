//! The form for the selected use case, and the entities it touches.

use teksilo::prelude::*;
use teksilo::widgets::{
    Checkbox, FormLayout, ListView, StandardListItem, TextInput, TextWidget, VStack,
};

use crate::features::UseCaseViewModel;
use crate::features::use_case_vm::associable;
use crate::shared::form::{field_label, required_field_label};
use crate::shared::list_or_empty::{emptiness, empty_state, list_or_empty};
use crate::shared::pane::FixedHeight;

/// Height of the entity association list. Tall enough for half a dozen rows, which
/// is what a use case usually touches, and scrollable past that.
const ASSOCIATION_LIST_HEIGHT: f32 = 180.0;

/// Height of an association row: one line, no subtitle.
const ASSOCIATION_ROW_HEIGHT: f32 = 32.0;

pub fn use_case_form(vm: &UseCaseViewModel) -> impl Widget {
    let name = {
        let on_submit = vm.clone();
        let on_blur = vm.clone();
        TextInput::new(vm.name())
            .label(tr!(use_cases_name()))
            .placeholder(tr!(use_cases_name_placeholder()))
            .validation(vm.name_validation())
            .on_submit_fn(move |_c| on_submit.commit())
            .on_blur_fn(move |_c| on_blur.commit())
    };

    let read_only = {
        let vm = vm.clone();
        Checkbox::new(vm.read_only())
            .label(tr!(use_cases_read_only()))
            .caption(tr!(use_cases_read_only_hint()))
            .labelled_externally()
            .on_change(move |value, _c| vm.set_read_only(value))
    };

    let long_operation = {
        let vm = vm.clone();
        Checkbox::new(vm.long_operation())
            .label(tr!(use_cases_long_operation()))
            .labelled_externally()
            .on_change(move |value, _c| vm.set_long_operation(value))
    };

    let mut form = FormLayout::new()
        .label_gap(12.0)
        .row_spacing(12.0)
        .label(tr!(use_cases_list_heading()))
        .line(required_field_label(tr!(use_cases_name())), name)
        .line(field_label(tr!(use_cases_read_only())), read_only);

    // A read-only use case writes nothing, so there is nothing for it to undo. The
    // row is absent rather than disabled, for the same reason the entity form hides
    // what a heritage-only entity cannot have.
    if !vm.read_only().get() {
        let vm = vm.clone();
        form = form.line(
            field_label(tr!(use_cases_undoable())),
            Checkbox::new(vm.undoable())
                .label(tr!(use_cases_undoable()))
                .labelled_externally()
                .on_change(move |value, _c| vm.set_undoable(value)),
        );
    }

    form = form.line(field_label(tr!(use_cases_long_operation())), long_operation);

    teksu!(
        VStack {
            spacing: 16.0
            child: form
            TextWidget::new(tr!(use_cases_entities())) {
                style: TextStyleRole::BodyBold
            }
            child: entity_associations(vm)
        }
    )
}

/// The entities this use case touches, one tick each.
///
/// A plain `ListView` of checkable rows rather than `CheckedModel`, which is keyed
/// by index: this list is rebuilt whenever an entity is added or renamed, and an
/// index-keyed set of ticks would move to the wrong rows when it is.
fn entity_associations(vm: &UseCaseViewModel) -> impl Widget {
    let delegate_vm = vm.clone();
    let list = ListView::from_source(vm.associable_model(), move |_index, row, _selected| {
        let id = row.id;
        let vm = delegate_vm.clone();
        Box::new(
            StandardListItem::new(lit!(row.name.clone()))
                .checkbox(delegate_vm.tick(id))
                .on_checkbox_toggle(move |checked, _c| vm.set_associated(id, checked))
                .label_overflow(TextOverflow::Ellipsis(EllipsisMode::Trailing)),
        )
    })
    .auto_item_height(ASSOCIATION_ROW_HEIGHT);

    let empty = emptiness(&vm.entities().version_signal(), {
        let entities = vm.entities();
        move || associable(&entities).is_empty()
    });

    FixedHeight::new(
        ASSOCIATION_LIST_HEIGHT,
        list_or_empty(
            empty,
            list,
            empty_state(
                tr!(use_cases_entities_empty()),
                tr!(use_cases_entities_empty_hint()),
            ),
        ),
    )
}
