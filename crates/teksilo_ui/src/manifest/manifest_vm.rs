//! The manifest's lifecycle, and the dirty flag every other screen reads.
//!
//! This is the one view-model the shell depends on: the window title, the Save
//! button, the navigation rail and every File menu row read its signals.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use teksilo::platform::file_dialog::{FileDialogRequest, FileDialogResult};
use teksilo::prelude::*;

use frontend::AppContext;
use frontend::commands::{handling_manifest_commands, undo_redo_commands};
use frontend::common::event::{DirectAccessEntity, EntityEvent, Event, Origin};
use frontend::handling_manifest::dtos::{LoadDto, SaveDto};

use crate::app_ids::AppIds;
use crate::bootstrap;

/// Every entity whose creation, update or removal means the manifest on disk is
/// now out of date.
///
/// `Created` is in the list on purpose. Without it, adding an entity leaves the
/// manifest reading clean, the Save button disabled, and the user one window-close
/// away from losing the entity they just made.
const DIRTYING_ENTITIES: &[DirectAccessEntity] = &[
    DirectAccessEntity::Workspace(EntityEvent::Updated),
    DirectAccessEntity::Entity(EntityEvent::Created),
    DirectAccessEntity::Entity(EntityEvent::Updated),
    DirectAccessEntity::Entity(EntityEvent::Removed),
    DirectAccessEntity::Field(EntityEvent::Created),
    DirectAccessEntity::Field(EntityEvent::Updated),
    DirectAccessEntity::Field(EntityEvent::Removed),
    DirectAccessEntity::Feature(EntityEvent::Created),
    DirectAccessEntity::Feature(EntityEvent::Updated),
    DirectAccessEntity::Feature(EntityEvent::Removed),
    DirectAccessEntity::UseCase(EntityEvent::Created),
    DirectAccessEntity::UseCase(EntityEvent::Updated),
    DirectAccessEntity::UseCase(EntityEvent::Removed),
    DirectAccessEntity::Dto(EntityEvent::Created),
    DirectAccessEntity::Dto(EntityEvent::Updated),
    DirectAccessEntity::Dto(EntityEvent::Removed),
    DirectAccessEntity::DtoField(EntityEvent::Created),
    DirectAccessEntity::DtoField(EntityEvent::Updated),
    DirectAccessEntity::DtoField(EntityEvent::Removed),
    DirectAccessEntity::Global(EntityEvent::Updated),
    DirectAccessEntity::UserInterface(EntityEvent::Updated),
    DirectAccessEntity::Relationship(EntityEvent::Created),
    DirectAccessEntity::Relationship(EntityEvent::Updated),
    DirectAccessEntity::Relationship(EntityEvent::Removed),
];

/// How long to wait for a load's own burst to *start* arriving.
///
/// Not a window during which edits are ignored: that would swallow a real one.
/// It bounds only the gap between the command returning and the first event
/// reaching this thread, after which a quiet frame means the burst is over. A
/// load that genuinely publishes nothing ends its settle here instead of waiting
/// for an event that is never coming.
const SETTLE_START_GRACE: Duration = Duration::from_millis(250);

#[derive(Clone)]
pub struct ManifestViewModel {
    app_ctx: Rc<AppContext>,
    ids: AppIds,
    is_open: Signal<bool>,
    is_saved: Signal<bool>,
    path: Signal<String>,
    busy: Signal<bool>,
    error: Signal<Option<LocalizedString>>,
    success: Signal<Option<LocalizedString>>,
    /// A dirtying event arrived since the last frame.
    saw_edit: Rc<Cell<bool>>,
    /// A load or a close is draining its event burst; the manifest is clean once
    /// the burst goes quiet. See `wire` for why this cannot simply be a flag set
    /// before the command and cleared after it.
    settling_clean: Rc<Cell<bool>>,
    /// When to give up waiting for the current settle's burst to begin.
    ///
    /// A quiet frame alone is not proof a burst is over, because it may not have
    /// started: the events are delivered by a background thread, so a frame can
    /// slip between the operation returning and its first event arriving. Ending
    /// the settle there let the rest of the burst read as user edits, and marked
    /// a manifest dirty immediately after it had been closed.
    settling_until: Rc<Cell<Option<Instant>>>,
    /// Whether the current settle has seen any event yet.
    settle_saw_any: Rc<Cell<bool>>,
}

