//! The use case list, its three flags, and the entities it touches.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use teksilo::data::{KeyedSelectionModel, ListModel, SelectionMode};
use teksilo::prelude::*;
use teksilo::widgets::ValidationState;

use frontend::AppContext;
use frontend::EntityId;
use frontend::commands::use_case_commands;
use frontend::common::direct_access::use_case::UseCaseRelationshipField;
use frontend::direct_access::{CreateUseCaseDto, UseCaseRelationshipDto};

use crate::app_ids::AppIds;
use crate::models::{FeatureUseCasesListModel, FeatureUseCasesRow, WorkspaceEntitiesListModel};
use crate::shared::validation::{is_snake_case, required_cased};
use crate::singles::SingleUseCase;

/// The name a new use case is given.
const NEW_USE_CASE_NAME: &str = "new_use_case";

/// Which DTOs a use case has, per row.
///
/// `FeatureUseCasesRow` cannot carry them: the generated row is built from a use
/// case's scalar fields, and a DTO is a reference. The subtitle needs both, so one
/// batched read per refresh fills this.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DtoPresence {
    pub input: bool,
    pub output: bool,
}

#[derive(Clone)]
pub struct UseCaseViewModel {
    app_ctx: Rc<AppContext>,
    ids: AppIds,
    list: FeatureUseCasesListModel,
    single: SingleUseCase,
    selection: KeyedSelectionModel<EntityId>,
    /// The feature whose use cases are listed, written by the feature selection.
    owner: Signal<Option<EntityId>>,
    /// Which DTOs each row has. See [`DtoPresence`].
    dtos: Signal<HashMap<EntityId, DtoPresence>>,
    /// Every entity in the manifest, for the association list.
    entities: WorkspaceEntitiesListModel,
    /// The entities the selected use case is associated with.
    associated: Signal<HashSet<EntityId>>,
    /// The entities a use case may be associated with, as a list of its own.
    ///
    /// A projection of the entity list rather than the entity list itself: a
    /// heritage-only entity is never instantiated, so a use case cannot touch one,
    /// and a delegate that skipped those rows would leave blank gaps in the list
    /// where they used to be.
    associable: ListModel<crate::models::WorkspaceEntitiesRow>,
    /// One checkbox signal per entity, kept across rebuilds.
    ///
    /// A `ListView` delegate runs per realization, so a signal made there would be
    /// a different signal every time the row scrolled back into view, and the
    /// checkbox would forget itself. Keyed by entity id rather than by index,
    /// because a refresh renumbers the rows.
    ticks: Rc<RefCell<HashMap<EntityId, Signal<bool>>>>,
}

impl std::fmt::Debug for UseCaseViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UseCaseViewModel")
            .field("rows", &self.list.len())
            .field("selected", &self.selected_id())
            .finish_non_exhaustive()
    }
}

