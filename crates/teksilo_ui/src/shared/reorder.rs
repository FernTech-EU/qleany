//! Drag and keyboard reorder that survives the next refresh.
//!
//! Every ordered list in this app is a `OrderedOneToMany` relationship stored on the
//! owner, and the generated list model already has a `move_to` that rewrites it. What
//! it does not have is a way to tell a `ListView` about it.
//!
//! Handing a generated model's inner `ListModel` to `ListView` with
//! `.reorderable(true)` looks like it works and is the worst of the options: the
//! built-in reorder calls `ListModel::move_item`, which shuffles the local `Vec` and
//! writes nothing. The row visibly moves, and the next refresh, which any edit
//! anywhere publishes, silently puts it back. A user would lose every reorder they
//! made and have no way to tell which ones took.
//!
//! [`ReorderableSource`] is the missing piece: a `ListDataSource` that reads through
//! the model and routes a drop to the backend instead of to the `Vec`. Drag and the
//! built-in Alt+Arrow both go through `accept_drop`, so both commit and both land on
//! the undo stack.

use std::rc::Rc;

use teksilo::core::signal::ObserverHandle;
use teksilo::data::{
    DataChange, DragEligibility, DragSource, DropCommit, DropPosition, DropQuery, DropResponse,
    ListDataSource, ListModel,
};

use frontend::EntityId;

/// Where a moved row has to be inserted for it to land where the user dropped it.
///
/// The backend's `move_relationship` removes the moved ids **first** and then inserts
/// into what is left, so an index measured against the list as the user sees it is
/// one too far whenever the row travels downwards. Getting this wrong is not a crash:
/// every downward drag simply lands one slot past where it was dropped, which reads
/// as the app being slightly broken rather than as a bug with a cause.
///
/// A free function, and public, because it is the whole of the logic worth testing
/// and a widget tree has no business being involved in testing it.
pub fn insertion_index(source: usize, target: usize, position: DropPosition) -> Option<usize> {
    if source == target {
        return None;
    }
    // The target's index once the moved row is gone.
    let base = if source < target { target - 1 } else { target };
    Some(match position {
        DropPosition::Before => base,
        // `Into` means reparent, which a flat list has no notion of. It is rejected
        // in `can_accept`, so this arm is only ever reached by a caller driving the
        // source directly, and treating it as After is the harmless reading.
        DropPosition::After | DropPosition::Into => base + 1,
    })
}

/// A `ListDataSource` over a generated list model whose reorder commits.
///
/// Holds the model and two closures, never a view-model. A view-model owns the
/// widget that owns the `ListView` that owns this source, so capturing one would
/// close an `Rc` cycle and leak the whole screen every time it was rebuilt.
pub struct ReorderableSource<T: 'static> {
    model: ListModel<T>,
    key_of: Rc<dyn Fn(&T) -> EntityId>,
    /// `(id, index)`, straight to the generated `move_to`, which is what talks to
    /// the backend and to the undo stack.
    move_to: Rc<dyn Fn(EntityId, i32)>,
}

impl<T: 'static> Clone for ReorderableSource<T> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            key_of: self.key_of.clone(),
            move_to: self.move_to.clone(),
        }
    }
}

impl<T: 'static> std::fmt::Debug for ReorderableSource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReorderableSource")
            .field("len", &self.model.len())
            .finish_non_exhaustive()
    }
}

impl<T: 'static> ReorderableSource<T> {
    pub fn new(
        model: ListModel<T>,
        key_of: impl Fn(&T) -> EntityId + 'static,
        move_to: impl Fn(EntityId, i32) + 'static,
    ) -> Self {
        Self {
            model,
            key_of: Rc::new(key_of),
            move_to: Rc::new(move_to),
        }
    }

    fn position_of(&self, key: &EntityId) -> Option<usize> {
        (0..self.model.len()).find(|i| self.model.with_item(*i, |t| (self.key_of)(t)) == Some(*key))
    }
}