impl std::fmt::Debug for ManifestViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManifestViewModel")
            .field("open", &self.is_open.get())
            .field("saved", &self.is_saved.get())
            .finish_non_exhaustive()
    }
}

impl ManifestViewModel {
    pub fn new(app_ctx: Rc<AppContext>, ids: AppIds) -> Self {
        Self {
            app_ctx,
            ids,
            is_open: Signal::new(false),
            is_saved: Signal::new(true),
            path: Signal::new(String::new()),
            busy: Signal::new(false),
            error: Signal::new(None),
            success: Signal::new(None),
            saw_edit: Rc::new(Cell::new(false)),
            settling_clean: Rc::new(Cell::new(false)),
            settling_until: Rc::new(Cell::new(None)),
            settle_saw_any: Rc::new(Cell::new(false)),
        }
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn is_open(&self) -> Signal<bool> {
        self.is_open.clone()
    }

    pub fn is_saved(&self) -> Signal<bool> {
        self.is_saved.clone()
    }

    pub fn busy(&self) -> Signal<bool> {
        self.busy.clone()
    }

    pub fn path(&self) -> Signal<String> {
        self.path.clone()
    }

    pub fn error(&self) -> Signal<Option<LocalizedString>> {
        self.error.clone()
    }

    pub fn success(&self) -> Signal<Option<LocalizedString>> {
        self.success.clone()
    }

    /// There is something to write, and nothing in the way of writing it.
    pub fn can_save(&self) -> Signal<bool> {
        self.is_open
            .zip(&self.is_saved)
            .zip(&self.busy)
            .map(|((open, saved), busy)| *open && !*saved && !*busy)
    }

    /// The window title, for the three states the Slint UI also distinguished.
    pub fn title(&self) -> Signal<LocalizedString> {
        self.is_open.zip(&self.path).map(|(open, path)| {
            if !*open {
                tr!(window_title_no_manifest())
            } else if path.is_empty() {
                tr!(window_title_unsaved())
            } else {
                tr!(window_title_with_path(path = path.clone()))
            }
        })
    }

    // ── commands ─────────────────────────────────────────────────────────────

    /// Load a manifest from a path. Any open manifest is closed first.
    ///
    /// The failure goes to the error channel the shell already shows, which is what
    /// every caller that has nothing to add wants. A caller that has to *stop* on a
    /// failure, rather than report one, uses [`Self::try_open_path`].
    pub fn open_path(&self, path: &str) {
        let _ = self.try_open_path(path);
    }

    /// Load a manifest, and say whether it worked.
    ///
    /// Reports through the error channel as well, so a caller can ignore the result
    /// and get the same behaviour as [`Self::open_path`].
    pub fn try_open_path(&self, path: &str) -> Result<(), String> {
        if self.is_open.get() {
            self.close();
        }
        self.busy.set(true);
        let result = handling_manifest_commands::load(
            &self.app_ctx,
            &LoadDto {
                manifest_path: path.to_string(),
            },
        );
        self.busy.set(false);
        match result {
            Ok(dto) => {
                self.adopt(dto.workspace_id, &dto.manifest_path);
                self.error.set(None);
                Ok(())
            }
            Err(e) => {
                let message = e.to_string();
                self.fail(&message);
                Err(message)
            }
        }
    }

    /// Write the manifest back to the path it came from.
    pub fn save(&self) {
        let path = self.path.get();
        if path.is_empty() {
            // Nothing to write over. The caller is expected to run the Save-as flow
            // instead; saying so is better than writing somewhere arbitrary.
            self.fail("this manifest has no path yet; use Save as");
            return;
        }
        self.save_to(&path);
    }

    /// Write the manifest to a chosen path, which becomes its path.
    pub fn save_to(&self, path: &str) {
        self.busy.set(true);
        let result = handling_manifest_commands::save(
            &self.app_ctx,
            &SaveDto {
                manifest_path: path.to_string(),
            },
        );
        self.busy.set(false);
        match result {
            Ok(()) => {
                self.path.set(path.to_string());
                self.is_saved.set(true);
                self.success.set(Some(tr!(status_manifest_saved())));
                self.error.set(None);
            }
            Err(e) => self.fail(&e.to_string()),
        }
    }

    /// Forget the open manifest.
    pub fn close(&self) {
        if let Err(e) = handling_manifest_commands::close(&self.app_ctx) {
            self.fail(&e.to_string());
            return;
        }
        self.close_undo_stacks();
        self.ids.clear();
        self.is_open.set(false);
        self.path.set(String::new());
        self.error.set(None);
        // A close empties the store, which publishes a removal per row. Same burst,
        // same settle as a load.
        self.begin_settling_clean();
    }

    /// Native picker for an existing manifest, then load it.
    pub fn pick_open(&self, ctx: &mut EventContext) {
        let me = self.clone();
        let req = FileDialogRequest::pick_file()
            .title(tr!(dialog_open_manifest()).resolve_now())
            .add_filter(
                tr!(dialog_manifest_filter()).resolve_now(),
                &["yaml", "yml"],
            )
            .default_file_name("qleany.yaml");
        let _ = ctx.pick_file(req, move |res, _ectx| {
            if let FileDialogResult::File(Some(path)) = res {
                me.open_path(&path.to_string_lossy());
            }
        });
    }

    /// Native picker for a destination, then write there.
    pub fn pick_save_as(&self, ctx: &mut EventContext) {
        let me = self.clone();
        let req = FileDialogRequest::save_file()
            .title(tr!(dialog_save_manifest_as()).resolve_now())
            .add_filter(
                tr!(dialog_manifest_filter()).resolve_now(),
                &["yaml", "yml"],
            )
            .default_file_name("qleany.yaml");
        let _ = ctx.pick_file(req, move |res, _ectx| {
            if let FileDialogResult::File(Some(path)) = res {
                me.save_to(&path.to_string_lossy());
            }
        });
    }

    /// Install the subscriptions. Called from `App::build`, on **every** build: a
    /// `BuildContext` subscription lives exactly one build cycle, so a guard here
    /// would leave the dirty flag deaf after the first rebuild.
    pub fn wire(&self, ctx: &mut BuildContext) {
        // `wake_at` rather than `frame_tick` alone: teksilo pumps a frame only when
        // something asks for one, so an effect hung on the tick alone might not run
        // until something unrelated woke the window.
        let wake = ctx.wake_at_handle();

        for entity in DIRTYING_ENTITIES {
            let saw_edit = self.saw_edit.clone();
            let wake = wake.clone();
            ctx.subscribe_event(Origin::DirectAccess(entity.clone()), move |_e: &Event| {
                saw_edit.set(true);
                wake.set(Some(std::time::Instant::now()));
            });
        }

        let tick = ctx.frame_tick();
        let me = self.clone();
        let settle_wake = wake.clone();
        ctx.effect(&tick, move |_| {
            me.settle_dirty();
            // Keep the window awake to the end of the settle. Teksilo pumps a
            // frame only when something asks for one, and once the burst stops
            // arriving nothing else would, so the settle would never be closed
            // out and the next real edit would be swallowed by it.
            if let Some(until) = me.settling_until.get() {
                settle_wake.set(Some(until));
            }
        });
    }

    /// One frame's worth of the dirty state machine.
    ///
    /// Split out from the effect so it can be driven directly in a test: the
    /// interesting property is the ordering, and no test can make real events
    /// arrive in a chosen order.
    pub(crate) fn settle_dirty(&self) {
        self.settle_dirty_at(Instant::now());
    }

    /// One frame's worth, against a clock the caller supplies.
    ///
    /// Split from [`Self::settle_dirty`] so a test can drive the window without
    /// sleeping: the interesting property is an ordering against a deadline, and
    /// no test can make real events arrive at a chosen moment.
    pub(crate) fn settle_dirty_at(&self, now: Instant) {
        let settling = self.settling_clean.get();

        if self.saw_edit.replace(false) {
            if settling {
                // The burst is under way, so a quiet frame from here on means it
                // is over rather than that it has not begun.
                self.settle_saw_any.set(true);
                return;
            }
            // Nothing is open, so nothing can have been edited: this is the tail
            // of a close's own removal burst, arriving after its settle ended.
            if !self.is_open.get() {
                return;
            }
            self.is_saved.set_if_changed(false);
            return;
        }

        if !settling {
            return;
        }
        // A quiet frame. If nothing has arrived yet the burst has not started,
        // so wait, but not forever.
        if !self.settle_saw_any.get() && self.settling_until.get().is_some_and(|until| now < until)
        {
            return;
        }
        self.settling_clean.set(false);
        self.settling_until.set(None);
        self.settle_saw_any.set(false);
        self.is_saved.set_if_changed(true);
    }

    // ── internals ────────────────────────────────────────────────────────────

    /// Take the open manifest's identity from the event that announced it.
    ///
    /// The workspace id and the path both ride on the event rather than being read
    /// back from the command's return value, so a `Create` and a `Load` are handled
    /// by one function and neither can disagree with the other.
    fn adopt(&self, workspace_id: u64, path: &str) {
        self.ids.workspace_id.set(Some(workspace_id));
        // The screens edit a `Global` and a `UserInterface` rather than the
        // workspace itself, and both hang off it. Resolved here, once, so no screen
        // has to walk the tree to find the row it was opened to edit.
        if let Some((global, user_interface)) =
            bootstrap::workspace_children(&self.app_ctx, workspace_id)
        {
            self.ids.global_id.set(Some(global));
            self.ids.user_interface_id.set(Some(user_interface));
        }
        self.open_undo_stacks();
        self.path.set(path.to_string());
        self.is_open.set(true);
        self.begin_settling_clean();
    }

    /// Mint one undo stack per screen that mutates the manifest.
    ///
    /// After the load, never before: a load writes the whole store, and a stack that
    /// existed while it ran would fill with hundreds of entries none of which is an
    /// edit the user made. Any stack left over from a previous manifest is deleted
    /// first, so reopening cannot leak one.
    fn open_undo_stacks(&self) {
        self.close_undo_stacks();
        // Anything the manager still holds belonged to the manifest that closed.
        undo_redo_commands::clear_all_stacks(&self.app_ctx);
        for stack in self.ids.stacks() {
            stack.set(Some(undo_redo_commands::create_new_stack(&self.app_ctx)));
        }
    }

    /// Give every stack back. The history belonged to the manifest that is closing;
    /// keeping it would let Ctrl+Z replay edits into whatever opens next.
    fn close_undo_stacks(&self) {
        for stack in self.ids.stacks() {
            if let Some(id) = stack.get()
                && let Err(e) = undo_redo_commands::delete_stack(&self.app_ctx, id)
            {
                log::warn!("could not delete undo stack {id}: {e}");
            }
            stack.set(None);
        }
    }

    /// A load or a close writes the whole store, and every one of those writes
    /// publishes an event this view-model would otherwise read as a user edit.
    ///
    /// The events are buffered by the unit of work and flushed together when the
    /// transaction commits, so they arrive *after* the command returns. Setting
    /// `is_saved = true` here and walking away would therefore be overwritten by
    /// the burst that follows. Instead the flag settles: `wire`'s frame effect
    /// declares the manifest clean only after a frame passes with no dirtying
    /// event, which is robust however many frames the burst spans.
    fn begin_settling_clean(&self) {
        self.settling_clean.set(true);
        self.settling_until
            .set(Some(Instant::now() + SETTLE_START_GRACE));
        self.settle_saw_any.set(false);
        self.saw_edit.set(false);
        self.is_saved.set(true);
    }

    /// Surface a backend failure.
    ///
    /// The message itself comes from `anyhow` and is not translated; the frame
    /// around it is, which is the honest split: inventing Fluent keys for every
    /// backend error string would be a catalogue that drifts from the backend.
    fn fail(&self, message: &str) {
        self.error
            .set(Some(tr!(status_error(message = message.to_string()))));
        self.success.set(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontend::commands::handling_app_lifecycle_commands;

    fn vm() -> ManifestViewModel {
        let ctx = Rc::new(AppContext::new());
        handling_app_lifecycle_commands::initialize_app(&ctx).expect("initialize_app");
        ManifestViewModel::new(ctx, AppIds::new())
    }

    /// The case that sent the first attempt at this back to the drawing board: a
    /// load publishes one event per entity it read, all of them after the command
    /// returned, and a flag simply set to `true` before them is overwritten.
    #[test]
    fn a_load_stays_clean_through_the_burst_it_causes() {
        let vm = vm();
        vm.is_open.set(true);
        vm.begin_settling_clean();
        assert!(vm.is_saved().get());
        let start = Instant::now();

        // Three frames of burst, all inside the window.
        for _ in 0..3 {
            vm.saw_edit.set(true);
            vm.settle_dirty_at(start);
            assert!(
                vm.is_saved().get(),
                "an event from the load itself must not mark the manifest dirty"
            );
        }

        // The burst has been seen, so the next quiet frame ends it.
        vm.settle_dirty_at(start);
        assert!(vm.is_saved().get());
        assert!(!vm.settling_clean.get(), "the settle should be over");
    }

    /// The bug this window exists for.
    ///
    /// A close returns, a frame goes by with the removal burst still in flight,
    /// and only then do the events arrive. With the settle ended by that first
    /// quiet frame, the burst read as user edits and the title bar offered to
    /// save a manifest that had just been closed.
    #[test]
    fn a_burst_that_arrives_late_still_does_not_dirty() {
        let vm = vm();
        vm.is_open.set(true);
        vm.begin_settling_clean();
        let start = Instant::now();

        // A quiet frame first: the events have not been delivered yet.
        vm.settle_dirty_at(start);

        // Now the burst lands.
        for _ in 0..3 {
            vm.saw_edit.set(true);
            vm.settle_dirty_at(start);
        }
        assert!(
            vm.is_saved().get(),
            "a burst that arrived after the first quiet frame is still the load's"
        );
    }

    /// An event with nothing open cannot be an edit, whatever the window says.
    #[test]
    fn an_event_with_no_manifest_open_is_never_an_edit() {
        let vm = vm();
        vm.is_open.set(false);
        // No settle in flight at all: this is the tail of a close whose window
        // has already closed out.
        vm.saw_edit.set(true);
        vm.settle_dirty_at(Instant::now());
        assert!(vm.is_saved().get());
    }

    /// Once the burst has drained, a real edit dirties as usual.
    #[test]
    fn an_edit_after_the_burst_marks_the_manifest_dirty() {
        let vm = vm();
        vm.is_open.set(true);
        vm.begin_settling_clean();
        let start = Instant::now();
        // Nothing ever arrived, so the grace period ends the settle.
        vm.settle_dirty_at(start + SETTLE_START_GRACE);
        assert!(!vm.settling_clean.get());

        vm.saw_edit.set(true);
        vm.settle_dirty_at(start + SETTLE_START_GRACE);
        assert!(!vm.is_saved().get());
    }

    /// A plain edit with no load in flight dirties on the very next frame.
    #[test]
    fn an_edit_with_no_load_in_flight_dirties_at_once() {
        let vm = vm();
        vm.is_open.set(true);
        assert!(vm.is_saved().get());
        vm.saw_edit.set(true);
        vm.settle_dirty();
        assert!(!vm.is_saved().get());
    }

    /// A frame with nothing in it changes nothing.
    #[test]
    fn a_quiet_frame_with_no_settle_pending_is_inert() {
        let vm = vm();
        // An edit is only an edit when there is something open to edit.
        vm.is_open.set(true);
        vm.saw_edit.set(true);
        vm.settle_dirty();
        assert!(!vm.is_saved().get());
        vm.settle_dirty();
        assert!(
            !vm.is_saved().get(),
            "a quiet frame must not clean a real edit"
        );
    }

    #[test]
    fn can_save_needs_open_dirty_and_idle() {
        let vm = vm();
        let can_save = vm.can_save();
        assert!(!can_save.get(), "nothing is open");

        vm.is_open.set(true);
        vm.is_saved.set(false);
        assert!(can_save.get());

        vm.busy.set(true);
        assert!(!can_save.get(), "a command is already running");
    }

    #[test]
    fn the_title_distinguishes_the_three_states() {
        let vm = vm();
        let title = vm.title();
        assert_eq!(title.get().resolve_now(), "Qleany");

        vm.is_open.set(true);
        assert_eq!(
            title.get().resolve_now(),
            "Qleany - new manifest without path"
        );

        vm.path.set("/tmp/qleany.yaml".to_string());
        assert_eq!(title.get().resolve_now(), "Qleany - /tmp/qleany.yaml");
    }

    /// Qleany's own manifest, which the repo is guaranteed to have. Read only:
    /// nothing below saves.
    fn qleany_manifest() -> &'static str {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../qleany.yaml")
    }

    /// A view-model with its context to hand, so a test can look at the store
    /// afterwards rather than only at the view-model's own signals.
    fn vm_with_context() -> (Rc<AppContext>, ManifestViewModel, AppIds) {
        let ctx = Rc::new(AppContext::new());
        handling_app_lifecycle_commands::initialize_app(&ctx).expect("initialize_app");
        let ids = AppIds::new();
        (ctx.clone(), ManifestViewModel::new(ctx, ids.clone()), ids)
    }

    /// Opening resolves the two rows the screens actually edit.
    ///
    /// The load event carries only the workspace id; `Global` and `UserInterface`
    /// hang off it, and a screen that had to find them itself would be the fourth
    /// place in the codebase that walks this tree.
    #[test]
    fn opening_resolves_the_rows_the_screens_edit() {
        let (_ctx, vm, ids) = vm_with_context();
        vm.open_path(qleany_manifest());

        assert!(
            vm.is_open().get(),
            "the manifest did not open: {:?}",
            vm.error.get()
        );
        assert!(ids.workspace_id.get().is_some());
        assert!(
            ids.global_id.get().is_some(),
            "Project settings would be empty"
        );
        assert!(
            ids.user_interface_id.get().is_some(),
            "the User Interface screen would be empty"
        );
    }

    /// One stack per screen that mutates the manifest, all distinct, all minted
    /// after the load rather than before it.
    #[test]
    fn opening_mints_one_undo_stack_per_screen() {
        let (_ctx, vm, ids) = vm_with_context();
        vm.open_path(qleany_manifest());

        let stacks: Vec<u64> = ids.stacks().iter().filter_map(|s| s.get()).collect();
        assert_eq!(stacks.len(), 4, "every mutating screen needs its own stack");
        let mut unique = stacks.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            4,
            "two screens are sharing a stack: {stacks:?}"
        );
    }

    /// Reopening does not leak stacks: the history belonged to the manifest that
    /// closed, and a stale stack would let Ctrl+Z replay one manifest's edits into
    /// the next.
    #[test]
    fn reopening_replaces_the_stacks_rather_than_adding_to_them() {
        let (_ctx, vm, ids) = vm_with_context();
        vm.open_path(qleany_manifest());
        let first: Vec<u64> = ids.stacks().iter().filter_map(|s| s.get()).collect();

        vm.open_path(qleany_manifest());
        let second: Vec<u64> = ids.stacks().iter().filter_map(|s| s.get()).collect();

        assert_eq!(second.len(), 4);
        for id in &second {
            assert!(!first.contains(id), "stack {id} survived a reopen");
        }
    }

    #[test]
    fn closing_gives_every_stack_back() {
        let (_ctx, vm, ids) = vm_with_context();
        vm.open_path(qleany_manifest());
        vm.close();

        for stack in ids.stacks() {
            assert_eq!(stack.get(), None);
        }
        assert_eq!(ids.global_id.get(), None);
        assert!(!vm.is_open().get());
    }
}
