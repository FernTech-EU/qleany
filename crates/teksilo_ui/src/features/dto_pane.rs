//! One DTO pane: enable, name, fields, and the selected field's form.
//!
//! Built twice per screen, once per side. It takes its view-model rather than
//! reaching for a shared one, which is what keeps editing the input DTO from
//! touching the output one.

use teksilo::prelude::*;
use teksilo::widgets::{
    ActivateOn, Checkbox, ComboBox, Divider, Expand, FixedSize, FormLayout, HStack, IconButton,
    IconButtonSize, ListView, MenuItem, MenuList, MessageBox, MessageBoxButtons, Padding,
    PlainTextEditor, StandardButton, StandardListItem, TextInput, TextWidget, VStack,
};

use crate::features::DtoViewModel;
use crate::features::dto_vm::{DTO_FIELD_TYPES, dto_field_subtitle, dto_field_type_name};
use crate::shared::form::{field_label, required_field_label};
use crate::shared::list_or_empty::{emptiness, empty_state, list_or_empty};
use crate::shared::reorder::ReorderableSource;
use crate::shared::row_menu::row_menu_button;

/// Height of the field list inside a pane. Two panes sit side by side under a form,
/// so neither can take the whole column.
const FIELD_LIST_HEIGHT: f32 = 160.0;

const ROW_HEIGHT: f32 = 44.0;

/// Height of the enum values editor.
const ENUM_EDITOR_HEIGHT: f32 = 88.0;

pub fn dto_pane(vm: &DtoViewModel) -> impl Widget {
    let enabled = vm.enabled().get();

    let toggle = {
        let vm = vm.clone();
        Checkbox::new(vm.enabled())
            .label(vm.side().enable_label())
            .on_change(move |wanted, ctx| {
                if wanted {
                    vm.enable();
                } else {
                    confirm_disable(&vm, ctx);
                }
            })
    };

    // No heading of its own: the tab the pane sits in already names it, and saying
    // "Input DTO" twice in adjacent lines reads as a mistake.
    let header = toggle;

    let body: Box<dyn Widget> = if enabled {
        Box::new(enabled_body(vm))
    } else {
        // US-DTO-03: one line saying what the pane is for, and no other control.
        Box::new(
            Padding::symmetric(8.0, 24.0)
                .child(TextWidget::new(vm.side().disabled_hint()).color(TextRole::Secondary)),
        )
    };

    teksu!(
        VStack {
            spacing: 12.0
            child: header
            child: body
        }
    )
}

fn enabled_body(vm: &DtoViewModel) -> impl Widget {
    let name = {
        let on_submit = vm.clone();
        let on_blur = vm.clone();
        TextInput::new(vm.name())
            .label(tr!(dto_name()))
            .placeholder(tr!(dto_name_placeholder()))
            .validation(vm.name_validation())
            .on_submit_fn(move |_c| on_submit.commit_name())
            .on_blur_fn(move |_c| on_blur.commit_name())
    };

    let form = FormLayout::new()
        .label_gap(12.0)
        .row_spacing(12.0)
        .label(vm.side().heading())
        .line(required_field_label(tr!(dto_name())), name);

    teksu!(
        VStack {
            spacing: 12.0
            child: form
            child: fields_pane(vm)
            Divider::horizontal
            child: field_form(vm)
        }
    )
}

/// The DTO's fields, with their own plus and their own reorder.
fn fields_pane(vm: &DtoViewModel) -> impl Widget {
    let source = {
        let reorder = vm.clone();
        ReorderableSource::new(
            vm.fields().list_model(),
            |row| row.id,
            move |id, index| reorder.reorder_field(id, index),
        )
    };

    let delegate_vm = vm.clone();
    let list = ListView::from_source_keyed(source, vm.selection(), move |_i, row, selected| {
        let id = row.id;
        let vm = delegate_vm.clone();
        Box::new(
            StandardListItem::new(lit!(row.name.clone()))
                .subtitle(lit!(dto_field_subtitle(row)))
                .selected(selected)
                .label_overflow(TextOverflow::Ellipsis(EllipsisMode::Trailing))
                .subtitle_overflow(TextOverflow::Ellipsis(EllipsisMode::Trailing))
                .trailing_slot(row_menu_button(move || {
                    let vm = vm.clone();
                    Box::new(
                        MenuList::new().item(
                            MenuItem::new(tr!(dto_field_delete()))
                                .on_activate_fn(move |_c| vm.remove_field(id)),
                        ),
                    )
                })),
        )
    })
    .reorderable(true)
    .activate_on(ActivateOn::SingleClick)
    .auto_item_height(ROW_HEIGHT);

    let empty = emptiness(&vm.fields().version_signal(), {
        let fields = vm.fields();
        move || fields.is_empty()
    });
    let body = list_or_empty(
        empty,
        list,
        empty_state(tr!(dto_fields_empty()), tr!(dto_fields_empty_hint())),
    );

    let add = vm.clone();
    teksu!(
        VStack {
            spacing: 0.0
            HStack {
                spacing: 8.0
                TextWidget::new(tr!(dto_fields_heading())) {
                    style: TextStyleRole::BodyBold
                }
                Expand::horizontal
                IconButton::new(crate::icons::action::add()) {
                    size: IconButtonSize::Toolbar
                    tooltip: tr!(dto_field_add())
                    on_activate_fn: move |_c| add.add_field()
                }
            }
            FixedSize {
                height: FIELD_LIST_HEIGHT
                child: body
            }
        }
    )
}

