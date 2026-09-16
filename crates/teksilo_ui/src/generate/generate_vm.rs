//! What the Generate screen knows.
//!
//! Three backend passes stand between opening the screen and a usable list:
//! enumerate the files the manifest implies, render each one's code, then compare
//! every rendering with what is on disk. The second is a long operation on a
//! background thread, so the screen has to show progress, survive being left, and
//! stop the work when it is.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use teksilo::data::ListModel;
use teksilo::prelude::*;

use frontend::AppContext;
use frontend::EntityId;
use frontend::commands::{
    file_generation_shared_steps_commands, long_operation_commands, rust_file_generation_commands,
};
use frontend::rust_file_generation::dtos::{FillRustFilesDto, GenerateRustFilesDto};

use crate::app_ids::AppIds;
use crate::generate::file_row::{ALL_GROUPS, FileRow, Filters, groups_of};
use crate::models::SystemFilesListModel;

/// How often a running long operation is polled.
///
/// The backend reports progress through events, but the `Completed` and `Failed`
/// variants are not published by every worker, so this is the backstop that keeps a
/// finished operation from leaving a modal on screen forever. Ten times a second
/// while something runs, and not at all otherwise: the wake is only requested while
/// an operation is in flight.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Where generation writes when "in temp" is on.
///
/// A prefix under the project root rather than a temporary directory of the OS: the
/// point is to look at what would be written next to what is there now.
const TEMP_PREFIX: &str = "temp";

/// Which pass the screen is running.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum Stage {
    /// Nothing is running.
    #[default]
    Idle,
    /// Working out what the manifest implies, and what each file would contain.
    Computing,
    /// Writing files.
    Generating,
}

/// A running long operation, and what it is for.
#[derive(Clone, Debug)]
struct Running {
    id: String,
    stage: Stage,
}

#[derive(Clone)]
pub struct GenerateViewModel {
    app_ctx: Rc<AppContext>,
    ids: AppIds,
    /// The generated model. It holds every file's rendered body, which is why the
    /// screen drops its owner on the way out.
    files: SystemFilesListModel,
    /// The lean projection the list actually reads.
    rows: ListModel<FileRow>,
    /// Every row, unfiltered, for the counts and the group list.
    all: Rc<RefCell<Vec<FileRow>>>,
    filters: Signal<Filters>,
    /// Bumped whenever the visible set changes, so anything derived can re-read.
    version: Signal<u64>,
    groups: Signal<Vec<String>>,
    selected_group: Signal<String>,
    /// One tick per file, by id, kept across rebuilds.
    ticks: Rc<RefCell<HashMap<EntityId, Signal<bool>>>>,
    selected_file: Signal<Option<EntityId>>,
    /// Show the difference against disk rather than the generated source.
    view_diff: Signal<bool>,
    /// Write under `temp/` rather than into the project.
    in_temp: Signal<bool>,
    stage: Signal<Stage>,
    progress: Signal<f32>,
    message: Signal<String>,
    status: Signal<Option<LocalizedString>>,
    running: Rc<RefCell<Option<Running>>>,
    /// Set while the screen is mounted, so leaving can tell the difference between
    /// "not started" and "left".
    entered: Rc<Cell<bool>>,
}

impl std::fmt::Debug for GenerateViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenerateViewModel")
            .field("stage", &self.stage.get())
            .field("rows", &self.rows.len())
            .finish_non_exhaustive()
    }
}

