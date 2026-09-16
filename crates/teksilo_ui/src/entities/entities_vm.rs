//! The entity list and the entity form.

use std::collections::HashMap;
use std::rc::Rc;

use teksilo::data::{KeyedSelectionModel, SelectionMode};
use teksilo::prelude::*;
use teksilo::widgets::{Toast, ValidationState};

use frontend::AppContext;
use frontend::EntityId;
use frontend::commands::{entity_commands, handling_manifest_commands, undo_redo_commands};
use frontend::common::direct_access::entity::EntityRelationshipField;
use frontend::direct_access::{CreateEntityDto, EntityRelationshipDto};

use crate::app_ids::AppIds;
use crate::models::{WorkspaceEntitiesListModel, WorkspaceEntitiesRow};
use crate::shared::validation::{is_pascal_case, required_cased};
use crate::singles::SingleEntity;

/// The name a new entity is given.
///
/// Valid PascalCase on purpose: an entity that arrives already failing its own name
/// validation puts an error on a form the user has not touched yet.
const NEW_ENTITY_NAME: &str = "NewEntity";

#[derive(Clone)]
pub struct EntitiesViewModel {
    app_ctx: Rc<AppContext>,
    ids: AppIds,
    list: WorkspaceEntitiesListModel,
    single: SingleEntity,
    /// Single selection: the form below shows one entity.
    selection: KeyedSelectionModel<EntityId>,
    /// The parent's **name** per row that has one.
    ///
    /// `WorkspaceEntitiesRow` cannot carry it: the generated row is built from the
    /// entity's scalar fields, and `inherits_from` is an entity reference, so it is
    /// filtered out before the row type is written. One batched read per refresh
    /// fills this instead, which is one query for the whole list rather than one per
    /// row.
    parents: Signal<HashMap<EntityId, String>>,
}

impl std::fmt::Debug for EntitiesViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntitiesViewModel")
            .field("rows", &self.list.len())
            .field("selected", &self.selected_id())
            .finish_non_exhaustive()
    }
}