/// The selected field's form.
fn field_form(vm: &DtoViewModel) -> impl Widget {
    let Some(_) = vm.selected_field() else {
        return teksu!(
            VStack {
                spacing: 4.0
                child: empty_state(
                    tr!(dto_field_none_selected()),
                    tr!(dto_field_none_selected_hint()),
                )
            }
        );
    };

    let name = {
        let on_submit = vm.clone();
        let on_blur = vm.clone();
        TextInput::new(vm.field_name())
            .label(tr!(dto_field_name()))
            .placeholder(tr!(dto_field_name_placeholder()))
            .validation(vm.field_name_validation())
            .on_submit_fn(move |_c| on_submit.commit_field())
            .on_blur_fn(move |_c| on_blur.commit_field())
    };

    let type_combo = {
        let vm = vm.clone();
        let selected = Signal::new(Some(vm.field_type().get()));
        ComboBox::from_items(DTO_FIELD_TYPES, selected, |t| lit!(dto_field_type_name(t)))
            .label(tr!(dto_field_type()))
            .on_select(move |chosen, _c| vm.set_field_type(chosen.clone()))
    };

    let optional = {
        let vm = vm.clone();
        Checkbox::new(vm.field_optional())
            .label(tr!(dto_field_optional()))
            .labelled_externally()
            .on_change(move |value, _c| vm.set_field_optional(value))
    };

    let is_list = {
        let vm = vm.clone();
        Checkbox::new(vm.field_is_list())
            .label(tr!(dto_field_is_list()))
            .labelled_externally()
            .on_change(move |value, _c| vm.set_field_is_list(value))
    };

    let mut form = FormLayout::new()
        .label_gap(12.0)
        .row_spacing(12.0)
        .label(tr!(dto_fields_heading()))
        .line(required_field_label(tr!(dto_field_name())), name)
        .line(required_field_label(tr!(dto_field_type())), type_combo)
        .line(field_label(tr!(dto_field_optional())), optional)
        .line(field_label(tr!(dto_field_is_list())), is_list);

    if vm.shows_enum() {
        let on_submit = vm.clone();
        let on_blur = vm.clone();
        form = form
            .line(
                required_field_label(tr!(dto_field_enum_name())),
                TextInput::new(vm.field_enum_name())
                    .label(tr!(dto_field_enum_name()))
                    .placeholder(tr!(dto_field_enum_name_placeholder()))
                    .validation(vm.field_enum_name_validation())
                    .on_submit_fn(move |_c| commit_enum_name(&on_submit))
                    .on_blur_fn(move |_c| commit_enum_name(&on_blur)),
            )
            .line(field_label(tr!(dto_field_enum_values())), {
                let editor = PlainTextEditor::new(vm.enum_document()).min_lines(3);
                teksu!(
                    FixedSize {
                        height: ENUM_EDITOR_HEIGHT
                        child: editor
                    }
                )
            });
    }

    teksu!(
        VStack {
            spacing: 12.0
            child: form
        }
    )
}

/// US-DTO-02: unticking destroys the DTO and every field on it, so it asks first.
///
/// Presented imperatively from the handler, which is why this lives beside the view
/// rather than in it: a `MessageBox` is shown, not built into the tree.
fn confirm_disable(vm: &DtoViewModel, ctx: &mut EventContext) {
    let name = vm.name().get();
    let count = vm.field_count();
    let confirmed = vm.clone();
    let cancelled = vm.clone();
    MessageBox::question(tr!(dto_disable_title()))
        .text(tr!(dto_disable_message(name = name, count = count as i64)))
        // Yes and No, with **No** as the default and as the escape: teksilo makes
        // the safe answer the one Enter takes, which is what this question needs.
        .buttons(MessageBoxButtons::YesNo)
        .on_result(move |result, _c| match result.button {
            StandardButton::Yes => confirmed.disable(),
            // The checkbox already wrote itself when it was clicked, so anything
            // other than Yes has to put it back: the DTO is still there.
            _ => cancelled.resync_enabled(),
        })
        .present(ctx);
}

fn commit_enum_name(vm: &DtoViewModel) {
    vm.commit_field();
}
