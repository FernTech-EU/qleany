//! The Generate screen: groups, files, filters and the preview.

use teksilo::core::BindingLevel;
use teksilo::data::{KeyedSelectionModel, ListModel, SelectionMode};
use teksilo::prelude::*;
use teksilo::widgets::{
    ActivateOn, Button, ButtonVariant, Divider, Expand, FixedSize, HStack, ListView, Padding,
    ProgressBar, RectWidget, SearchField, StandardListItem, TextWidget, VStack,
};

use crate::generate::GenerateViewModel;
use crate::generate::file_row::{FileRow, status_role};
use crate::generate::generate_vm::{FilterKind, Stage};
use crate::generate::preview::preview;
use crate::shared::form::{heading, inline_checkbox};
use crate::shared::keyed::KeyedSource;
use crate::shared::list_or_empty::{empty_state, list_or_empty};
use crate::shared::pane::FixedWidth;

/// Width of the group column.
const GROUP_COLUMN_WIDTH: f32 = 180.0;

/// Width of the file column. Wide enough for an elided path plus its tick.
const FILE_COLUMN_WIDTH: f32 = 420.0;

const ROW_HEIGHT: f32 = 40.0;

/// Width of the status stripe down a row's leading edge.
const STRIPE_WIDTH: f32 = 3.0;

/// How tall the stripe is: most of the row, with a little room top and bottom so
/// two adjacent stripes read as two rows rather than one bar.
const STRIPE_HEIGHT: f32 = ROW_HEIGHT - 12.0;

pub struct GeneratePage {
    vm: GenerateViewModel,
    /// The group list, rebuilt from the view-model's group names.
    groups: ListModel<String>,
    file_selection: KeyedSelectionModel<frontend::EntityId>,
    /// The text filter's own signal, observed rather than handed a callback:
    /// `SearchField` writes the signal it is given and offers no change hook.
    query: Signal<String>,
    root_child: Option<WidgetId>,
}

impl std::fmt::Debug for GeneratePage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeneratePage").finish_non_exhaustive()
    }
}

impl GeneratePage {
    pub fn new(vm: GenerateViewModel) -> Self {
        Self {
            vm,
            groups: ListModel::new(),
            file_selection: KeyedSelectionModel::new(SelectionMode::Single),
            query: Signal::new(String::new()),
            root_child: None,
        }
    }
}

impl Widget for GeneratePage {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        self.vm.wire(ctx);

        let registry = ctx.binding_registry();
        let me = ctx.self_id();
        // The version covers the rows, the ticks and the counts; the rest are the
        // things that change which widgets exist rather than what they show. Bound
        // on the source signals, never on a `.map(..)` temporary.
        self.vm
            .version()
            .bind_to(me, registry, BindingLevel::Rebuild);
        self.vm.stage().bind_to(me, registry, BindingLevel::Rebuild);
        self.vm
            .groups()
            .bind_to(me, registry, BindingLevel::Rebuild);
        self.vm
            .selected_file()
            .bind_to(me, registry, BindingLevel::Rebuild);
        self.vm
            .view_diff()
            .bind_to(me, registry, BindingLevel::Rebuild);

        self.groups
            .reconcile_by_key(self.vm.groups().get(), |name| name.clone());

        // The selection model is observed rather than handed a callback: a data view
        // has no `on_selection_changed`.
        {
            let vm = self.vm.clone();
            let selection = self.file_selection.selection_signal();
            ctx.effect(&selection, move |keys| {
                if let Some(file) = keys.iter().copied().next() {
                    vm.select_file(file);
                }
            });
        }

        // The text filter, likewise: `SearchField` writes its signal.
        {
            let vm = self.vm.clone();
            let query = self.query.clone();
            ctx.effect(&query, move |text| vm.set_text_filter(text));
        }

        let body = teksu!(
            HStack {
                spacing: 0.0
                child: group_column(&self.vm, self.groups.clone())
                Divider::vertical
                child: file_column(
                    &self.vm,
                    self.file_selection.clone(),
                    self.query.clone(),
                )
                Divider::vertical
                Expand {
                    child: preview(&self.vm, ctx)
                }
            }
        );