impl EntitiesViewModel {
    pub fn new(
        app_ctx: Rc<AppContext>,
        ids: AppIds,
        list: WorkspaceEntitiesListModel,
        single: SingleEntity,
    ) -> Self {
        Self {
            app_ctx,
            ids,
            list,
            single,
            selection: KeyedSelectionModel::new(SelectionMode::Single),
            parents: Signal::new(HashMap::new()),
        }
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn list(&self) -> WorkspaceEntitiesListModel {
        self.list.clone()
    }

    pub fn selection(&self) -> KeyedSelectionModel<EntityId> {
        self.selection.clone()
    }

    /// The selected entity, or `None`. Derived from the selection model rather than
    /// stored beside it, so the two cannot disagree.
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

    pub fn only_for_heritage(&self) -> Signal<bool> {
        self.single.only_for_heritage()
    }

    pub fn single_model(&self) -> Signal<bool> {
        self.single.single_model()
    }

    pub fn undoable(&self) -> Signal<bool> {
        self.single.undoable()
    }

    /// Whether the fields that only make sense on a concrete entity are shown.
    ///
    /// A heritage-only entity is never instantiated, so it has no row to undo, no
    /// single to bind and nothing to inherit from.
    pub fn concrete(&self) -> Signal<bool> {
        self.single.only_for_heritage().map(|only| !*only)
    }

    pub fn name_validation(&self) -> Signal<ValidationState> {
        required_cased(
            &self.single.name(),
            tr!(entities_name_required()),
            tr!(entities_name_pascal_case()),
            is_pascal_case,
        )
    }

    /// The entities a concrete entity may inherit from: the heritage-only ones,
    /// minus itself.
    ///
    /// Read from the live rows rather than kept as a second list, so an entity that
    /// has just been marked heritage-only appears here on the same refresh that
    /// changed it.
    pub fn heritage_options(&self) -> Vec<WorkspaceEntitiesRow> {
        let selected = self.selected_id();
        self.list
            .rows()
            .into_iter()
            .filter(|row| row.only_for_heritage && Some(row.id) != selected)
            .collect()
    }

    /// The parent of the selected entity, if it has one.
    pub fn inherits_from(&self) -> Option<EntityId> {
        self.single.dto().and_then(|dto| dto.inherits_from)
    }

    /// What a row says under its name: "abstract", "extends Parent", both, or
    /// nothing at all.
    pub fn subtitle(&self, row: &WorkspaceEntitiesRow) -> Option<LocalizedString> {
        let abstract_part = row
            .only_for_heritage
            .then(|| tr!(entities_subtitle_abstract()).resolve_now());
        let extends_part =
            self.parents.get().get(&row.id).map(|parent| {
                tr!(entities_subtitle_extends(parent = parent.clone())).resolve_now()
            });

        match (abstract_part, extends_part) {
            (Some(left), Some(right)) => {
                Some(tr!(entities_subtitle_both(left = left, right = right)))
            }
            (Some(only), None) | (None, Some(only)) => Some(lit!(only)),
            (None, None) => None,
        }
    }

    // ── commands ─────────────────────────────────────────────────────────────

    /// Append an entity and select it.
    pub fn add(&self) {
        let dto = CreateEntityDto {
            name: NEW_ENTITY_NAME.to_string(),
            undoable: true,
            ..Default::default()
        };
        if let Some(id) = self.list.create(&dto, -1, self.stack()) {
            self.select(id);
        }
    }

    /// Delete an entity, with its fields and relationships: the relationship is
    /// strong, so the backend cascades.
    pub fn remove(&self, id: EntityId) {
        self.list.remove(id, self.stack());
        if self.selected_id() == Some(id) {
            self.selection.clear();
        }
    }

    pub fn select(&self, id: EntityId) {
        self.selection.select(id);
    }

    /// Put an `erDiagram` of the model on the clipboard.
    ///
    /// The clipboard rather than a file: a Mermaid block is something a user pastes
    /// into a README or an issue, and asking them where to save it first would be a
    /// dialog in the way of a paste.
    ///
    /// Through `EventContext::app_state`, which is where teksilo puts the clipboard
    /// handle. The Slint UI carried its own `arboard` dependency and a hand-copied
    /// Linux ownership workaround for this one call.
    pub fn export_to_mermaid(&self, ctx: &mut EventContext) {
        let diagram = match handling_manifest_commands::export_to_mermaid(&self.app_ctx) {
            Ok(dto) => dto.mermaid_diagram,
            Err(e) => {
                log::error!("could not export the entities to mermaid: {e}");
                return;
            }
        };
        let Some(clipboard) = ctx.app_state::<teksilo::platform::clipboard::ClipboardHandle>()
        else {
            log::error!("no clipboard is installed; the mermaid diagram was not copied");
            return;
        };
        if let Err(e) = clipboard.set_text(&diagram) {
            log::error!("could not write to the clipboard: {e}");
            return;
        }
        // A toast rather than a bare announcement: teksilo's toast is already a
        // live region, so this is both seen and spoken, and doing both by hand would
        // say it twice.
        Toast::success(tr!(status_mermaid_copied())).present(ctx);
    }

    /// Write the edited scalar fields back, as one undo entry.
    pub fn commit(&self) {
        self.single.save(self.stack());
    }

    /// Mark the entity heritage-only, or concrete again.
    ///
    /// Marking it heritage-only is **two** backend writes: the flag, and clearing
    /// the parent it may have had. They are bracketed into one composite so undo
    /// takes both back together; without it, one Ctrl+Z would leave an entity that
    /// is abstract and still inherits from something, which the generator has no
    /// meaning for.
    pub fn set_only_for_heritage(&self, only: bool) {
        if self.single.only_for_heritage().get() == only {
            return;
        }
        let Some(id) = self.selected_id() else {
            return;
        };

        let composite = only && self.inherits_from().is_some();
        if composite
            && let Err(e) = undo_redo_commands::begin_composite(&self.app_ctx, self.stack())
        {
            log::error!("could not start a composite edit: {e}");
            return;
        }

        self.single.set_only_for_heritage(only);
        if only {
            // A heritage-only entity is never instantiated, so neither flag has
            // anything to act on.
            self.single.set_undoable(false);
            self.single.set_single_model(false);
        }
        self.commit();

        if only && self.inherits_from().is_some() {
            self.write_inherits_from(id, None);
        }

        if composite {
            undo_redo_commands::end_composite(&self.app_ctx);
        }
    }

    pub fn set_single_model(&self, value: bool) {
        self.single.set_single_model(value);
        self.commit();
    }

    pub fn set_undoable(&self, value: bool) {
        self.single.set_undoable(value);
        self.commit();
    }

    /// Point the selected entity at a parent, or at none.
    ///
    /// Not part of `commit`: `inherits_from` is a relationship, and the generated
    /// `save` writes scalars only.
    pub fn set_inherits_from(&self, parent: Option<EntityId>) {
        let Some(id) = self.selected_id() else {
            return;
        };
        if self.inherits_from() == parent {
            return;
        }
        self.write_inherits_from(id, parent);
    }

    fn write_inherits_from(&self, id: EntityId, parent: Option<EntityId>) {
        let dto = EntityRelationshipDto {
            id,
            field: EntityRelationshipField::InheritsFrom,
            // An empty list is how this relationship says "no parent"; there is no
            // separate clear command.
            right_ids: parent.into_iter().collect(),
        };
        if let Err(e) = entity_commands::set_entity_relationship(&self.app_ctx, self.stack(), &dto)
        {
            log::error!("could not set the parent of entity {id}: {e}");
            return;
        }
        // The handle's `dto` is what `inherits_from` reads, and a relationship write
        // publishes an `Updated` for the entity, so this only shortens the wait.
        self.single.refresh();
        self.refresh_parents();
    }

    // ── wiring ───────────────────────────────────────────────────────────────

    /// Install the subscriptions. Called from `build`, on **every** build.
    pub fn wire(&self, ctx: &mut BuildContext) {
        // The list itself is pointed at the open manifest by `App`: it is read by
        // more than this screen, so it cannot belong to whichever one is mounted.

        // The form follows the selection.
        let me = self.clone();
        let selection = self.selection.selection_signal();
        ctx.effect(&selection, move |_| me.point_at_selection());
        self.point_at_selection();

        // The subtitles follow the list.
        let me = self.clone();
        let version = self.list.version_signal();
        ctx.effect(&version, move |_| {
            me.prune_selection();
            me.refresh_parents();
        });
        self.refresh_parents();
    }

    fn point_at_selection(&self) {
        let id = self.selected_id();
        if self.single.id() == id {
            return;
        }
        self.single.set_id(id);
    }

    /// Forget a selection whose entity is gone.
    ///
    /// Without it, deleting the selected entity leaves the form bound to an id the
    /// store no longer has, and the next commit writes to nothing.
    fn prune_selection(&self) {
        let live: Vec<EntityId> = self.list.rows().iter().map(|r| r.id).collect();
        self.selection.prune_missing(|id| live.contains(id));
    }

    /// One batched read of every row's parent.
    fn refresh_parents(&self) {
        let rows = self.list.rows();
        if rows.is_empty() {
            self.parents.set_if_changed(HashMap::new());
            return;
        }
        let ids: Vec<EntityId> = rows.iter().map(|r| r.id).collect();
        // The parents are entities of the same workspace, so their names are already
        // in the rows. Reading them here saves a second round trip per parent.
        let names: HashMap<EntityId, String> =
            rows.iter().map(|r| (r.id, r.name.clone())).collect();

        let relationships = match entity_commands::get_entity_relationship_many(
            &self.app_ctx,
            &ids,
            &EntityRelationshipField::InheritsFrom,
        ) {
            Ok(map) => map,
            Err(e) => {
                log::error!("could not read the entity parents: {e}");
                return;
            }
        };

        let parents: HashMap<EntityId, String> = relationships
            .into_iter()
            .filter_map(|(id, parents)| {
                let parent = parents.first()?;
                Some((id, names.get(parent)?.clone()))
            })
            .collect();
        self.parents.set_if_changed(parents);
    }

    fn stack(&self) -> Option<u64> {
        self.ids.entities_stack.get()
    }

    /// The screen's undo stack, for anything that has to read it later rather than
    /// now. A reorder adapter outlives the frame that built it, so it captures this
    /// signal rather than the value.
    pub fn stack_signal(&self) -> Signal<Option<u64>> {
        self.ids.entities_stack.clone()
    }
}
