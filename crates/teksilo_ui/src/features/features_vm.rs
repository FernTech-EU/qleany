//! The feature list.

use std::rc::Rc;

use teksilo::data::{KeyedSelectionModel, SelectionMode};
use teksilo::prelude::*;
use teksilo::widgets::ValidationState;

use frontend::AppContext;
use frontend::EntityId;
use frontend::direct_access::CreateFeatureDto;

use crate::app_ids::AppIds;
use crate::edit::{UndoAction, labeled};
use crate::models::WorkspaceFeaturesListModel;
use crate::shared::validation::{is_snake_case, required_cased};
use crate::singles::SingleFeature;

/// The name a new feature is given: valid snake_case, so the form it opens is not
/// already showing an error the user did not cause.
const NEW_FEATURE_NAME: &str = "new_feature";

#[derive(Clone)]
pub struct FeaturesViewModel {
    app_ctx: Rc<AppContext>,
    ids: AppIds,
    list: WorkspaceFeaturesListModel,
    single: SingleFeature,
    selection: KeyedSelectionModel<EntityId>,
}

impl std::fmt::Debug for FeaturesViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeaturesViewModel")
            .field("rows", &self.list.len())
            .field("selected", &self.selected_id())
            .finish_non_exhaustive()
    }
}

impl FeaturesViewModel {
    pub fn new(
        app_ctx: Rc<AppContext>,
        ids: AppIds,
        list: WorkspaceFeaturesListModel,
        single: SingleFeature,
    ) -> Self {
        Self {
            app_ctx,
            ids,
            list,
            single,
            selection: KeyedSelectionModel::new(SelectionMode::Single),
        }
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn list(&self) -> WorkspaceFeaturesListModel {
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
            tr!(features_name_required()),
            tr!(features_name_snake_case()),
            is_snake_case,
        )
    }

    // ── commands ─────────────────────────────────────────────────────────────

    pub fn add(&self) {
        let dto = CreateFeatureDto {
            name: NEW_FEATURE_NAME.to_string(),
            ..Default::default()
        };
        let stack = self.stack();
        let created = labeled(&self.app_ctx, stack, UndoAction::AddFeature, || {
            self.list.create(&dto, -1, stack)
        });
        if let Some(id) = created {
            self.selection.select(id);
        }
    }

    /// Delete a feature, with its use cases and their DTOs: the relationships are
    /// strong all the way down, so the backend cascades.
    pub fn remove(&self, id: EntityId) {
        let stack = self.stack();
        labeled(&self.app_ctx, stack, UndoAction::RemoveFeature, || {
            self.list.remove(id, stack)
        });
        if self.selected_id() == Some(id) {
            self.selection.clear();
        }
    }

    pub fn select(&self, id: EntityId) {
        self.selection.select(id);
    }

    pub fn commit(&self) {
        let stack = self.stack();
        labeled(&self.app_ctx, stack, UndoAction::EditFeature, || {
            self.single.save(stack)
        });
    }

    // ── wiring ───────────────────────────────────────────────────────────────

    pub fn wire(&self, ctx: &mut BuildContext) {
        // The list itself is pointed at the open manifest by `App`: it is read by
        // more than this screen, so it cannot belong to whichever one is mounted.

        let me = self.clone();
        let selection = self.selection.selection_signal();
        ctx.effect(&selection, move |_| me.point_at_selection());
        self.point_at_selection();

        let me = self.clone();
        let version = self.list.version_signal();
        ctx.effect(&version, move |_| me.prune_selection());
    }

    fn point_at_selection(&self) {
        let id = self.selected_id();
        if self.single.id() == id {
            return;
        }
        self.single.set_id(id);
    }

    /// Forget a selection whose feature is gone, or the form stays bound to an id
    /// the store no longer has and the next commit writes to nothing.
    fn prune_selection(&self) {
        let live: Vec<EntityId> = self.list.rows().iter().map(|r| r.id).collect();
        self.selection.prune_missing(|id| live.contains(id));
    }

    /// Move a row, as one named undo entry.
    ///
    /// Here rather than in the page: the reorder adapter outlives the frame that
    /// built it, and what it needs is a command, not a model and a stack signal it
    /// would have to name the operation from itself.
    pub fn reorder(&self, id: EntityId, index: i32) {
        let stack = self.stack();
        labeled(&self.app_ctx, stack, UndoAction::ReorderFeatures, || {
            self.list.move_to(id, index, stack)
        });
    }

    fn stack(&self) -> Option<u64> {
        self.ids.features_stack.get()
    }
}
