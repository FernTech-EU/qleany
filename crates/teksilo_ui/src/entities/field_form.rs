//! The form for the selected field.
//!
//! Which rows exist is decided by the `shows_*` rules on
//! [`crate::entities::field_vm::FieldShape`], and every
//! control writes through `FieldViewModel::apply`, so no control can change the one
//! thing it names without also clearing whatever that change rules out.

use teksilo::prelude::*;
use teksilo::widgets::{
    Checkbox, ComboBox, FixedSize, FormLayout, PlainTextEditor, TextInput, VStack,
};

use frontend::EntityId;

use crate::entities::field_vm::{
    FIELD_TYPES, RELATIONSHIP_TYPES, field_type_name, relationship_name,
};
use crate::entities::{EntitiesViewModel, FieldViewModel};
use crate::models::WorkspaceEntitiesRow;
use crate::shared::form::{field_label, required_field_label};

/// The height of the enum values editor. Four lines plus its frame: enough to show
/// a small enum whole, and to make it obvious that it takes more than one line.
const ENUM_EDITOR_HEIGHT: f32 = 96.0;

/// What the "no referenced entity" row of the combo stands for. See the entity
/// form's `NO_PARENT`, which is the same idea.
const NO_ENTITY: EntityId = 0;

pub fn field_form(vm: &FieldViewModel, entities: &EntitiesViewModel) -> impl Widget {
    let shape = vm.shape();

    let name = {
        let on_submit = vm.clone();
        let on_blur = vm.clone();
        TextInput::new(vm.name())
            .label(tr!(fields_name()))
            .placeholder(tr!(fields_name_placeholder()))
            .validation(vm.name_validation())
            .on_submit_fn(move |_c| on_submit.commit())
            .on_blur_fn(move |_c| on_blur.commit())
    };

    let type_combo = {
        let vm = vm.clone();
        let selected = Signal::new(Some(shape.field_type.clone()));
        ComboBox::from_items(FIELD_TYPES, selected, |t| lit!(field_type_name(t)))
            .label(tr!(fields_type()))
            .on_select(move |chosen, _c| {
                let chosen = chosen.clone();
                vm.apply(move |shape| shape.with_type(chosen));
            })
    };

    let mut form = FormLayout::new()
        .label_gap(12.0)
        .row_spacing(12.0)
        .label(tr!(fields_list_heading()))
        .line(required_field_label(tr!(fields_name())), name)
        .line(required_field_label(tr!(fields_type())), type_combo);

    if shape.shows_referenced_entity() {
        form = form.line(
            field_label(tr!(fields_referenced_entity())),
            entity_combo(vm, entities, shape.entity),
        );
    }

    if shape.shows_relationship() {
        let on_select = vm.clone();
        let selected = Signal::new(Some(shape.relationship.clone()));
        form = form.line(
            field_label(tr!(fields_relationship())),
            ComboBox::from_items(RELATIONSHIP_TYPES, selected, |r| lit!(relationship_name(r)))
                .label(tr!(fields_relationship()))
                .on_select(move |chosen, _c| {
                    let chosen = chosen.clone();
                    on_select.apply(move |shape| shape.with_relationship(chosen));
                }),
        );
    }

    if shape.shows_enum() {
        let on_submit = vm.clone();
        let on_blur = vm.clone();
        let enum_name = TextInput::new(vm.enum_name())
            .label(tr!(fields_enum_name()))
            .placeholder(tr!(fields_enum_name_placeholder()))
            .validation(vm.enum_name_validation())
            .on_submit_fn(move |_c| commit_enum_name(&on_submit))
            .on_blur_fn(move |_c| commit_enum_name(&on_blur));
        form = form.line(required_field_label(tr!(fields_enum_name())), enum_name);

        form = form.line(
            field_label(tr!(fields_enum_values())),
            enum_values_editor(vm),
        );
    }

    if shape.shows_optional() {
        form = form.line(
            field_label(tr!(fields_optional())),
            toggle(vm, vm.optional(), tr!(fields_optional()), |shape, on| {
                shape.with_optional(on)
            }),
        );
    }

    if shape.shows_is_list() {
        form = form.line(
            field_label(tr!(fields_is_list())),
            toggle(vm, vm.is_list(), tr!(fields_is_list()), |shape, on| {
                shape.with_is_list(on)
            }),
        );
    }

    if shape.shows_list_model() {
        form = form.line(
            field_label(tr!(fields_list_model())),
            toggle(
                vm,
                vm.list_model(),
                tr!(fields_list_model()),
                |shape, on| shape.with_list_model(on),
            ),
        );
    }

    if shape.shows_displayed_field() {
        let on_submit = vm.clone();
        let on_blur = vm.clone();
        form = form.line(
            field_label(tr!(fields_displayed_field())),
            TextInput::new(vm.displayed_field())
                .label(tr!(fields_displayed_field()))
                .placeholder(tr!(fields_displayed_field_placeholder()))
                .on_submit_fn(move |_c| commit_displayed_field(&on_submit))
                .on_blur_fn(move |_c| commit_displayed_field(&on_blur)),
        );
    }

    if shape.shows_strong() {
        form = form.line(
            field_label(tr!(fields_strong())),
            toggle(vm, vm.strong(), tr!(fields_strong()), |shape, on| {
                shape.with_strong(on)
            }),
        );
    }

    teksu!(
        VStack {
            spacing: 12.0
            child: form
        }
    )
}