impl GenerateViewModel {
    pub fn new(app_ctx: Rc<AppContext>, ids: AppIds, files: SystemFilesListModel) -> Self {
        Self {
            app_ctx,
            ids,
            files,
            rows: ListModel::new(),
            all: Rc::new(RefCell::new(Vec::new())),
            filters: Signal::new(Filters::default()),
            version: Signal::new(0),
            groups: Signal::new(vec![ALL_GROUPS.to_string()]),
            selected_group: Signal::new(ALL_GROUPS.to_string()),
            ticks: Rc::new(RefCell::new(HashMap::new())),
            selected_file: Signal::new(None),
            // US-GEN-07: the difference is what a user came to see; the source is
            // there for when they want the whole file.
            view_diff: Signal::new(true),
            // US-GEN-09: on at every launch. Writing into the project is the
            // deliberate act, not the default one.
            in_temp: Signal::new(true),
            stage: Signal::new(Stage::Idle),
            progress: Signal::new(0.0),
            message: Signal::new(String::new()),
            status: Signal::new(None),
            running: Rc::new(RefCell::new(None)),
            entered: Rc::new(Cell::new(false)),
        }
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn rows(&self) -> ListModel<FileRow> {
        self.rows.clone()
    }

    pub fn version(&self) -> Signal<u64> {
        self.version.clone()
    }

    pub fn groups(&self) -> Signal<Vec<String>> {
        self.groups.clone()
    }

    pub fn selected_group(&self) -> Signal<String> {
        self.selected_group.clone()
    }

    pub fn filters(&self) -> Signal<Filters> {
        self.filters.clone()
    }

    pub fn view_diff(&self) -> Signal<bool> {
        self.view_diff.clone()
    }

    pub fn in_temp(&self) -> Signal<bool> {
        self.in_temp.clone()
    }

    pub fn stage(&self) -> Signal<Stage> {
        self.stage.clone()
    }

    pub fn busy(&self) -> Signal<bool> {
        self.stage.map(|s| *s != Stage::Idle)
    }

    pub fn progress(&self) -> Signal<f32> {
        self.progress.clone()
    }

    pub fn message(&self) -> Signal<String> {
        self.message.clone()
    }

    pub fn status(&self) -> Signal<Option<LocalizedString>> {
        self.status.clone()
    }

    pub fn selected_file(&self) -> Signal<Option<EntityId>> {
        self.selected_file.clone()
    }

    /// The tick of one file, made once and kept. See the entity association list,
    /// which has the same problem: a signal made in a delegate is a different signal
    /// every time the row is realized.
    pub fn tick(&self, file: EntityId) -> Signal<bool> {
        let mut ticks = self.ticks.borrow_mut();
        ticks
            .entry(file)
            .or_insert_with(|| Signal::new(false))
            .clone()
    }

    /// How many visible files are ticked.
    ///
    /// Visible, not every file: a filter is a statement about what the user is
    /// working on, and "Generate (412)" after filtering down to four would be a
    /// promise to write something they cannot see.
    pub fn selected_count(&self) -> usize {
        let ticks = self.ticks.borrow();
        (0..self.rows.len())
            .filter_map(|i| self.rows.with_item(i, |row| row.id))
            .filter(|id| ticks.get(id).map(|t| t.get()).unwrap_or(false))
            .count()
    }

    /// The generated body of one file, straight from the model.
    pub fn code_of(&self, file: EntityId) -> Option<String> {
        self.files
            .rows()
            .into_iter()
            .find(|row| row.id == file)
            .and_then(|row| row.generated_code)
    }

    /// The difference between what would be written and what is on disk.
    pub fn diff_of(&self, file: EntityId) -> Result<String, String> {
        file_generation_shared_steps_commands::get_file_diff(
            &self.app_ctx,
            &frontend::file_generation_shared_steps::dtos::GetDiffDto { file_id: file },
        )
        .map(|dto| dto.diff_text)
        .map_err(|e| e.to_string())
    }

    // ── commands ─────────────────────────────────────────────────────────────

    pub fn set_group(&self, group: &str) {
        self.selected_group.set_if_changed(group.to_string());
        let mut filters = self.filters.get();
        filters.group = Some(group.to_string());
        self.filters.set_if_changed(filters);
        self.reproject();
    }

    pub fn set_text_filter(&self, text: &str) {
        let mut filters = self.filters.get();
        filters.text = text.to_string();
        self.filters.set_if_changed(filters);
        self.reproject();
    }

    /// Flip one of the six status or nature filters.
    pub fn set_filter(&self, which: FilterKind, on: bool) {
        let mut filters = self.filters.get();
        match which {
            FilterKind::Modified => filters.modified = on,
            FilterKind::New => filters.new = on,
            FilterKind::Unchanged => filters.unchanged = on,
            FilterKind::Infrastructure => filters.infrastructure = on,
            FilterKind::Aggregate => filters.aggregate = on,
            FilterKind::Scaffold => filters.scaffold = on,
        }
        self.filters.set_if_changed(filters);
        self.reproject();
    }

    /// Tick or untick every visible file, leaving the hidden ones alone.
    pub fn set_all_visible(&self, ticked: bool) {
        for index in 0..self.rows.len() {
            if let Some(id) = self.rows.with_item(index, |row| row.id) {
                self.tick(id).set_if_changed(ticked);
            }
        }
        self.bump();
    }

    /// A tick changed. The button's count is derived from the ticks, and a tick is
    /// its own signal rather than part of the row, so nothing else would notice.
    pub fn tick_changed(&self) {
        self.bump();
    }

    pub fn select_file(&self, file: EntityId) {
        self.selected_file.set_if_changed(Some(file));
    }

    pub fn set_view_diff(&self, on: bool) {
        self.view_diff.set_if_changed(on);
    }

    pub fn set_in_temp(&self, on: bool) {
        self.in_temp.set_if_changed(on);
    }

    /// The screen was opened: point the file list at `System` and start the pipeline.
    pub fn enter(&self) {
        if self.entered.replace(true) {
            return;
        }
        self.files.set_owner_id(self.ids.system_id.get());
        self.start_computing();
    }

    /// The screen was left.
    ///
    /// Two things happen, and both matter. The long operation is cancelled, which
    /// the Slint UI never did: its Cancel button set a flag the worker did not read,
    /// so leaving the screen left a thread rendering every file in the manifest.
    /// And the file list is unpointed, which releases the rendered bodies: they are
    /// the largest thing this app holds, and holding them for a screen nobody is
    /// looking at is what makes a big manifest feel heavy everywhere else.
    pub fn leave(&self) {
        if !self.entered.replace(false) {
            return;
        }
        self.cancel();
        self.files.set_owner_id(None);
        self.rows.clear();
        self.all.borrow_mut().clear();
        self.ticks.borrow_mut().clear();
        self.selected_file.set_if_changed(None);
        self.bump();
    }

    /// Stop whatever is running.
    pub fn cancel(&self) {
        let running = self.running.borrow_mut().take();
        if let Some(running) = running {
            // The real command, not a flag: this is what `setup_cancel_generate_callback`
            // in the Slint UI never called, which is why its Cancel button did nothing.
            long_operation_commands::cancel_operation(&self.app_ctx, &running.id);
        }
        self.stage.set_if_changed(Stage::Idle);
        self.progress.set_if_changed(0.0);
        self.message.set_if_changed(String::new());
    }

    /// Re-run the whole pipeline.
    pub fn refresh(&self) {
        self.cancel();
        self.start_computing();
    }

    /// Write the ticked files.
    pub fn generate(&self) {
        if self.stage.get() != Stage::Idle {
            return;
        }
        let file_ids = self.selected_ids();
        if file_ids.is_empty() {
            return;
        }

        let prefix = if self.in_temp.get() {
            TEMP_PREFIX.to_string()
        } else {
            String::new()
        };
        let dto = GenerateRustFilesDto {
            file_ids,
            root_path: ".".to_string(),
            prefix,
        };
        match rust_file_generation_commands::generate_rust_files(&self.app_ctx, &dto) {
            Ok(id) => self.begin(id, Stage::Generating, tr!(generate_step_writing())),
            Err(e) => self.fail(&e.to_string()),
        }
    }

    /// The ticked files, in list order.
    pub fn selected_ids(&self) -> Vec<EntityId> {
        let ticks = self.ticks.borrow();
        (0..self.rows.len())
            .filter_map(|i| self.rows.with_item(i, |row| row.id))
            .filter(|id| ticks.get(id).map(|t| t.get()).unwrap_or(false))
            .collect()
    }

    // ── the pipeline ─────────────────────────────────────────────────────────

    /// Enumerate the files, then start rendering them.
    fn start_computing(&self) {
        if let Err(e) = rust_file_generation_commands::fill_rust_files(
            &self.app_ctx,
            &FillRustFilesDto {
                only_list_already_existing: false,
            },
        ) {
            self.fail(&e.to_string());
            return;
        }
        // The list is usable already: every file is there, with its status still
        // unknown, which is why an unknown status is never filtered out.
        self.reproject();

        match rust_file_generation_commands::fill_code_in_rust_files(&self.app_ctx) {
            Ok(id) => self.begin(id, Stage::Computing, tr!(generate_step_rendering())),
            Err(e) => self.fail(&e.to_string()),
        }
    }

    fn begin(&self, id: String, stage: Stage, message: LocalizedString) {
        *self.running.borrow_mut() = Some(Running {
            id,
            stage: stage.clone(),
        });
        self.stage.set_if_changed(stage);
        self.progress.set_if_changed(0.0);
        self.message.set_if_changed(message.resolve_now());
        self.status.set(None);
    }

    /// One poll of the running operation. Returns whether anything is still running.
    fn poll(&self) -> bool {
        let Some(running) = self.running.borrow().clone() else {
            return false;
        };

        if let Some(progress) =
            long_operation_commands::get_operation_progress(&self.app_ctx, &running.id)
        {
            self.progress.set_if_changed(progress.percentage / 100.0);
            if let Some(message) = progress.message.clone()
                && !message.is_empty()
            {
                self.message.set_if_changed(message);
            }
        }

        let finished = long_operation_commands::is_operation_finished(&self.app_ctx, &running.id)
            // An operation the manager has forgotten is finished as far as this
            // screen is concerned: waiting on it would hang the modal forever.
            .unwrap_or(true);
        if !finished {
            return true;
        }

        self.running.borrow_mut().take();
        match running.stage {
            Stage::Computing => self.finish_computing(),
            Stage::Generating => self.finish_generating(&running.id),
            Stage::Idle => {}
        }
        self.stage.set_if_changed(Stage::Idle);
        false
    }

    /// The rendering pass is done: compare every rendering with the disk.
    fn finish_computing(&self) {
        if let Err(e) = file_generation_shared_steps_commands::fill_status_in_files(&self.app_ctx) {
            self.fail(&e.to_string());
            return;
        }
        self.reproject();
    }

    fn finish_generating(&self, id: &str) {
        // The count comes from the result rather than from what was ticked: a
        // cancelled run writes some of them, and the backend offers no rollback, so
        // saying how many were written is the only honest report.
        let written = long_operation_commands::get_operation_result(&self.app_ctx, id)
            .map(|json| count_written(&json))
            .unwrap_or(0);
        self.status
            .set(Some(tr!(status_generated_files(count = written as i64))));
        // What is on disk has changed, so every file's status has.
        self.refresh_statuses();
    }

    fn refresh_statuses(&self) {
        if let Err(e) = file_generation_shared_steps_commands::fill_status_in_files(&self.app_ctx) {
            log::error!("could not re-read the file statuses: {e}");
            return;
        }
        self.reproject();
    }

    /// Re-read the generated model and rebuild the lean projection.
    fn reproject(&self) {
        self.files.refresh();
        let all: Vec<FileRow> = self.files.rows().iter().map(FileRow::of).collect();
        self.groups.set_if_changed(groups_of(&all));

        let filters = self.filters.get();
        let visible: Vec<FileRow> = all.iter().filter(|r| filters.shows(r)).cloned().collect();
        // Keyed reconciliation rather than a replace: the selection and the scroll
        // position survive a status pass that changed nothing the user can see.
        self.rows.reconcile_by_key(visible, |row| row.id);
        *self.all.borrow_mut() = all;
        self.bump();
    }

    fn bump(&self) {
        let next = self.version.get().wrapping_add(1);
        self.version.set(next);
    }

    fn fail(&self, message: &str) {
        log::error!("generation failed: {message}");
        self.status
            .set(Some(tr!(status_error(message = message.to_string()))));
        self.stage.set_if_changed(Stage::Idle);
        self.running.borrow_mut().take();
    }

    // ── wiring ───────────────────────────────────────────────────────────────

    /// Install the poll. Called from `build`, on every build.
    ///
    /// The wake is only asked for while something is running, so an idle Generate
    /// screen costs nothing: teksilo pumps a frame when something asks for one.
    pub fn wire(&self, ctx: &mut BuildContext) {
        let wake = ctx.wake_at_handle();
        let tick = ctx.frame_tick();
        let me = self.clone();
        ctx.effect(&tick, move |_| {
            if me.poll() {
                wake.set(Some(Instant::now() + POLL_INTERVAL));
            }
        });
    }
}

/// Which of the six checkbox filters.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterKind {
    Modified,
    New,
    Unchanged,
    Infrastructure,
    Aggregate,
    Scaffold,
}

