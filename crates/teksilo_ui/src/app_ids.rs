//! The only mutable global state the app holds: entity ids, and the undo stacks.
//!
//! Everything else is read reactively through the generated `singles` and `models`,
//! which point at the ids here. The pattern is Qleany's own, ported from the C++
//! `SingleWork`: hold an id, let the handle fetch and refresh itself.
//!
//! Tiering, so a later reader knows what may be shared:
//! - **Tier 1**, per process: the `AppContext` and the event hub, owned by `run()`.
//! - **Tier 2**, per open manifest: everything in this struct, cleared on close.
//! - **Tier 3**, per window: the current screen, owned by `App`.

use teksilo::prelude::*;

use frontend::EntityId;

/// Which screen the rail has selected. Undo and redo follow this, because Qleany
/// keeps one stack per screen rather than one for the application.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Screen {
    #[default]
    Home,
    Project,
    Entities,
    Features,
    UserInterface,
    Generate,
}

impl Screen {
    pub const ALL: [Screen; 6] = [
        Screen::Home,
        Screen::Project,
        Screen::Entities,
        Screen::Features,
        Screen::UserInterface,
        Screen::Generate,
    ];

    pub fn index(self) -> usize {
        Screen::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Screen {
        Screen::ALL.get(i).copied().unwrap_or(Screen::Home)
    }
}

/// The ids and stacks that live for as long as a manifest is open.
///
/// Cloning is a handle clone: every clone sees the same signals.
#[derive(Clone)]
pub struct AppIds {
    pub workspace_id: Signal<Option<EntityId>>,
    pub global_id: Signal<Option<EntityId>>,
    pub user_interface_id: Signal<Option<EntityId>>,
    pub system_id: Signal<Option<EntityId>>,

    /// One undo stack per screen that mutates the manifest.
    ///
    /// Four stacks rather than one, matching what the Slint UI already records and
    /// what `docs/undo-redo-architecture.md` calls Approach B: panel-scoped stacks,
    /// explicitly not application-wide time travel. Home and Generate mutate
    /// nothing a user would undo, so they have no stack.
    pub project_stack: Signal<Option<u64>>,
    pub entities_stack: Signal<Option<u64>>,
    pub features_stack: Signal<Option<u64>>,
    pub user_interface_stack: Signal<Option<u64>>,

    /// The screen the rail has selected. Ctrl+Z reads this to pick a stack.
    pub screen: Signal<Screen>,
}

impl Default for AppIds {
    fn default() -> Self {
        Self::new()
    }
}

impl AppIds {
    pub fn new() -> Self {
        Self {
            workspace_id: Signal::new(None),
            global_id: Signal::new(None),
            user_interface_id: Signal::new(None),
            system_id: Signal::new(None),
            project_stack: Signal::new(None),
            entities_stack: Signal::new(None),
            features_stack: Signal::new(None),
            user_interface_stack: Signal::new(None),
            screen: Signal::new(Screen::Home),
        }
    }

    /// The undo stack a given screen writes to, if it has one.
    pub fn stack_for(&self, screen: Screen) -> Signal<Option<u64>> {
        match screen {
            Screen::Project => self.project_stack.clone(),
            Screen::Entities => self.entities_stack.clone(),
            Screen::Features => self.features_stack.clone(),
            Screen::UserInterface => self.user_interface_stack.clone(),
            Screen::Home | Screen::Generate => Signal::new(None),
        }
    }

    /// Every stack that exists, for create-on-load and delete-on-close.
    pub fn stacks(&self) -> [Signal<Option<u64>>; 4] {
        [
            self.project_stack.clone(),
            self.entities_stack.clone(),
            self.features_stack.clone(),
            self.user_interface_stack.clone(),
        ]
    }

    /// Forget the open manifest. Every single and list model pointed at one of
    /// these ids empties itself in response.
    pub fn clear(&self) {
        self.workspace_id.set(None);
        self.global_id.set(None);
        self.user_interface_id.set(None);
        self.system_id.set(None);
        for stack in self.stacks() {
            stack.set(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_round_trips_through_its_index() {
        for screen in Screen::ALL {
            assert_eq!(Screen::from_index(screen.index()), screen);
        }
    }

    #[test]
    fn an_out_of_range_index_falls_back_to_home() {
        assert_eq!(Screen::from_index(99), Screen::Home);
    }

    #[test]
    fn home_and_generate_have_no_undo_stack() {
        let ids = AppIds::new();
        ids.project_stack.set(Some(7));
        assert_eq!(ids.stack_for(Screen::Home).get(), None);
        assert_eq!(ids.stack_for(Screen::Generate).get(), None);
        assert_eq!(ids.stack_for(Screen::Project).get(), Some(7));
    }

    #[test]
    fn clear_forgets_every_id_and_stack() {
        let ids = AppIds::new();
        ids.workspace_id.set(Some(1));
        ids.global_id.set(Some(2));
        ids.system_id.set(Some(3));
        ids.entities_stack.set(Some(4));
        ids.clear();
        assert_eq!(ids.workspace_id.get(), None);
        assert_eq!(ids.global_id.get(), None);
        assert_eq!(ids.system_id.get(), None);
        assert_eq!(ids.entities_stack.get(), None);
    }
}
