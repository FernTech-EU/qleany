//! The Features screen: features, their use cases, and the two DTO panes.

use teksilo::core::BindingLevel;
use teksilo::prelude::*;
use teksilo::widgets::{
    ActivateOn, Divider, Expand, HStack, IconButton, IconButtonSize, ListView, MenuItem, MenuList,
    Padding, ScrollArea, StandardListItem, TabId, TabInfo, TabWidget, TextInput, TextWidget,
    VStack,
};

use crate::about::confirm::{Cascade, confirm_delete};
use crate::features::dto_pane::dto_pane;
use crate::features::use_case_form::use_case_form;
use crate::features::{DtoViewModel, FeaturesViewModel, UseCaseViewModel};
use crate::shared::form::heading;
use crate::shared::list_or_empty::{emptiness, empty_state, list_or_empty};
use crate::shared::pane::FixedWidth;
use crate::shared::reorder::ReorderableSource;

/// Width of the feature column.
const FEATURE_COLUMN_WIDTH: f32 = 220.0;

/// Width of the use case column. Wider than the features: a use case name carries
/// a subtitle of up to five flags beneath it.
const USE_CASE_COLUMN_WIDTH: f32 = 260.0;

const ROW_HEIGHT: f32 = 48.0;

pub struct FeaturesPage {
    features: FeaturesViewModel,
    use_cases: UseCaseViewModel,
    dto_in: DtoViewModel,
    dto_out: DtoViewModel,
    /// Which DTO tab is open, and the two ids it can name.
    ///
    /// All three are fields rather than values made in `build`: this page rebuilds
    /// whenever a list changes, and `TabId::fresh()` mints a new id every time it is
    /// called, so a tab set rebuilt with fresh ids leaves the remembered selection
    /// pointing at a tab that no longer exists. The symptom is not a tab that snaps
    /// back, it is a tab widget with *neither* pane mounted, which reads as the
    /// screen having lost its content.
    dto_tab: Signal<Option<TabId>>,
    dto_in_tab: TabId,
    dto_out_tab: TabId,
    root_child: Option<WidgetId>,
}

impl std::fmt::Debug for FeaturesPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeaturesPage").finish_non_exhaustive()
    }
}

impl FeaturesPage {
    pub fn new(
        features: FeaturesViewModel,
        use_cases: UseCaseViewModel,
        dto_in: DtoViewModel,
        dto_out: DtoViewModel,
    ) -> Self {
        Self {
            features,
            use_cases,
            dto_in,
            dto_out,
            dto_tab: Signal::new(None),
            dto_in_tab: TabId::fresh(),
            dto_out_tab: TabId::fresh(),
            root_child: None,
        }
    }
}

impl Widget for FeaturesPage {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        self.features.wire(ctx);
        self.use_cases.wire(ctx);
        self.dto_in.wire(ctx);
        self.dto_out.wire(ctx);

        // Every signal that decides which rows exist, bound at `Rebuild` on the
        // source rather than on a `.map(..)` of it: a derived signal built inline is
        // dropped at the end of the statement and never fires again.
        let registry = ctx.binding_registry();
        let me = ctx.self_id();
        for signal in [
            self.features.selection().selection_signal(),
            self.use_cases.selection().selection_signal(),
        ] {
            signal.bind_to(me, registry, BindingLevel::Rebuild);
        }
        self.use_cases
            .read_only()
            .bind_to(me, registry, BindingLevel::Rebuild);
        for pane in [&self.dto_in, &self.dto_out] {
            pane.enabled().bind_to(me, registry, BindingLevel::Rebuild);
            pane.selection()
                .selection_signal()
                .bind_to(me, registry, BindingLevel::Rebuild);
            pane.field_type()
                .bind_to(me, registry, BindingLevel::Rebuild);
        }