/// How many files a generation run reported writing.
///
/// The long-operation manager hands results back as a JSON string, which is its
/// interface rather than a choice here. Counting the entries of the `files` array
/// is all this screen needs, and a whole deserialisation type for one number would
/// have to be kept in step with a DTO that is already generated elsewhere.
///
/// A malformed or absent result counts as nothing written, which is the safe way to
/// be wrong: it under-reports rather than claiming files that are not there.
fn count_written(result: &str) -> usize {
    let Some(start) = result.find("\"files\"") else {
        return 0;
    };
    let Some(open) = result[start..].find('[') else {
        return 0;
    };
    let rest = &result[start + open + 1..];
    let Some(close) = rest.find(']') else {
        return 0;
    };
    let body = rest[..close].trim();
    if body.is_empty() {
        return 0;
    }
    body.matches('"').count() / 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontend::common::entities::{FileNature, FileStatus};

    fn vm() -> GenerateViewModel {
        let ctx = Rc::new(AppContext::new());
        let files = SystemFilesListModel::new(ctx.clone(), Signal::new(None));
        GenerateViewModel::new(ctx, AppIds::new(), files)
    }

    fn row(id: EntityId, name: &str) -> FileRow {
        FileRow {
            id,
            name: name.to_string(),
            relative_path: "crates/".to_string(),
            group: "common".to_string(),
            status: FileStatus::Modified,
            nature: FileNature::Infrastructure,
        }
    }

    /// US-GEN-09 and US-GEN-07: the two settings every launch starts from.
    #[test]
    fn the_launch_defaults_are_the_cautious_ones() {
        let vm = vm();
        assert!(vm.in_temp().get(), "writing into the project is deliberate");
        assert!(
            vm.view_diff().get(),
            "the difference is what a user came to see"
        );
        assert_eq!(vm.stage().get(), Stage::Idle);
    }

    /// US-GEN-05: wholesale selection applies to the visible rows and no others.
    #[test]
    fn select_all_leaves_hidden_rows_alone() {
        let vm = vm();
        vm.rows.replace_all(vec![row(1, "a.rs"), row(2, "b.rs")]);
        // A row that exists but is filtered out: it has a tick, and nothing here
        // should touch it.
        let hidden = vm.tick(99);
        hidden.set(false);

        vm.set_all_visible(true);
        assert!(vm.tick(1).get());
        assert!(vm.tick(2).get());
        assert!(!hidden.get(), "a hidden row keeps its tick");

        vm.set_all_visible(false);
        assert!(!vm.tick(1).get());
    }

    /// US-GEN-06: the count is of visible ticked files.
    #[test]
    fn the_count_is_of_what_is_on_screen() {
        let vm = vm();
        vm.rows.replace_all(vec![row(1, "a.rs"), row(2, "b.rs")]);
        vm.tick(1).set(true);
        vm.tick(99).set(true);

        assert_eq!(vm.selected_count(), 1, "the hidden row does not count");
        assert_eq!(vm.selected_ids(), vec![1]);
    }

    /// A tick is made once and kept, or a row scrolling back into view would forget
    /// it.
    #[test]
    fn a_tick_is_the_same_signal_every_time() {
        let vm = vm();
        let first = vm.tick(7);
        first.set(true);
        assert!(vm.tick(7).get());
    }

    /// US-GEN-11: leaving releases the rendered bodies and forgets the ticks.
    #[test]
    fn leaving_releases_everything_it_was_holding() {
        let vm = vm();
        vm.entered.set(true);
        vm.rows.replace_all(vec![row(1, "a.rs")]);
        vm.tick(1).set(true);
        vm.selected_file.set(Some(1));

        vm.leave();
        assert_eq!(vm.rows.len(), 0);
        assert!(vm.ticks.borrow().is_empty());
        assert_eq!(vm.selected_file().get(), None);
        assert_eq!(vm.stage().get(), Stage::Idle);
    }

    /// Leaving a screen that was never entered does nothing, so an effect that fires
    /// on every screen change cannot cancel work that belongs to another screen.
    #[test]
    fn leaving_without_entering_is_inert() {
        let vm = vm();
        vm.rows.replace_all(vec![row(1, "a.rs")]);
        vm.leave();
        assert_eq!(vm.rows.len(), 1, "nothing was entered, so nothing is left");
    }

    /// The result is JSON from the long-operation manager, and a run that reports
    /// nothing must not be reported as a success over files that were never written.
    #[test]
    fn a_generation_result_is_counted_from_its_file_list() {
        assert_eq!(
            count_written(r#"{"files":["a.rs","b.rs"],"duration":"1s"}"#),
            2
        );
        assert_eq!(count_written(r#"{"files":[]}"#), 0);
        assert_eq!(count_written("not json at all"), 0);
        assert_eq!(count_written(""), 0);
    }

    #[test]
    fn a_filter_change_reaches_the_filters() {
        let vm = vm();
        vm.set_filter(FilterKind::Unchanged, true);
        assert!(vm.filters().get().unchanged);

        vm.set_text_filter("repo");
        assert_eq!(vm.filters().get().text, "repo");

        vm.set_group("cli");
        assert_eq!(vm.filters().get().group.as_deref(), Some("cli"));
        assert_eq!(vm.selected_group().get(), "cli");
    }
}
