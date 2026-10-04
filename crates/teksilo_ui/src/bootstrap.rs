//! Walking the store's tree for the ids the rest of the app hangs off.
//!
//! Qleany's store is a tree: `Root` owns `System` and `Workspace`, and a `Workspace`
//! owns its `Global` and its `UserInterface`. Nothing in the UI can read a `Global`
//! before someone has walked that tree, and a walk written twice in two places is
//! the more expensive mistake, so it is written once here and the answers are kept
//! in [`crate::app_ids::AppIds`].

use frontend::AppContext;
use frontend::EntityId;
use frontend::commands::{root_commands, workspace_commands};

/// The `System` row, which owns every generated file the Generate screen lists.
///
/// Read from `Root` rather than assumed. The Slint UI hard-codes
/// `const ROOT_SYSTEM_ID: u64 = 1` in six files; that is true of a freshly seeded
/// store and stops being true the moment anything is created before it.
pub fn system_id(app_ctx: &AppContext) -> Option<EntityId> {
    match root_commands::get_all_root(app_ctx) {
        Ok(roots) => match roots.first() {
            Some(root) => root.system,
            None => {
                log::error!("no Root in the store; initialize_app has not run");
                None
            }
        },
        Err(e) => {
            log::error!("could not read Root: {e}");
            None
        }
    }
}

/// The `Global` and the `UserInterface` an open workspace owns, in that order.
///
/// Both are `OneToOne` and `Strong`, so a workspace that exists always has both;
/// a `None` here means the read failed, not that the manifest has no project
/// settings.
pub fn workspace_children(
    app_ctx: &AppContext,
    workspace_id: EntityId,
) -> Option<(EntityId, EntityId)> {
    match workspace_commands::get_workspace(app_ctx, &workspace_id) {
        Ok(Some(workspace)) => Some((workspace.global, workspace.user_interface)),
        Ok(None) => {
            log::error!("workspace {workspace_id} is gone between the load and the read");
            None
        }
        Err(e) => {
            log::error!("could not read workspace {workspace_id}: {e}");
            None
        }
    }
}