/// One boolean row.
///
/// `on_change` rather than an effect over the bound signal, for the reason the
/// entity form gives: a reload writes that signal too, and an effect would make
/// every reload look like an edit.
fn toggle(
    vm: &FieldViewModel,
    checked: Signal<bool>,
    label: LocalizedString,
    rule: fn(crate::entities::field_vm::FieldShape, bool) -> crate::entities::field_vm::FieldShape,
) -> Checkbox {
    let vm = vm.clone();
    Checkbox::new(checked)
        .label(label)
        .labelled_externally()
        .on_change(move |on, _c| vm.apply(move |shape| rule(shape, on)))
}

/// The referenced entity combo: "None", then every entity that can be instantiated.
///
/// Heritage-only entities are excluded: a field cannot point at something the
/// generator never creates a table for.
fn entity_combo(
    vm: &FieldViewModel,
    entities: &EntitiesViewModel,
    current: Option<EntityId>,
) -> impl Widget {
    let mut options = vec![WorkspaceEntitiesRow {
        id: NO_ENTITY,
        name: tr!(common_none()).resolve_now(),
        ..Default::default()
    }];
    options.extend(
        entities
            .list()
            .rows()
            .into_iter()
            .filter(|row| !row.only_for_heritage),
    );

    let current = current.unwrap_or(NO_ENTITY);
    let selected = Signal::new(
        options
            .iter()
            .find(|row| row.id == current)
            .or_else(|| options.first())
            .cloned(),
    );

    let vm = vm.clone();
    ComboBox::from_items(options, selected, |row| lit!(row.name.clone()))
        .label(tr!(fields_referenced_entity()))
        .on_select(move |row, _c| {
            let chosen = (row.id != NO_ENTITY).then_some(row.id);
            vm.apply(move |shape| shape.with_entity(chosen));
        })
}

/// The enum variants, one per line.
///
/// A plain multi-line editor rather than a list of rows: a variant can carry a whole
/// struct body, and a row-per-variant editor would have to parse one to show it.
///
/// The document is seeded here and read back by the view-model, which owns both the
/// handle and the commit. The editor reports a version rather than a blur, so the
/// commit waits for typing to stop; see [`FieldViewModel::wire`].
fn enum_values_editor(vm: &FieldViewModel) -> impl Widget {
    let document = vm.enum_document();
    let editor = PlainTextEditor::new(document).min_lines(4);
    teksu!(
        FixedSize {
            height: ENUM_EDITOR_HEIGHT
            child: editor
        }
    )
}

fn commit_enum_name(vm: &FieldViewModel) {
    let value = vm.enum_name().get();
    vm.set_enum_name(&value);
    vm.commit();
}

fn commit_displayed_field(vm: &FieldViewModel) {
    let value = vm.displayed_field().get();
    vm.set_displayed_field(&value);
    vm.commit();
}
