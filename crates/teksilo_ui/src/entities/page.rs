//! The Entities screen: entity list, entity form with its field list, field form.

use teksilo::core::BindingLevel;
use teksilo::prelude::*;
use teksilo::widgets::{
    ActivateOn, Divider, Expand, HStack, IconButton, IconButtonSize, ListView, MenuItem, MenuList,
    Padding, ScrollArea, StandardListItem, TextWidget, VStack,
};

use crate::about::confirm::{Cascade, confirm_delete};
use crate::entities::entity_form::entity_form;
use crate::entities::field_form::field_form;
use crate::entities::field_vm::field_subtitle;
use crate::entities::{EntitiesViewModel, FieldViewModel};
use crate::intents::name;
use crate::shared::form::heading;
use crate::shared::list_or_empty::{emptiness, empty_state, list_or_empty};
use crate::shared::pane::FixedWidth;
use crate::shared::reorder::ReorderableSource;
use crate::shared::row_menu::row_menu_button;

/// Width of the entity column. Wide enough for a long PascalCase name beside its
/// overflow button without eliding.
const ENTITY_COLUMN_WIDTH: f32 = 260.0;

/// Width of the field form column. Wider than the lists: it carries a combo, a
/// multi-line editor and up to nine rows of labelled controls.
const FIELD_COLUMN_WIDTH: f32 = 380.0;

/// Height of a two-line row: a name over its subtitle.
const ROW_HEIGHT: f32 = 48.0;

pub struct EntitiesPage {
    entities: EntitiesViewModel,
    fields: FieldViewModel,
    root_child: Option<WidgetId>,
}

impl std::fmt::Debug for EntitiesPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntitiesPage").finish_non_exhaustive()
    }
}

impl EntitiesPage {
    pub fn new(entities: EntitiesViewModel, fields: FieldViewModel) -> Self {
        Self {
            entities,
            fields,
            root_child: None,
        }
    }
}

impl Widget for EntitiesPage {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        self.entities.wire(ctx);
        self.fields.wire(ctx);

        // Both forms are built from values rather than bound signal by signal: which
        // rows exist depends on the selected entity's heritage flag and on the
        // selected field's type, and a row that appears or disappears is a different
        // tree, not a different value. All four are bound at `Rebuild` on the source
        // signals, never on a `.map(..)` of one, which would be dropped at the end of
        // the statement and never fire again.
        let registry = ctx.binding_registry();
        let me = ctx.self_id();
        self.entities
            .selection()
            .selection_signal()
            .bind_to(me, registry, BindingLevel::Rebuild);
        self.entities
            .only_for_heritage()
            .bind_to(me, registry, BindingLevel::Rebuild);
        self.fields
            .selection()
            .selection_signal()
            .bind_to(me, registry, BindingLevel::Rebuild);
        self.fields
            .field_type()
            .bind_to(me, registry, BindingLevel::Rebuild);
        self.fields
            .relationship()
            .bind_to(me, registry, BindingLevel::Rebuild);
        self.fields
            .list_model()
            .bind_to(me, registry, BindingLevel::Rebuild);
        // The entity list feeds two combos, so a rename or a new entity has to reach
        // the forms as well as the list.
        self.entities
            .list()
            .version_signal()
            .bind_to(me, registry, BindingLevel::Rebuild);