        let id = ctx.add(teksu!(
            VStack {
                spacing: 0.0
                Padding::new(16.0, 20.0, 8.0, 20.0) {
                    child: heading(tr!(features_title()))
                }
                Expand {
                    HStack {
                        spacing: 0.0
                        child: feature_column(&self.features)
                        Divider::vertical
                        child: use_case_column(&self.use_cases, &self.features)
                        Divider::vertical
                        Expand {
                            child: detail_column(
                                &self.use_cases,
                                &self.dto_in,
                                &self.dto_out,
                                self.dto_tab.clone(),
                                (self.dto_in_tab, self.dto_out_tab),
                            )
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

fn feature_column(vm: &FeaturesViewModel) -> impl Widget {
    let source = {
        let reorder = vm.clone();
        ReorderableSource::new(
            vm.list().list_model(),
            |row| row.id,
            move |id, index| reorder.reorder(id, index),
        )
    };

    let delegate_vm = vm.clone();
    let list = ListView::from_source_keyed(source, vm.selection(), move |_i, row, selected| {
        let id = row.id;
        let name = row.name.clone();
        let vm = delegate_vm.clone();
        Box::new(
            StandardListItem::new(lit!(row.name.clone()))
                .selected(selected)
                .label_overflow(TextOverflow::Ellipsis(EllipsisMode::Trailing))
                .trailing_slot(row_menu(tr!(features_delete()), {
                    let name = name.clone();
                    move |ctx| {
                        // US-SAFE-01: a feature takes its use cases and their DTOs.
                        let vm = vm.clone();
                        confirm_delete(ctx, Cascade::Feature, &name, move || vm.remove(id));
                    }
                })),
        )
    })
    .reorderable(true)
    .activate_on(ActivateOn::SingleClick)
    .auto_item_height(ROW_HEIGHT);

    let empty = emptiness(&vm.list().version_signal(), {
        let list = vm.list();
        move || list.is_empty()
    });
    let body = list_or_empty(
        empty,
        list,
        empty_state(tr!(features_empty()), tr!(features_empty_hint())),
    );

    let add = vm.clone();
    FixedWidth::new(
        FEATURE_COLUMN_WIDTH,
        teksu!(
            VStack {
                spacing: 0.0
                child: pane_header(tr!(features_list_heading()), tr!(features_add()), Signal::new(true), move |_c| add.add())
                Divider::horizontal
                Expand {
                    child: body
                }
            }
        ),
    )
}

fn use_case_column(vm: &UseCaseViewModel, features: &FeaturesViewModel) -> impl Widget {
    let source = {
        let reorder = vm.clone();
        ReorderableSource::new(
            vm.list().list_model(),
            |row| row.id,
            move |id, index| reorder.reorder(id, index),
        )
    };

    let delegate_vm = vm.clone();
    let list = ListView::from_source_keyed(source, vm.selection(), move |_i, row, selected| {
        let id = row.id;
        let vm = delegate_vm.clone();
        let name = row.name.clone();
        let mut item = StandardListItem::new(lit!(row.name.clone()))
            .selected(selected)
            .label_overflow(TextOverflow::Ellipsis(EllipsisMode::Trailing))
            .trailing_slot(row_menu(tr!(use_cases_delete()), {
                let name = name.clone();
                move |ctx| {
                    // US-SAFE-01: a use case takes its DTOs.
                    let vm = vm.clone();
                    confirm_delete(ctx, Cascade::UseCase, &name, move || vm.remove(id));
                }
            }));
        if let Some(subtitle) = delegate_vm.subtitle(row) {
            item = item
                .subtitle(subtitle)
                .subtitle_overflow(TextOverflow::Ellipsis(EllipsisMode::Trailing));
        }
        Box::new(item)
    })
    .reorderable(true)
    .activate_on(ActivateOn::SingleClick)
    .auto_item_height(ROW_HEIGHT);

    let empty = emptiness(&vm.list().version_signal(), {
        let list = vm.list();
        move || list.is_empty()
    });
    let body = list_or_empty(
        empty,
        list,
        empty_state(tr!(use_cases_empty()), tr!(use_cases_empty_hint())),
    );

    // Without a feature there is nothing to add a use case to, so the plus says so
    // rather than doing nothing.
    let enabled = features.has_selection();
    let add = vm.clone();
    FixedWidth::new(
        USE_CASE_COLUMN_WIDTH,
        teksu!(
            VStack {
                spacing: 0.0
                child: feature_form(features)
                Divider::horizontal
                child: pane_header(tr!(use_cases_list_heading()), tr!(use_cases_add()), enabled, move |_c| add.add())
                Divider::horizontal
                Expand {
                    child: body
                }
            }
        ),
    )
}

/// The selected feature's name, above its use cases.
///
/// In this column rather than in the detail one, which belongs to the use case: a
/// feature has exactly one editable property, and putting it over the list of what
/// it contains is where the Slint UI had it too.
fn feature_form(vm: &FeaturesViewModel) -> Box<dyn Widget> {
    if vm.selected_id().is_none() {
        return Box::new(teksu!(
            Padding::new(12.0, 12.0, 12.0, 12.0) {
                TextWidget::new(tr!(features_none_selected())) {
                    color: TextRole::Secondary
                    style: TextStyleRole::Small
                }
            }
        ));
    }

    let on_submit = vm.clone();
    let on_blur = vm.clone();
    let name = TextInput::new(vm.name())
        .label(tr!(features_name()))
        .placeholder(tr!(features_name_placeholder()))
        .validation(vm.name_validation())
        .on_submit_fn(move |_c| on_submit.commit())
        .on_blur_fn(move |_c| on_blur.commit());

    Box::new(teksu!(
        Padding::new(12.0, 12.0, 12.0, 12.0) {
            VStack {
                spacing: 4.0
                TextWidget::new(tr!(features_name())) {
                    style: TextStyleRole::Small
                    color: TextRole::Secondary
                }
                child: name
            }
        }
    ))
}

/// The use case's form over its two DTO panes, side by side.
/// The use case's form over its two DTO panes, side by side.
///
/// The `ScrollArea` is returned bare rather than wrapped in a `VStack`. A stack
/// proposes its intrinsic height to a child that does not expand, and a scroll
/// area's intrinsic height is a fixed fallback of 200 dp, so a wrapped one ends up
/// 200 dp tall with its content spilling out below it, unclipped and overlapping
/// whatever is underneath.
fn detail_column(
    use_cases: &UseCaseViewModel,
    dto_in: &DtoViewModel,
    dto_out: &DtoViewModel,
    dto_tab: Signal<Option<TabId>>,
    tab_ids: (TabId, TabId),
) -> Box<dyn Widget> {
    if use_cases.selected_id().is_none() {
        return Box::new(empty_state(
            tr!(use_cases_none_selected()),
            tr!(use_cases_none_selected_hint()),
        ));
    }

    let form = use_case_form(use_cases);
    // Tabs rather than two panes side by side, which is what the Slint UI did too.
    // Each pane carries a name field, a field list and a field form, and half of a
    // 600 dp column is not enough for one of those: the fields overflow their pane
    // and paint over the next. One at a time, full width, and the tab bar says which
    // of the two is on screen.
    let panes = TabWidget::new(dto_tab)
        .static_tab_with_id(
            tab_ids.0,
            TabInfo::new().title(dto_in.side().heading()),
            dto_pane(dto_in),
        )
        .static_tab_with_id(
            tab_ids.1,
            TabInfo::new().title(dto_out.side().heading()),
            dto_pane(dto_out),
        );

    Box::new(teksu!(
        ScrollArea {
            Padding::new(16.0, 16.0, 16.0, 16.0) {
                VStack {
                    spacing: 16.0
                    child: form
                    Divider::horizontal
                    child: panes
                }
            }
        }
    ))
}

/// A list pane's title bar: a name and a plus.
fn pane_header(
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

/// The overflow button every row carries, with one entry in it.
///
/// A visible affordance as well as the right-click menu: a menu that only a
/// right-click reveals is one most users never find. The menu is built on demand so
/// it closes over the row it belongs to rather than over the current selection.
fn row_menu(
    label: LocalizedString,
    on_activate: impl Fn(&mut EventContext) + Clone + 'static,
) -> impl Widget {
    IconButton::new(crate::icons::action::more())
        .size(IconButtonSize::Toolbar)
        .tooltip(tr!(common_more_actions()))
        .context_menu(move |_pos, _ctx| {
            let on_activate = on_activate.clone();
            Some(Box::new(
                MenuList::new().item(MenuItem::new(label.clone()).on_activate_fn(on_activate)),
            ) as Box<dyn Widget>)
        })
}