impl<T: 'static> ListDataSource for ReorderableSource<T> {
    type Item = T;
    /// The entity's own id, not its index. An index is not identity here: a refresh
    /// arrives whenever anything in the manifest changes and renumbers every row, so
    /// index-keyed selection would jump to a different entity on someone else's edit.
    type Key = EntityId;

    fn len(&self) -> usize {
        self.model.len()
    }

    fn with_item<R>(&self, index: usize, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.model.with_item(index, f)
    }

    fn key_at(&self, index: usize) -> Option<EntityId> {
        self.model.with_item(index, |t| (self.key_of)(t))
    }

    fn index_of(&self, key: &EntityId) -> Option<usize> {
        self.position_of(key)
    }

    fn observe_changes(&self, f: impl Fn(&DataChange) + 'static) -> ObserverHandle {
        self.model.observe_changes(f)
    }

    fn drag(&self, _key: &EntityId) -> DragEligibility {
        DragEligibility::CanDrag
    }

    fn can_accept(&self, query: &DropQuery<'_, EntityId>) -> DropResponse {
        match &query.source {
            // `Into` is a reparent. These lists are flat, so there is nothing to
            // reparent into and saying so up front is what stops the view from
            // painting a drop indicator for a drop it would then refuse.
            DragSource::SameView { .. } => match query.position {
                DropPosition::Into => DropResponse::Reject,
                DropPosition::Before | DropPosition::After => DropResponse::Accept,
            },
            // Nothing in this app drags between lists. A field belongs to one entity
            // and a use case to one feature, so accepting a foreign row would mean
            // inventing a reparent the backend has no command for.
            DragSource::Foreign { .. } => DropResponse::Reject,
        }
    }

    fn accept_drop(&self, commit: DropCommit<'_, EntityId>) -> bool {
        let DragSource::SameView { key: from } = commit.source else {
            return false;
        };
        let (Some(source), Some(target)) =
            (self.position_of(&from), self.position_of(&commit.target))
        else {
            return false;
        };
        let Some(index) = insertion_index(source, target, commit.position) else {
            return false;
        };
        // No local `move_item`: the model is refreshed by the command below, so
        // moving the `Vec` here as well would show the row in two places for a frame
        // and hide a failed write behind a move that appeared to work.
        (self.move_to)(from, index as i32);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both directions, because the correction only applies to one of them and a
    /// test of the other alone passes with the arithmetic missing entirely.
    #[test]
    fn moving_down_accounts_for_the_row_leaving_its_old_slot() {
        // [a b c d], drag a (0) to after c (2). Removing a leaves [b c d], where c is
        // at 1, so a goes to 2: [b c a d].
        assert_eq!(insertion_index(0, 2, DropPosition::After), Some(2));
        // Before c: [b a c d].
        assert_eq!(insertion_index(0, 2, DropPosition::Before), Some(1));
    }

    #[test]
    fn moving_up_needs_no_correction() {
        // [a b c d], drag d (3) before b (1). Removing d leaves [a b c], b still at 1.
        assert_eq!(insertion_index(3, 1, DropPosition::Before), Some(1));
        assert_eq!(insertion_index(3, 1, DropPosition::After), Some(2));
    }

    #[test]
    fn a_row_dropped_on_itself_does_not_move() {
        assert_eq!(insertion_index(2, 2, DropPosition::Before), None);
        assert_eq!(insertion_index(2, 2, DropPosition::After), None);
    }

    /// The arithmetic against a real list, so the reasoning above is checked rather
    /// than restated.
    #[test]
    fn the_index_is_the_one_that_lands_the_row_where_it_was_dropped() {
        let original = ["a", "b", "c", "d"];
        for source in 0..original.len() {
            for target in 0..original.len() {
                for (position, expected_neighbour) in
                    [(DropPosition::Before, true), (DropPosition::After, false)]
                {
                    let Some(index) = insertion_index(source, target, position) else {
                        continue;
                    };
                    let mut remaining: Vec<&str> = original.to_vec();
                    let moved = remaining.remove(source);
                    remaining.insert(index, moved);

                    let landed = remaining.iter().position(|s| *s == moved).unwrap();
                    let neighbour = remaining
                        .iter()
                        .position(|s| *s == original[target])
                        .unwrap();
                    if expected_neighbour {
                        assert_eq!(
                            landed + 1,
                            neighbour,
                            "{moved} dropped before {} in {original:?}",
                            original[target]
                        );
                    } else {
                        assert_eq!(
                            neighbour + 1,
                            landed,
                            "{moved} dropped after {} in {original:?}",
                            original[target]
                        );
                    }
                }
            }
        }
    }

    /// The source reads through the model, so a refresh that replaced every row is
    /// visible to it without anything being told.
    #[test]
    fn the_source_reads_the_model_rather_than_a_copy_of_it() {
        let model: ListModel<(EntityId, &str)> = ListModel::from_vec(vec![(7, "a"), (8, "b")]);
        let source = ReorderableSource::new(model.clone(), |row| row.0, |_, _| {});

        assert_eq!(source.len(), 2);
        assert_eq!(source.key_at(1), Some(8));
        assert_eq!(source.index_of(&8), Some(1));

        model.replace_all(vec![(9, "c")]);
        assert_eq!(source.len(), 1);
        assert_eq!(source.key_at(0), Some(9));
        assert_eq!(source.index_of(&8), None, "a row that is gone has no index");
    }

    /// A drop routes to the backend closure with the corrected index, and does not
    /// touch the local `Vec`: the refresh the command triggers is what moves the row.
    #[test]
    fn a_drop_commands_the_backend_and_leaves_the_model_alone() {
        use std::cell::RefCell;

        let model: ListModel<(EntityId, &str)> =
            ListModel::from_vec(vec![(1, "a"), (2, "b"), (3, "c")]);
        let commands = Rc::new(RefCell::new(Vec::new()));
        let recorder = commands.clone();
        let source = ReorderableSource::new(
            model.clone(),
            |row| row.0,
            move |id, index| recorder.borrow_mut().push((id, index)),
        );

        let applied = source.accept_drop(DropCommit {
            source: DragSource::SameView { key: 1 },
            target: 3,
            position: DropPosition::After,
        });

        assert!(applied);
        assert_eq!(&*commands.borrow(), &[(1, 2)]);
        assert_eq!(
            model.with_item(0, |r| r.0),
            Some(1),
            "the local order must wait for the refresh"
        );
    }

    /// The source is a `ListDataSource` a `ListView` will actually take.
    ///
    /// The arithmetic above can be right while the trait bounds are wrong, and the
    /// place that would be discovered is the first list screen. Constructing the view
    /// here is a compile-time proof that `from_source_keyed` accepts it, keyed on the
    /// entity id rather than on an index.
    #[test]
    fn a_list_view_accepts_it_keyed_by_entity_id() {
        use teksilo::data::{KeyedSelectionModel, SelectionMode};
        use teksilo::prelude::lit;
        use teksilo::widgets::{ListView, StandardListItem};

        let model: ListModel<(EntityId, String)> =
            ListModel::from_vec(vec![(1, "Alpha".to_string())]);
        let source = ReorderableSource::new(model, |row| row.0, |_, _| {});
        let keyed: KeyedSelectionModel<EntityId> = KeyedSelectionModel::new(SelectionMode::Single);

        let _list = ListView::from_source_keyed(source, keyed, |_index, row, selected| {
            Box::new(StandardListItem::new(lit!(row.1.clone())).selected(selected))
        })
        .reorderable(true);
    }

    #[test]
    fn a_drop_from_another_view_is_refused() {
        let model: ListModel<(EntityId, &str)> = ListModel::from_vec(vec![(1, "a")]);
        let source = ReorderableSource::new(
            model,
            |row| row.0,
            |_, _| panic!("a foreign drop must not reach the backend"),
        );

        assert_eq!(
            source.can_accept(&DropQuery {
                source: DragSource::SameView { key: 1 },
                target: 1,
                position: DropPosition::Into,
            }),
            DropResponse::Reject,
            "a flat list has nothing to reparent into"
        );
    }
}
