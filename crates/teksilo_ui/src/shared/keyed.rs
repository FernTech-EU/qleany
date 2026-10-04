//! A list source keyed by the entity's own id.
//!
//! `ListModel`'s own `ListDataSource::Key` is `usize`, the row's index, and its doc
//! says outright that the key is not stable across a mutation. That is fine for a
//! list nothing else touches and wrong for every list in this app: a refresh arrives
//! whenever anything in the manifest changes and renumbers every row, so an
//! index-keyed selection quietly moves to a different entity when someone edits
//! something else.
//!
//! [`crate::shared::reorder::ReorderableSource`] is the same idea for a list that
//! can be reordered. This one is the read-only half, for the lists that cannot.

use std::rc::Rc;

use teksilo::core::signal::ObserverHandle;
use teksilo::data::{DataChange, ListDataSource, ListModel};

use frontend::EntityId;

/// A `ListDataSource` over a `ListModel`, keyed by each row's entity id.
pub struct KeyedSource<T: 'static> {
    model: ListModel<T>,
    key_of: Rc<dyn Fn(&T) -> EntityId>,
}

impl<T: 'static> Clone for KeyedSource<T> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            key_of: self.key_of.clone(),
        }
    }
}

impl<T: 'static> std::fmt::Debug for KeyedSource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyedSource")
            .field("len", &self.model.len())
            .finish_non_exhaustive()
    }
}

impl<T: 'static> KeyedSource<T> {
    pub fn new(model: ListModel<T>, key_of: impl Fn(&T) -> EntityId + 'static) -> Self {
        Self {
            model,
            key_of: Rc::new(key_of),
        }
    }
}

impl<T: 'static> ListDataSource for KeyedSource<T> {
    type Item = T;
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
        (0..self.model.len()).find(|i| self.model.with_item(*i, |t| (self.key_of)(t)) == Some(*key))
    }

    fn observe_changes(&self, f: impl Fn(&DataChange) + 'static) -> ObserverHandle {
        self.model.observe_changes(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_is_the_entity_id_not_the_index() {
        let model: ListModel<(EntityId, &str)> = ListModel::from_vec(vec![(7, "a"), (8, "b")]);
        let source = KeyedSource::new(model.clone(), |row| row.0);

        assert_eq!(source.key_at(0), Some(7));
        assert_eq!(source.index_of(&8), Some(1));

        // The rows move; the keys do not.
        model.replace_all(vec![(8, "b"), (7, "a")]);
        assert_eq!(source.index_of(&7), Some(1));
        assert_eq!(source.index_of(&8), Some(0));
    }

    #[test]
    fn a_row_that_is_gone_has_no_index() {
        let model: ListModel<(EntityId, &str)> = ListModel::from_vec(vec![(7, "a")]);
        let source = KeyedSource::new(model.clone(), |row| row.0);
        model.replace_all(vec![]);
        assert_eq!(source.index_of(&7), None);
        assert_eq!(source.len(), 0);
    }
}