impl UseCaseViewModel {
    pub fn new(
        app_ctx: Rc<AppContext>,
        ids: AppIds,
        list: FeatureUseCasesListModel,
        single: SingleUseCase,
        owner: Signal<Option<EntityId>>,
        entities: WorkspaceEntitiesListModel,
    ) -> Self {
        Self {
            app_ctx,
            ids,
            list,
            single,
            selection: KeyedSelectionModel::new(SelectionMode::Single),
            owner,
            dtos: Signal::new(HashMap::new()),
            entities,
            associated: Signal::new(HashSet::new()),
            associable: ListModel::new(),
            ticks: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// The associable entities, as a model a `ListView` can read.
    pub fn associable_model(&self) -> ListModel<crate::models::WorkspaceEntitiesRow> {
        self.associable.clone()
    }

    /// The signal behind one entity's tick, made once and kept.
    pub fn tick(&self, entity: EntityId) -> Signal<bool> {
        let mut ticks = self.ticks.borrow_mut();
        ticks
            .entry(entity)
            .or_insert_with(|| Signal::new(self.associated.get().contains(&entity)))
            .clone()
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn list(&self) -> FeatureUseCasesListModel {
        self.list.clone()
    }

    pub fn selection(&self) -> KeyedSelectionModel<EntityId> {
        self.selection.clone()
    }

    pub fn selected(&self) -> Signal<Option<EntityId>> {
        self.selection
            .selection_signal()
            .map(|keys| keys.iter().copied().next())
    }

    pub fn selected_id(&self) -> Option<EntityId> {
        self.selection.selected_keys().first().copied()
    }

    pub fn has_selection(&self) -> Signal<bool> {
        self.selection.selection_signal().map(|k| !k.is_empty())
    }

    pub fn name(&self) -> Signal<String> {
        self.single.name()
    }

    pub fn name_validation(&self) -> Signal<ValidationState> {
        required_cased(
            &self.single.name(),
            tr!(use_cases_name_required()),
            tr!(use_cases_name_snake_case()),
            is_snake_case,
        )
    }

    pub fn read_only(&self) -> Signal<bool> {
        self.single.read_only()
    }

    pub fn long_operation(&self) -> Signal<bool> {
        self.single.long_operation()
    }

    pub fn undoable(&self) -> Signal<bool> {
        self.single.undoable()
    }

    /// Whether the Undoable flag is shown.
    ///
    /// A read-only use case writes nothing, so there is nothing for it to undo. The
    /// row is hidden rather than disabled: a disabled control invites the user to
    /// work out why.
    pub fn undoable_applies(&self) -> Signal<bool> {
        self.single.read_only().map(|ro| !*ro)
    }

    /// Which DTOs the selected use case has.
    pub fn dto_presence(&self, id: EntityId) -> DtoPresence {
        self.dtos.get().get(&id).copied().unwrap_or_default()
    }

    /// What a row says under its name.
    pub fn subtitle(&self, row: &FeatureUseCasesRow) -> Option<LocalizedString> {
        let parts = subtitle_parts(row, self.dto_presence(row.id));
        (!parts.is_empty()).then(|| lit!(parts.join(" · ")))
    }

    pub fn entities(&self) -> WorkspaceEntitiesListModel {
        self.entities.clone()
    }

    /// Whether the selected use case is associated with an entity.
    pub fn is_associated(&self, entity: EntityId) -> bool {
        self.associated.get().contains(&entity)
    }

    pub fn associated_signal(&self) -> Signal<HashSet<EntityId>> {
        self.associated.clone()
    }

    pub fn stack_signal(&self) -> Signal<Option<u64>> {
        self.ids.features_stack.clone()
    }

    // ── commands ─────────────────────────────────────────────────────────────

    /// Append a use case with every flag off and no DTOs, which is the only shape
    /// that is valid before the user has said anything about it.
    pub fn add(&self) {
        let dto = CreateUseCaseDto {
            name: NEW_USE_CASE_NAME.to_string(),
            ..Default::default()
        };
        if let Some(id) = self.list.create(&dto, -1, self.stack()) {
            self.selection.select(id);
        }
    }

    pub fn remove(&self, id: EntityId) {
        self.list.remove(id, self.stack());
        if self.selected_id() == Some(id) {
            self.selection.clear();
        }
    }

    pub fn select(&self, id: EntityId) {
        self.selection.select(id);
    }

    pub fn commit(&self) {
        self.single.save(self.stack());
        self.list.refresh();
    }

    /// Mark the use case read-only, which also takes Undoable away.
    ///
    /// Both in one write: they are scalars on the same row, so the single's `save`
    /// covers them and there is nothing to bracket.
    pub fn set_read_only(&self, value: bool) {
        self.single.set_read_only(value);
        if value {
            self.single.set_undoable(false);
        }
        self.commit();
    }

    pub fn set_undoable(&self, value: bool) {
        self.single.set_undoable(value);
        self.commit();
    }

    pub fn set_long_operation(&self, value: bool) {
        self.single.set_long_operation(value);
        self.commit();
    }

    /// Associate the selected use case with an entity, or stop.
    ///
    /// The whole set is rewritten, because that is the shape of the command: a
    /// relationship is set, not added to.
    pub fn set_associated(&self, entity: EntityId, associated: bool) {
        let Some(id) = self.selected_id() else {
            return;
        };
        let mut wanted = self.associated.get();
        if associated {
            wanted.insert(entity);
        } else {
            wanted.remove(&entity);
        }

        // Written in the manifest's own entity order rather than in the set's, so
        // the YAML does not churn between saves.
        let ordered: Vec<EntityId> = self
            .entities
            .rows()
            .iter()
            .map(|row| row.id)
            .filter(|row_id| wanted.contains(row_id))
            .collect();

        let dto = UseCaseRelationshipDto {
            id,
            field: UseCaseRelationshipField::Entities,
            right_ids: ordered,
        };
        if let Err(e) =
            use_case_commands::set_use_case_relationship(&self.app_ctx, self.stack(), &dto)
        {
            log::error!("could not associate entity {entity} with use case {id}: {e}");
            // The tick reverts: the list re-reads from the store, which still holds
            // what it held before.
            self.refresh_associations();
            return;
        }
        self.associated.set_if_changed(wanted.clone());
        self.sync_ticks(&wanted);
    }

    // ── wiring ───────────────────────────────────────────────────────────────

    pub fn wire(&self, ctx: &mut BuildContext) {
        let list = self.list.clone();
        let owner = self.owner.clone();
        ctx.effect(&owner, move |id| list.set_owner_id(*id));
        self.list.set_owner_id(owner.get());

        // A different feature means a different set of use cases, and the selected
        // one belonged to the feature being left.
        let selection = self.selection.clone();
        let owner_changed = self.owner.clone();
        ctx.effect(&owner_changed, move |_| selection.clear());

        let me = self.clone();
        let selection = self.selection.selection_signal();
        ctx.effect(&selection, move |_| {
            me.point_at_selection();
            me.refresh_associations();
        });
        self.point_at_selection();
        self.refresh_associations();

        let me = self.clone();
        let version = self.list.version_signal();
        ctx.effect(&version, move |_| {
            me.prune_selection();
            me.refresh_dtos();
        });
        self.refresh_dtos();

        // The association list follows the entities, which are edited on another
        // screen entirely.
        let me = self.clone();
        let entities_version = self.entities.version_signal();
        ctx.effect(&entities_version, move |_| me.refresh_associable());
        self.refresh_associable();
    }

    /// Re-project the entity list, keeping the rows that are still there.
    fn refresh_associable(&self) {
        self.associable
            .reconcile_by_key(associable(&self.entities), |row| row.id);
    }

    fn point_at_selection(&self) {
        let id = self.selected_id();
        if self.single.id() == id {
            return;
        }
        self.single.set_id(id);
    }

    fn prune_selection(&self) {
        let live: Vec<EntityId> = self.list.rows().iter().map(|r| r.id).collect();
        self.selection.prune_missing(|id| live.contains(id));
    }

    /// One batched read per DTO side, for the whole list.
    fn refresh_dtos(&self) {
        let ids: Vec<EntityId> = self.list.rows().iter().map(|r| r.id).collect();
        if ids.is_empty() {
            self.dtos.set_if_changed(HashMap::new());
            return;
        }
        let mut presence: HashMap<EntityId, DtoPresence> = HashMap::new();
        for (field, is_input) in [
            (UseCaseRelationshipField::DtoIn, true),
            (UseCaseRelationshipField::DtoOut, false),
        ] {
            match use_case_commands::get_use_case_relationship_many(&self.app_ctx, &ids, &field) {
                Ok(map) => {
                    for (id, dtos) in map {
                        let entry = presence.entry(id).or_default();
                        if is_input {
                            entry.input = !dtos.is_empty();
                        } else {
                            entry.output = !dtos.is_empty();
                        }
                    }
                }
                Err(e) => {
                    log::error!("could not read the use case DTOs: {e}");
                    return;
                }
            }
        }
        self.dtos.set_if_changed(presence);
    }

    /// Which entities the selected use case touches.
    fn refresh_associations(&self) {
        let Some(id) = self.selected_id() else {
            self.associated.set_if_changed(HashSet::new());
            self.sync_ticks(&HashSet::new());
            return;
        };
        match use_case_commands::get_use_case_relationship(
            &self.app_ctx,
            &id,
            &UseCaseRelationshipField::Entities,
        ) {
            Ok(ids) => {
                let live: HashSet<EntityId> = ids.into_iter().collect();
                self.associated.set_if_changed(live.clone());
                self.sync_ticks(&live);
            }
            Err(e) => log::error!("could not read the entities of use case {id}: {e}"),
        }
    }

    /// Bring every tick in line with the store.
    ///
    /// This is what makes a failed write revert visibly: the checkbox wrote itself
    /// on the click, the write failed, and the re-read puts it back.
    fn sync_ticks(&self, live: &HashSet<EntityId>) {
        for (entity, tick) in self.ticks.borrow().iter() {
            tick.set_if_changed(live.contains(entity));
        }
    }

    fn stack(&self) -> Option<u64> {
        self.ids.features_stack.get()
    }
}

/// The flags a use case row shows, in the order the Slint UI showed them.
///
/// A free function over plain values, so the rule is testable without a backend:
/// the list is short, the order is deliberate, and getting it wrong is the kind of
/// thing that only shows up in a screenshot.
pub fn subtitle_parts(row: &FeatureUseCasesRow, dtos: DtoPresence) -> Vec<&'static str> {
    let mut parts = Vec::new();
    if row.read_only {
        parts.push("RO");
    }
    if row.long_operation {
        parts.push("long");
    }
    if row.undoable {
        parts.push("undo");
    }
    if dtos.input {
        parts.push("input");
    }
    if dtos.output {
        parts.push("output");
    }
    parts
}

/// A guard against an entity the manifest no longer has.
///
/// Association is stored as a weak relationship, so deleting an entity leaves any
/// use case that named it pointing at nothing. Filtering here is what keeps a stale
/// id out of the next write.
pub fn live_associations(
    associated: &HashSet<EntityId>,
    live: impl IntoIterator<Item = EntityId>,
) -> HashSet<EntityId> {
    let live: HashSet<EntityId> = live.into_iter().collect();
    associated.intersection(&live).copied().collect()
}

/// Every entity a use case may be associated with.
///
/// Heritage-only entities are excluded: they are never instantiated, so a use case
/// cannot touch one.
pub fn associable(
    entities: &WorkspaceEntitiesListModel,
) -> Vec<crate::models::WorkspaceEntitiesRow> {
    entities
        .rows()
        .into_iter()
        .filter(|row| !row.only_for_heritage)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(read_only: bool, long: bool, undoable: bool) -> FeatureUseCasesRow {
        FeatureUseCasesRow {
            id: 1,
            name: "do_something".to_string(),
            read_only,
            long_operation: long,
            undoable,
            ..Default::default()
        }
    }

    /// US-FEAT-06: the order is RO, long, undo, input, output, and nothing else.
    #[test]
    fn the_subtitle_lists_the_flags_in_a_fixed_order() {
        let all = subtitle_parts(
            &row(true, true, true),
            DtoPresence {
                input: true,
                output: true,
            },
        );
        assert_eq!(all, vec!["RO", "long", "undo", "input", "output"]);
    }

    #[test]
    fn a_use_case_with_no_flags_has_no_subtitle() {
        assert!(subtitle_parts(&row(false, false, false), DtoPresence::default()).is_empty());
    }

    #[test]
    fn each_flag_appears_on_its_own() {
        assert_eq!(
            subtitle_parts(&row(true, false, false), DtoPresence::default()),
            vec!["RO"]
        );
        assert_eq!(
            subtitle_parts(&row(false, true, false), DtoPresence::default()),
            vec!["long"]
        );
        assert_eq!(
            subtitle_parts(&row(false, false, true), DtoPresence::default()),
            vec!["undo"]
        );
        assert_eq!(
            subtitle_parts(
                &row(false, false, false),
                DtoPresence {
                    input: true,
                    output: false
                }
            ),
            vec!["input"]
        );
        assert_eq!(
            subtitle_parts(
                &row(false, false, false),
                DtoPresence {
                    input: false,
                    output: true
                }
            ),
            vec!["output"]
        );
    }

    /// An entity that has been deleted must not travel into the next write: the
    /// relationship is weak, so nothing else prunes it.
    #[test]
    fn an_association_with_a_deleted_entity_is_dropped() {
        let associated: HashSet<EntityId> = [1, 2, 3].into_iter().collect();
        let still_here = live_associations(&associated, [1, 3, 9]);
        assert_eq!(still_here, [1, 3].into_iter().collect::<HashSet<_>>());
    }
}