        let id = ctx.add(teksu!(
            VStack {
                spacing: 0.0
                child: header(&self.vm)
                Divider::horizontal
                child_opt: progress_strip(&self.vm)
                Expand {
                    child: body
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

/// The title, the two write settings, and the button that does the work.
fn header(vm: &GenerateViewModel) -> impl Widget {
    let count = vm.selected_count();
    let busy = vm.busy().get();

    let in_temp = {
        let vm = vm.clone();
        inline_checkbox(tr!(generate_in_temp()), vm.in_temp(), move |on, _c| {
            vm.set_in_temp(on)
        })
    };
    let view_diff = {
        let vm = vm.clone();
        inline_checkbox(tr!(generate_view_diff()), vm.view_diff(), move |on, _c| {
            vm.set_view_diff(on)
        })
    };

    let run = vm.clone();
    let refresh = vm.clone();
    teksu!(
        Padding::new(12.0, 20.0, 8.0, 20.0) {
            HStack {
                spacing: 12.0
                child: heading(tr!(generate_title()))
                Expand::horizontal
                child: view_diff
                child: in_temp
                Button::new(tr!(generate_refresh())) {
                    enabled: !busy
                    on_activate_fn: move |_c| refresh.refresh()
                }
                Button::new(tr!(generate_run(count = count as i64))) {
                    variant: ButtonVariant::Filled
                    // US-GEN-06: nothing ticked is nothing to write, and a button
                    // that looks pressable and writes nothing is worse than one
                    // that says so.
                    enabled: count > 0 && !busy
                    on_activate_fn: move |_c| run.generate()
                }
            }
        }
    )
}

/// What is running, if anything, and the way to stop it.
///
/// `None` when there is nothing to say, rather than an empty widget. A `Spacer` in
/// a vertical stack is not a blank line, it is a claim on every pixel the stack has
/// to spare, which pushed the whole screen to the bottom of the window.
fn progress_strip(vm: &GenerateViewModel) -> Option<Box<dyn Widget>> {
    let stage = vm.stage().get();
    if stage == Stage::Idle {
        // The last run's outcome, once there is one.
        return vm.status().get().map(|status| {
            Box::new(teksu!(
                Padding::new(6.0, 20.0, 6.0, 20.0) {
                    TextWidget::new(status) {
                        style: TextStyleRole::Small
                        color: TextRole::Secondary
                    }
                }
            )) as Box<dyn Widget>
        });
    }

    let title = match stage {
        Stage::Generating => tr!(generate_generating_title()),
        _ => tr!(generate_computing_title()),
    };
    let cancel = vm.clone();
    Some(Box::new(teksu!(
        Padding::new(8.0, 20.0, 8.0, 20.0) {
            VStack {
                spacing: 6.0
                HStack {
                    spacing: 12.0
                    TextWidget::new(title) {
                        style: TextStyleRole::BodyBold
                    }
                    TextWidget::new(lit!(String::new())) {
                        text: vm.message()
                        style: TextStyleRole::Small
                        color: TextRole::Secondary
                    }
                    Expand::horizontal
                    Button::new(tr!(generate_cancel())) {
                        on_activate_fn: move |_c| cancel.cancel()
                    }
                }
                ProgressBar::new(0.0) {
                    value: vm.progress()
                }
            }
        }
    )))
}

/// Column one: "All", then every group the manifest writes into.
///
/// Selected by name rather than through a selection model: a group list is short
/// and its rows are its own names, so the current group is a string comparison
/// rather than a second piece of state to keep in step.
fn group_column(vm: &GenerateViewModel, groups: ListModel<String>) -> impl Widget {
    let current = vm.selected_group().get();
    let on_activate_vm = vm.clone();
    let activate_groups = groups.clone();
    let list = ListView::from_source(groups, move |_index, name, _selected| {
        Box::new(
            StandardListItem::new(lit!(name.clone()))
                .selected(*name == current)
                .label_overflow(TextOverflow::Ellipsis(EllipsisMode::Trailing)),
        )
    })
    .activate_on(ActivateOn::SingleClick)
    .on_activate(move |index, _c| {
        if let Some(name) = activate_groups.with_item(index, |n: &String| n.clone()) {
            on_activate_vm.set_group(&name);
        }
    })
    .auto_item_height(ROW_HEIGHT);

    FixedWidth::new(
        GROUP_COLUMN_WIDTH,
        teksu!(
            VStack {
                spacing: 0.0
                Padding::new(8.0, 12.0, 8.0, 12.0) {
                    TextWidget::new(tr!(generate_groups_heading())) {
                        style: TextStyleRole::BodyBold
                    }
                }
                Divider::horizontal
                Expand {
                    child: list
                }
            }
        ),
    )
}

/// Column two: the filters, the wholesale selection, and the files.
fn file_column(
    vm: &GenerateViewModel,
    selection: KeyedSelectionModel<frontend::EntityId>,
    query: Signal<String>,
) -> impl Widget {
    let delegate_vm = vm.clone();
    // Keyed on the file's own id: a status pass renumbers nothing, but a filter
    // change does, and an index-keyed selection would jump to a different file.
    let source = KeyedSource::new(vm.rows(), |row: &FileRow| row.id);
    let list = ListView::from_source_keyed(source, selection, move |_i, row, selected| {
        Box::new(file_item(&delegate_vm, row, selected))
    })
    .activate_on(ActivateOn::SingleClick)
    .auto_item_height(ROW_HEIGHT);

    let empty = vm.version().map({
        let vm = vm.clone();
        move |_| vm.rows().is_empty()
    });
    let body = list_or_empty(
        empty,
        list,
        empty_state(tr!(generate_empty()), tr!(generate_empty_hint())),
    );

    FixedWidth::new(
        FILE_COLUMN_WIDTH,
        teksu!(
            VStack {
                spacing: 0.0
                child: filter_bar(vm, query)
                Divider::horizontal
                Expand {
                    child: body
                }
            }
        ),
    )
}

/// One file: a status stripe, a tick, and its path.
fn file_item(vm: &GenerateViewModel, row: &FileRow, selected: bool) -> impl Widget {
    let (prefix, name) = row.display_parts();
    let id = row.id;

    // US-GEN-03: the directory is context and the name is what a user scans for, so
    // the name carries the weight. A custom label slot rather than a formatted
    // string, which would have to pick one style for both.
    let label = teksu!(
        HStack {
            spacing: 0.0
            Expand::horizontal
            TextWidget::new(lit!(prefix)) {
                style: TextStyleRole::Small
                color: TextRole::Secondary
            }
            TextWidget::new(lit!(name)) {
                style: TextStyleRole::SmallBold
            }
        }
    );

    // Both dimensions. A `RectWidget` has no natural size of its own, and
    // `FixedSize` falls back to the child's natural size on any axis it is not
    // given, so a stripe with only a width is three pixels wide and none tall:
    // present in the tree, invisible on screen.
    let stripe = teksu!(
        FixedSize {
            width: STRIPE_WIDTH
            height: STRIPE_HEIGHT
            RectWidget::new() {
                background: status_role(&row.status)
            }
        }
    );

    let toggled = vm.clone();
    StandardListItem::new(lit!(row.full_path()))
        .label_slot(label)
        .leading_slot(stripe)
        .checkbox(vm.tick(id))
        .on_checkbox_toggle(move |_checked, _c| toggled.tick_changed())
        .selected(selected)
}

/// Three checkboxes on a line.
fn filter_row(
    vm: &GenerateViewModel,
    toggles: [(FilterKind, LocalizedString, bool); 3],
) -> impl Widget {
    let mut row = HStack::new().spacing(16.0);
    for (kind, label, on) in toggles {
        let vm = vm.clone();
        row = row.child(inline_checkbox(label, Signal::new(on), move |value, _c| {
            vm.set_filter(kind, value)
        }));
    }
    row.child(Expand::horizontal())
}

/// The text filter, the six checkboxes and the wholesale selection.
fn filter_bar(vm: &GenerateViewModel, query: Signal<String>) -> impl Widget {
    let filters = vm.filters().get();

    let text = SearchField::new(query).placeholder(tr!(generate_filter_placeholder()));

    // Two rows of three rather than a `Wrap`: a wrap hands each child an equal
    // share of the line, which for six labelled checkboxes in a 420 dp column is
    // enough for the box and an ellipsis. The split is meaningful anyway, status
    // over nature, and it is the one the Slint UI drew.
    let status_row = filter_row(
        vm,
        [
            (
                FilterKind::Modified,
                tr!(generate_status_modified()),
                filters.modified,
            ),
            (FilterKind::New, tr!(generate_status_new()), filters.new),
            (
                FilterKind::Unchanged,
                tr!(generate_status_unchanged()),
                filters.unchanged,
            ),
        ],
    );
    let nature_row = filter_row(
        vm,
        [
            (
                FilterKind::Infrastructure,
                tr!(generate_nature_infrastructure()),
                filters.infrastructure,
            ),
            (
                FilterKind::Aggregate,
                tr!(generate_nature_aggregate()),
                filters.aggregate,
            ),
            (
                FilterKind::Scaffold,
                tr!(generate_nature_scaffold()),
                filters.scaffold,
            ),
        ],
    );

    let select_all = vm.clone();
    let unselect_all = vm.clone();
    teksu!(
        Padding::new(8.0, 12.0, 8.0, 12.0) {
            VStack {
                spacing: 8.0
                child: text
                child: status_row
                child: nature_row
                HStack {
                    spacing: 8.0
                    Button::new(tr!(generate_select_all())) {
                        on_activate_fn: move |_c| select_all.set_all_visible(true)
                    }
                    Button::new(tr!(generate_unselect_all())) {
                        on_activate_fn: move |_c| unselect_all.set_all_visible(false)
                    }
                    Expand::horizontal
                }
            }
        }
    )
}