        let id = ctx.add(teksu!(
            VStack {
                spacing: 0.0
                child: header(&self.entities)
                Expand {
                    HStack {
                        spacing: 0.0
                        child: entity_column(&self.entities)
                        Divider::vertical
                        Expand {
                            child: middle_column(&self.entities, &self.fields)
                        }
                        child_opt: field_column(&self.entities, &self.fields)
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

/// The screen's title, and the one action that is about the model as a whole.
fn header(vm: &EntitiesViewModel) -> impl Widget {
    let has_entities = emptiness(&vm.list().version_signal(), {
        let list = vm.list();
        move || list.is_empty()
    })
    .map(|empty| !*empty);

    teksu!(
        Padding::new(16.0, 20.0, 8.0, 20.0) {
            HStack {
                spacing: 8.0
                child: heading(tr!(entities_title()))
                Expand::horizontal
                IconButton::new(crate::icons::action::diagram()) {
                    size: IconButtonSize::Toolbar
                    tooltip: tr!(entities_export_mermaid())
                    enabled: has_entities
                    on_activate_fn: |c| c.send_intent(Intent::new(name::EXPORT_MERMAID))
                }
            }
        }
    )
}

/// Column one: every entity in the manifest, in manifest order.
fn entity_column(vm: &EntitiesViewModel) -> impl Widget {
    let source = {
        let reorder = vm.clone();
        ReorderableSource::new(
            vm.list().list_model(),
            |row| row.id,
            move |id, index| reorder.reorder(id, index),
        )
    };

    let delegate_vm = vm.clone();
    let list = ListView::from_source_keyed(source, vm.selection(), move |_index, row, selected| {
        let id = row.id;
        let vm = delegate_vm.clone();
        let mut item = StandardListItem::new(lit!(row.name.clone()))
            .selected(selected)
            .label_overflow(TextOverflow::Ellipsis(EllipsisMode::Trailing))
            .trailing_slot(row_menu_button(move || {
                let vm = vm.clone();
                let name = name_of(&vm, id);
                Box::new(MenuList::new().item(
                    MenuItem::new(tr!(entities_delete())).on_activate_fn(move |ctx| {
                        // US-SAFE-01: an entity takes its fields and relationships
                        // with it, and the row does not say how many.
                        let vm = vm.clone();
                        confirm_delete(ctx, Cascade::Entity, &name, move || vm.remove(id));
                    }),
                )) as Box<dyn Widget>
            }));
        if let Some(subtitle) = delegate_vm.subtitle(row) {
            item = item
                .subtitle(subtitle)
                .subtitle_overflow(TextOverflow::Ellipsis(EllipsisMode::Trailing));
        }
        Box::new(item)
    })
    .reorderable(true)
    // Single click, not the double-click default: this is a master list, and its
    // rows are selected far more often than they are opened.
    .activate_on(ActivateOn::SingleClick)
    .auto_item_height(ROW_HEIGHT);

    let empty = emptiness(&vm.list().version_signal(), {
        let list = vm.list();
        move || list.is_empty()
    });
    let body = list_or_empty(
        empty,
        list,
        empty_state(tr!(entities_empty()), tr!(entities_empty_hint())),
    );

    let add = vm.clone();
    FixedWidth::new(
        ENTITY_COLUMN_WIDTH,
        teksu!(
            VStack {
                spacing: 0.0
                child: pane_header(tr!(entities_list_heading()), tr!(entities_add()), move |_c| add.add())
                Divider::horizontal
                Expand {
                    child: body
                }
            }
        ),
    )
}

/// Column two: the selected entity's form, over its field list.
fn middle_column(entities: &EntitiesViewModel, fields: &FieldViewModel) -> impl Widget {
    let form: Box<dyn Widget> = if entities.selected_id().is_some() {
        Box::new(teksu!(
            ScrollArea {
                Padding::new(16.0, 16.0, 16.0, 16.0) {
                    child: entity_form(entities)
                }
            }
        ))
    } else {
        Box::new(empty_state(
            tr!(entities_none_selected()),
            tr!(entities_none_selected_hint()),
        ))
    };

    teksu!(
        VStack {
            spacing: 0.0
            child: form
            Divider::horizontal
            Expand {
                child: field_pane(entities, fields)
            }
        }
    )
}

/// The field list, beside nothing: its form is the third column.
fn field_pane(entities: &EntitiesViewModel, fields: &FieldViewModel) -> impl Widget {
    let source = {
        let reorder = fields.clone();
        ReorderableSource::new(
            fields.list().list_model(),
            |row| row.id,
            move |id, index| reorder.reorder(id, index),
        )
    };

    let delegate_vm = fields.clone();
    let list = ListView::from_source_keyed(source, fields.selection(), move |_i, row, selected| {
        let id = row.id;
        let vm = delegate_vm.clone();
        Box::new(
            StandardListItem::new(lit!(row.name.clone()))
                .subtitle(lit!(field_subtitle(row)))
                .selected(selected)
                .label_overflow(TextOverflow::Ellipsis(EllipsisMode::Trailing))
                .subtitle_overflow(TextOverflow::Ellipsis(EllipsisMode::Trailing))
                .trailing_slot(row_menu_button(move || {
                    let vm = vm.clone();
                    Box::new(MenuList::new().item(
                        MenuItem::new(tr!(fields_delete())).on_activate_fn(move |_c| vm.remove(id)),
                    )) as Box<dyn Widget>
                })),
        )
    })
    .reorderable(true)
    .activate_on(ActivateOn::SingleClick)
    .auto_item_height(ROW_HEIGHT);

    let empty = emptiness(&fields.list().version_signal(), {
        let list = fields.list();
        move || list.is_empty()
    });
    let body = list_or_empty(
        empty,
        list,
        empty_state(tr!(fields_empty()), tr!(fields_empty_hint())),
    );

    // Without an entity there is nothing to add a field to, so the pane says so
    // rather than offering a plus that would do nothing.
    let enabled = entities.has_selection();
    let add = fields.clone();
    teksu!(
        VStack {
            spacing: 0.0
            child: pane_header_enabled(tr!(fields_list_heading()), tr!(fields_add()), enabled, move |_c| add.add())
            Divider::horizontal
            Expand {
                child: body
            }
        }
    )
}

/// Column three: the selected field's form, or nothing at all.
///
/// Absent rather than empty when no field is selected: the middle column then takes
/// the width, which is what makes a narrow window usable.
fn field_column(entities: &EntitiesViewModel, fields: &FieldViewModel) -> Option<impl Widget> {
    fields.selected_id()?;
    let form = field_form(fields, entities);
    let column = FixedWidth::new(
        FIELD_COLUMN_WIDTH,
        teksu!(
            ScrollArea {
                Padding::new(16.0, 16.0, 16.0, 16.0) {
                    child: form
                }
            }
        ),
    );
    Some(teksu!(
        HStack {
            spacing: 0.0
            Divider::vertical
            child: column
        }
    ))
}

/// An entity's own name, for a confirmation that has to say it.
fn name_of(vm: &EntitiesViewModel, id: frontend::EntityId) -> String {
    vm.list()
        .rows()
        .into_iter()
        .find(|row| row.id == id)
        .map(|row| row.name)
        .unwrap_or_default()
}

/// A list pane's title bar: a name and a plus.
fn pane_header(
    title: LocalizedString,
    add_label: LocalizedString,
    on_add: impl Fn(&mut EventContext) + 'static,
) -> impl Widget {
    pane_header_enabled(title, add_label, Signal::new(true), on_add)
}

fn pane_header_enabled(
    title: LocalizedString,
    add_label: LocalizedString,
    enabled: Signal<bool>,
    on_add: impl Fn(&mut EventContext) + 'static,
) -> impl Widget {
    teksu!(
        Padding::new(8.0, 12.0, 8.0, 12.0) {
            HStack {
                spacing: 8.0
                TextWidget::new(title) {
                    style: TextStyleRole::BodyBold
                }
                Expand::horizontal
                IconButton::new(crate::icons::action::add()) {
                    size: IconButtonSize::Toolbar
                    tooltip: add_label
                    enabled: enabled
                    on_activate_fn: on_add
                }
            }
        }
    )
}
