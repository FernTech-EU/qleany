//! Undo and redo for the screen the user is looking at.
//!
//! The generated `UndoRedoBinder` takes its stack id once, at construction. That
//! fits an app with one stack; this one has four, and their ids are minted afresh
//! every time a manifest is opened. So the binder's shape is kept and its stack is
//! read from a signal instead: the active screen's.

use std::rc::Rc;

use teksilo::prelude::*;

use frontend::AppContext;
use frontend::commands::undo_redo_commands;
use frontend::common::event::{Event, Origin, UndoRedoEvent};

use crate::app_ids::{AppIds, Screen};
use crate::edit::UndoAction;

#[derive(Clone)]
pub struct UndoViewModel {
    app_ctx: Rc<AppContext>,
    ids: AppIds,
    can_undo: Signal<bool>,
    can_redo: Signal<bool>,
    /// What the next undo and redo would take back, when this UI named them.
    undo_action: Signal<Option<UndoAction>>,
    redo_action: Signal<Option<UndoAction>>,
}

impl std::fmt::Debug for UndoViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoViewModel")
            .field("can_undo", &self.can_undo.get())
            .field("can_redo", &self.can_redo.get())
            .finish_non_exhaustive()
    }
}

impl UndoViewModel {
    pub fn new(app_ctx: Rc<AppContext>, ids: AppIds) -> Self {
        Self {
            app_ctx,
            ids,
            can_undo: Signal::new(false),
            can_redo: Signal::new(false),
            undo_action: Signal::new(None),
            redo_action: Signal::new(None),
        }
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn can_undo(&self) -> Signal<bool> {
        self.can_undo.clone()
    }

    pub fn can_redo(&self) -> Signal<bool> {
        self.can_redo.clone()
    }

    /// What the Edit menu's Undo row says.
    ///
    /// Named when this UI recorded a name, plain otherwise: an entry with no label
    /// is better described as "Undo" than as a guess.
    ///
    /// A `LocalizedString` that observes the action rather than a `Signal` of one.
    /// `MenuEntry::new` takes its title once and a menu model is built once, so a
    /// signal would be read at build time and frozen. `also_observing` is the seam
    /// teksilo provides for exactly this, and its own doc uses an Undo row as the
    /// example.
    pub fn undo_label(&self) -> LocalizedString {
        Self::describe_stack(&self.undo_action, true)
    }

    pub fn redo_label(&self) -> LocalizedString {
        Self::describe_stack(&self.redo_action, false)
    }

    fn describe_stack(action: &Signal<Option<UndoAction>>, undo: bool) -> LocalizedString {
        let read = action.clone();
        localized(move || match read.get() {
            Some(action) => {
                let described = action.describe().resolve_now();
                if undo {
                    tr!(undo_menu_labelled(action = described)).resolve_now()
                } else {
                    tr!(redo_menu_labelled(action = described)).resolve_now()
                }
            }
            None if undo => tr!(undo_menu_plain()).resolve_now(),
            None => tr!(redo_menu_plain()).resolve_now(),
        })
        .also_observing(action)
    }

    // ── commands ─────────────────────────────────────────────────────────────

    /// Undo on the active screen's stack, and say what was undone.
    ///
    /// US-UNDO-04: the stack is the one the visible screen writes to, so Ctrl+Z can
    /// never reach an edit the user cannot see. Home and Generate have no stack, and
    /// `stack_for` answers `None` for them, which the guard below turns into a no-op
    /// rather than an undo on the default stack.
    pub fn undo(&self, ctx: &mut EventContext) {
        let Some(stack) = self.active_stack() else {
            return;
        };
        let undone = self.undo_action.get();
        if let Err(e) = undo_redo_commands::undo(&self.app_ctx, Some(stack)) {
            log::error!("undo failed: {e}");
            return;
        }
        self.refresh();
        self.announce(ctx, undone, true);
    }

    pub fn redo(&self, ctx: &mut EventContext) {
        let Some(stack) = self.active_stack() else {
            return;
        };
        let redone = self.redo_action.get();
        if let Err(e) = undo_redo_commands::redo(&self.app_ctx, Some(stack)) {
            log::error!("redo failed: {e}");
            return;
        }
        self.refresh();
        self.announce(ctx, redone, false);
    }

    /// Say what just happened, once.
    ///
    /// A toast rather than a bare announcement: teksilo's toast is already a live
    /// region, so this is both seen and spoken. Nothing is said for an unnamed
    /// entry, because "Undone:" with nothing after it is worse than silence.
    fn announce(&self, ctx: &mut EventContext, action: Option<UndoAction>, undone: bool) {
        let Some(action) = action else {
            return;
        };
        let described = action.describe().resolve_now();
        let message = if undone {
            tr!(undo_toast(action = described))
        } else {
            tr!(redo_toast(action = described))
        };
        teksilo::widgets::Toast::info(message).present(ctx);
    }

    /// Give up every stack. Loading a manifest writes hundreds of rows, and none of
    /// it is an edit the user made.
    pub fn clear_all(&self) {
        undo_redo_commands::clear_all_stacks(&self.app_ctx);
        self.refresh();
    }

    // ── wiring ───────────────────────────────────────────────────────────────

    pub fn wire(&self, ctx: &mut BuildContext) {
        // `StackChanged` is not optional. It is the only event that fires when
        // history is *created*, so without it the Undo row never enables: the
        // manager otherwise announces only the consumption of history.
        for event in [
            UndoRedoEvent::Undone,
            UndoRedoEvent::Redone,
            UndoRedoEvent::StackChanged,
        ] {
            let me = self.clone();
            ctx.subscribe_event(Origin::UndoRedo(event), move |_e: &Event| me.refresh());
        }

        // The active screen decides which stack is being asked about, so a screen
        // change is as much a reason to re-read as an edit is.
        let me = self.clone();
        let screen = self.ids.screen.clone();
        ctx.effect(&screen, move |_| me.refresh());

        // And so is opening or closing a manifest, which mints and discards them.
        for stack in self.ids.stacks() {
            let me = self.clone();
            ctx.effect(&stack, move |_| me.refresh());
        }

        self.refresh();
    }

    /// The stack the visible screen writes to.
    fn active_stack(&self) -> Option<u64> {
        self.ids.stack_for(self.ids.screen.get()).get()
    }

    /// `set_if_changed` throughout: these are bound at rebuild level, and an
    /// unconditional write to one of those is a build, dirty, rebuild loop at frame
    /// rate.
    fn refresh(&self) {
        let stack = self.active_stack();
        let (can_undo, can_redo) = match stack {
            Some(stack) => (
                undo_redo_commands::can_undo(&self.app_ctx, Some(stack)),
                undo_redo_commands::can_redo(&self.app_ctx, Some(stack)),
            ),
            // No stack is not an empty stack: Home and Generate mutate nothing a
            // user would undo, and offering them a live Undo would undo someone
            // else's edit.
            None => (false, false),
        };
        self.can_undo.set_if_changed(can_undo);
        self.can_redo.set_if_changed(can_redo);

        let undo_action = stack
            .and_then(|s| undo_redo_commands::undo_label(&self.app_ctx, Some(s)))
            .and_then(|label| UndoAction::of(&label));
        let redo_action = stack
            .and_then(|s| undo_redo_commands::redo_label(&self.app_ctx, Some(s)))
            .and_then(|label| UndoAction::of(&label));
        self.undo_action.set_if_changed(undo_action);
        self.redo_action.set_if_changed(redo_action);
    }
}

/// Whether a screen has an undo stack at all.
///
/// A free function so the rule is checkable without a backend: Home offers nothing
/// to undo, and Generate writes files rather than manifest rows, which the undo
/// manager has no command for.
pub fn has_undo_stack(screen: Screen) -> bool {
    !matches!(screen, Screen::Home | Screen::Generate)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// US-UNDO-04: the two screens that mutate nothing a user would undo.
    #[test]
    fn home_and_generate_have_no_undo() {
        assert!(!has_undo_stack(Screen::Home));
        assert!(!has_undo_stack(Screen::Generate));
        for screen in [
            Screen::Project,
            Screen::Entities,
            Screen::Features,
            Screen::UserInterface,
        ] {
            assert!(has_undo_stack(screen), "{screen:?}");
        }
    }

    /// US-UNDO-04: undo targets the active screen's stack, and only that one.
    #[test]
    fn the_active_stack_follows_the_screen() {
        let ids = AppIds::new();
        ids.project_stack.set(Some(1));
        ids.entities_stack.set(Some(2));
        let vm = UndoViewModel::new(Rc::new(AppContext::new()), ids.clone());

        ids.screen.set(Screen::Project);
        assert_eq!(vm.active_stack(), Some(1));

        ids.screen.set(Screen::Entities);
        assert_eq!(vm.active_stack(), Some(2));

        ids.screen.set(Screen::Home);
        assert_eq!(vm.active_stack(), None, "Home has no stack to undo on");
    }

    /// A screen with no stack reads as nothing to undo, rather than falling through
    /// to the default stack, which would undo an edit made somewhere else.
    #[test]
    fn a_screen_without_a_stack_offers_nothing() {
        let ids = AppIds::new();
        ids.screen.set(Screen::Generate);
        let vm = UndoViewModel::new(Rc::new(AppContext::new()), ids);
        vm.refresh();
        assert!(!vm.can_undo().get());
        assert!(!vm.can_redo().get());
    }

    /// US-UNDO-03: the menu row names the operation when there is a name, and reads
    /// plainly when there is not.
    #[test]
    fn the_menu_row_names_what_it_would_undo() {
        let vm = UndoViewModel::new(Rc::new(AppContext::new()), AppIds::new());
        let label = vm.undo_label();
        // The `&` is the keyboard mnemonic, which the menu strips when it draws the
        // row and `resolve_now` does not.
        assert_eq!(label.resolve_now(), "&Undo");

        // The same string, re-resolved: the row is built once and has to follow the
        // stack without being rebuilt.
        vm.undo_action.set(Some(UndoAction::AddEntity));
        assert_eq!(label.resolve_now(), "&Undo add entity");

        vm.undo_action.set(None);
        assert_eq!(
            label.resolve_now(),
            "&Undo",
            "an unnamed entry reads plainly"
        );
    }
}
