//! What the demo generator knows.
//!
//! The demo is the one thing in this application that runs a whole pipeline:
//! create a manifest, load it, validate it, enumerate the files it implies, render
//! every one of them, compare them with the disk, and write them out. The Slint UI
//! ran that pipeline on a thread of its own and posted progress back through the
//! event loop, which it could do because Slint's context is `Send`. Teksilo's is
//! not, and `AppContext` is an `Rc`, so the pipeline here is a state machine driven
//! one step per frame instead.
//!
//! That is not a workaround. One step per frame is what keeps the progress bar
//! honest: the bar moves because a step finished, not because a timer fired, and
//! the one genuinely long step, rendering every file, is a backend long operation
//! on its own thread that this polls.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::Duration;

use teksilo::prelude::*;

use frontend::AppContext;
use frontend::EntityId;
use frontend::commands::{
    cpp_qt_file_generation_commands, file_commands, file_generation_shared_steps_commands,
    handling_manifest_commands, long_operation_commands, rust_file_generation_commands,
    system_commands,
};
use frontend::common::direct_access::system::SystemRelationshipField;
use frontend::cpp_qt_file_generation::dtos::FillCppQtFilesDto;
use frontend::handling_manifest::dtos::{CreateDto, ManifestTemplate};
use frontend::rust_file_generation::dtos::FillRustFilesDto;

use crate::bootstrap;
use crate::manifest::ManifestViewModel;
use crate::new_manifest::new_manifest_vm::{create_language, option_name};
use crate::project::Language;
use crate::user_interface::Target;

/// What the generated application is called. Fixed: the demo is a sample, and
/// asking a user to name something they are about to delete is a question with no
/// good answer.
const APPLICATION_NAME: &str = "Demo";

/// Who the generated application says it is by.
const ORGANISATION_NAME: &str = "FernTech";

/// The folder Browse appends to whatever parent the user picked.
const FOLDER_NAME: &str = "qleany-demo";

/// What the destination field starts as, tilde and all.
///
/// The literal tilde rather than the expanded path: it is shorter, it is what a
/// user would type, and [`expand_tilde`] puts it back before anything touches the
/// filesystem.
pub const DEFAULT_DESTINATION: &str = "~/qleany-demo";

/// How many files are written per frame.
///
/// Writing a few hundred small files takes well under a second in total, but doing
/// it in one step would freeze the progress bar for all of it. Sixteen at a time
/// keeps the bar moving without making the writing itself measurably slower.
const WRITE_CHUNK: usize = 16;

/// How long to wait before polling a running long operation again.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Which face the dialog is showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Phase {
    /// Pick a language and a destination.
    #[default]
    Form,
    /// The pipeline is running, and nothing can interrupt it.
    Running,
    /// It worked. Here is what you got.
    Done,
}

/// One step of the pipeline.
///
/// The order is the order they run in, and [`Step::progress`] is monotonic across
/// it, so a bar driven from this can only ever move forwards.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    /// Make the destination folder.
    CreateFolder,
    /// Write a fresh `qleany.yaml` into it.
    CreateManifest,
    /// Read it back into the store, which is what every later step reads.
    LoadManifest,
    /// Refuse to generate from a manifest that does not validate.
    RunChecks,
    /// Work out which files the manifest implies.
    FillFiles,
    /// Render every one of them. A backend long operation.
    GenerateCode,
    /// Compare each rendering with what is on disk.
    CompareWithDisk,
    /// Write them out.
    WriteFiles,
    /// C++/Qt only: a repository and a tag, which its build needs.
    InitGit,
}

impl Step {
    /// The step this one hands over to, or `None` when the run is finished.
    ///
    /// `InitGit` is skipped for Rust rather than being a no-op step, so the bar
    /// does not pause on a step that does nothing.
    fn next(self, language: Language) -> Option<Step> {
        let next = match self {
            Step::CreateFolder => Step::CreateManifest,
            Step::CreateManifest => Step::LoadManifest,
            Step::LoadManifest => Step::RunChecks,
            Step::RunChecks => Step::FillFiles,
            Step::FillFiles => Step::GenerateCode,
            Step::GenerateCode => Step::CompareWithDisk,
            Step::CompareWithDisk => Step::WriteFiles,
            Step::WriteFiles => Step::InitGit,
            Step::InitGit => return None,
        };
        if next == Step::InitGit && language != Language::CppQt {
            return None;
        }
        Some(next)
    }

    /// Where the bar stands when this step starts, from 0 to 1.
    pub fn progress(self) -> f32 {
        match self {
            Step::CreateFolder => 0.05,
            Step::CreateManifest => 0.10,
            Step::LoadManifest => 0.15,
            Step::RunChecks => 0.20,
            Step::FillFiles => 0.25,
            Step::GenerateCode => 0.30,
            Step::CompareWithDisk => 0.85,
            Step::WriteFiles => 0.90,
            Step::InitGit => 0.99,
        }
    }

    /// Where the bar stands when this step is over.
    ///
    /// Only the two steps that report progress from the inside need this:
    /// everything else is a single jump to the next step's start.
    fn progress_end(self) -> f32 {
        match self {
            Step::GenerateCode => 0.80,
            Step::WriteFiles => 0.99,
            other => other.progress(),
        }
    }

    /// How long to wait before asking an unfinished step to carry on.
    ///
    /// Only the rendering step is worth waiting on: it is watching a thread, and
    /// asking ten times a second is plenty. The writing step comes straight back,
    /// because its turns are chunks of work rather than polls, and a hundred
    /// milliseconds between them would add seconds of doing nothing to a run.
    fn retry_delay(self) -> Duration {
        match self {
            Step::GenerateCode => POLL_INTERVAL,
            _ => Duration::ZERO,
        }
    }

    /// What the dialog says it is doing.
    pub fn message(self) -> LocalizedString {
        match self {
            Step::CreateFolder => tr!(demo_step_folder()),
            Step::CreateManifest => tr!(demo_step_manifest()),
            Step::LoadManifest => tr!(demo_step_loading()),
            Step::RunChecks => tr!(demo_step_checks()),
            Step::FillFiles => tr!(demo_step_listing()),
            Step::GenerateCode => tr!(demo_step_rendering()),
            Step::CompareWithDisk => tr!(demo_step_comparing()),
            Step::WriteFiles => tr!(demo_step_writing()),
            Step::InitGit => tr!(demo_step_git()),
        }
    }
}

/// The frontends the demo generates, in the order its summary names them.
///
/// Two per language, and the summary sentence is built from this same list, so the
/// bullet cannot claim a frontend the manifest did not ask for. The Slint UI and
/// the CLI both hard-code that sentence separately, and both still say "CLI and
/// Slint" for a demo that has generated CLI and Teksilo since the Teksilo templates
/// landed.
pub fn demo_targets(language: Language) -> [Target; 2] {
    match language {
        Language::Rust => [Target::RustCli, Target::RustTeksilo],
        Language::CppQt => [Target::CppQtQuick, Target::CppQtWidgets],
    }
}

/// A frontend's plain name, for the summary sentence.
///
/// Not [`Target::label`]: that is what the User Interface screen's checkboxes say,
/// and one of them carries a "(recommended)" that has no place in a sentence
/// listing what was generated.
fn target_name(target: Target) -> LocalizedString {
    match target {
        Target::RustCli => tr!(ui_target_rust_cli()),
        Target::RustTeksilo => tr!(demo_ui_teksilo()),
        Target::RustSlint => tr!(ui_target_rust_slint()),
        Target::RustIos => tr!(ui_target_rust_ios()),
        Target::RustAndroid => tr!(ui_target_rust_android()),
        Target::CppQtWidgets => tr!(ui_target_cpp_qt_widgets()),
        Target::CppQtQuick => tr!(ui_target_cpp_qt_quick()),
    }
}

/// The last bullet of the summary: which frontends came with the project.
pub fn ui_line(language: Language) -> LocalizedString {
    let [first, second] = demo_targets(language);
    tr!(demo_includes_ui(
        first = target_name(first).resolve_now(),
        second = target_name(second).resolve_now()
    ))
}

/// The command a user runs next, in the destination they chose.
pub fn next_command(language: Language, path: &Path) -> String {
    let path = path.display();
    match language {
        Language::Rust => format!("cd {path} && cargo run --bin demo"),
        Language::CppQt => format!(
            "cd {path} && mkdir build && cd build && cmake .. && cmake --build . --target all -j$(nproc)"
        ),
    }
}

/// Expand a leading `~` or `~/` to the user's home directory.
///
/// `std::env::home_dir` rather than the `dirs` crate the Slint UI carries: it was
/// un-deprecated and fixed in Rust 1.87, and this crate's floor is 1.90.
pub fn expand_tilde(path: &str) -> PathBuf {
    let home = || std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
    if path == "~" {
        home()
    } else if let Some(rest) = path.strip_prefix("~/") {
        home().join(rest)
    } else {
        PathBuf::from(path)
    }
}

/// What the destination field means, once.
///
/// An empty field is the default rather than an error: a user who cleared it has
/// not asked for anything in particular, and the placeholder already says where it
/// would go.
pub fn resolve_destination(typed: &str) -> PathBuf {
    let typed = typed.trim();
    if typed.is_empty() {
        expand_tilde(DEFAULT_DESTINATION)
    } else {
        expand_tilde(typed)
    }
}

/// Where Browse leaves the field: the picked parent, with the demo folder under it.
///
/// US-DEMO-02. The user picks where the project goes, not what it is called, which
/// is why picking `/home/me/code` twice cannot produce two projects on top of each
/// other.
pub fn browsed_destination(parent: &Path) -> PathBuf {
    parent.join(FOLDER_NAME)
}

/// A reason the run never started.
///
/// Each one is checked before anything is written. The Slint UI checked git only
/// after rendering and writing every file, so a missing `user.email` cost several
/// hundred files on disk and a failure at 95 per cent.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Blocked {
    /// Something is already there. Generating over it would merge two projects.
    ManifestExists(String),
    /// C++/Qt needs a repository, and there is no git.
    NoGit,
    /// C++/Qt needs a commit, and git will refuse to make one.
    NoGitIdentity,
}

impl Blocked {
    pub fn message(&self) -> LocalizedString {
        match self {
            Blocked::ManifestExists(path) => tr!(demo_error_exists(path = path.clone())),
            Blocked::NoGit => tr!(demo_error_no_git()),
            Blocked::NoGitIdentity => tr!(demo_error_no_git_identity()),
        }
    }
}

/// What a running pipeline is holding.
struct Run {
    language: Language,
    path: PathBuf,
    step: Step,
    /// The files still to write. Ids rather than rows: a row carries the whole
    /// rendered body, and holding every body at once is the single largest thing
    /// this application could be asked to keep.
    pending: Vec<EntityId>,
    /// How far into `pending` the writing has got.
    cursor: usize,
    written: usize,
    manifest_lines: usize,
    /// The backend long operation, while one is in flight.
    operation: Option<String>,
}

#[derive(Clone)]
pub struct DemoViewModel {
    app_ctx: Rc<AppContext>,
    /// The demo loads its own manifest into the same store the rest of the app
    /// reads, so it goes through the manifest view-model rather than around it: the
    /// title bar, the rail and every screen then describe what is actually loaded.
    manifest: ManifestViewModel,
    /// Which language, by index into [`Language::ALL`], because a radio group
    /// selects by index.
    language_index: Signal<usize>,
    destination: Signal<String>,
    phase: Signal<Phase>,
    progress: Signal<f32>,
    message: Signal<String>,
    error: Signal<Option<LocalizedString>>,
    /// Where it ended up, for Open folder and for the next-step command.
    result_path: Signal<String>,
    written: Signal<usize>,
    manifest_lines: Signal<usize>,
    /// The copy button's glyph has confirmed the copy.
    copied: Signal<bool>,
    run: Rc<RefCell<Option<Run>>>,
}

impl std::fmt::Debug for DemoViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DemoViewModel")
            .field("phase", &self.phase.get())
            .field("progress", &self.progress.get())
            .finish_non_exhaustive()
    }
}

impl DemoViewModel {
    pub fn new(app_ctx: Rc<AppContext>, manifest: ManifestViewModel) -> Self {
        Self {
            app_ctx,
            manifest,
            language_index: Signal::new(0),
            destination: Signal::new(DEFAULT_DESTINATION.to_string()),
            phase: Signal::new(Phase::Form),
            progress: Signal::new(0.0),
            message: Signal::new(tr!(demo_step_preparing()).resolve_now()),
            error: Signal::new(None),
            result_path: Signal::new(String::new()),
            written: Signal::new(0),
            manifest_lines: Signal::new(0),
            copied: Signal::new(false),
            run: Rc::new(RefCell::new(None)),
        }
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn language_index(&self) -> Signal<usize> {
        self.language_index.clone()
    }

    pub fn language(&self) -> Language {
        Language::ALL
            .get(self.language_index.get())
            .copied()
            .unwrap_or_default()
    }

    pub fn destination(&self) -> Signal<String> {
        self.destination.clone()
    }

    pub fn phase(&self) -> Signal<Phase> {
        self.phase.clone()
    }

    pub fn progress(&self) -> Signal<f32> {
        self.progress.clone()
    }

    pub fn message(&self) -> Signal<String> {
        self.message.clone()
    }

    pub fn error(&self) -> Signal<Option<LocalizedString>> {
        self.error.clone()
    }

    pub fn result_path(&self) -> Signal<String> {
        self.result_path.clone()
    }

    pub fn copied(&self) -> Signal<bool> {
        self.copied.clone()
    }

    /// "Generated N files from a manifest of M lines."
    pub fn summary_stats(&self) -> LocalizedString {
        tr!(demo_summary_stats(
            files = self.written.get() as i64,
            lines = self.manifest_lines.get() as i64
        ))
    }

    /// The command the summary offers to copy.
    pub fn next_command(&self) -> String {
        next_command(self.language(), Path::new(&self.result_path.get()))
    }

    // ── commands ─────────────────────────────────────────────────────────────

    /// Put the dialog back the way it opens.
    ///
    /// Called when the dialog is opened rather than when it is closed: a run that
    /// finished should still be readable if the modal is somehow rebuilt, and the
    /// state that matters is what the *next* run starts from.
    pub fn reset(&self) {
        if self.phase.get() == Phase::Running {
            return;
        }
        self.phase.set_if_changed(Phase::Form);
        self.progress.set_if_changed(0.0);
        self.message.set(tr!(demo_step_preparing()).resolve_now());
        self.error.set(None);
        self.copied.set_if_changed(false);
        self.destination
            .set_if_changed(DEFAULT_DESTINATION.to_string());
    }

    /// Ask for a parent folder, and put the demo folder under it.
    pub fn browse(&self, ctx: &mut EventContext) {
        let destination = self.destination.clone();
        let request = FileDialogRequest::pick_folder()
            .title("Where should the demo project go?")
            .starting_dir(expand_tilde("~"));
        let _ = ctx.pick_folder(request, move |result, _ectx| {
            if let FileDialogResult::Folder(Some(parent)) = result {
                destination.set(browsed_destination(&parent).to_string_lossy().to_string());
            }
        });
    }

    /// Everything that has to be true before a single file is written.
    pub fn preflight(&self, language: Language, path: &Path) -> Result<(), Blocked> {
        let manifest = path.join("qleany.yaml");
        if manifest.exists() {
            return Err(Blocked::ManifestExists(path.display().to_string()));
        }
        if language == Language::CppQt {
            git_preflight()?;
        }
        Ok(())
    }

    /// Start the pipeline.
    pub fn start(&self) {
        if self.phase.get() == Phase::Running {
            return;
        }
        let language = self.language();
        let path = resolve_destination(&self.destination.get());

        if let Err(blocked) = self.preflight(language, &path) {
            self.error.set(Some(blocked.message()));
            return;
        }

        self.error.set(None);
        self.copied.set_if_changed(false);
        self.written.set_if_changed(0);
        self.manifest_lines.set_if_changed(0);
        self.result_path.set(path.to_string_lossy().to_string());
        self.progress.set(0.0);
        self.message.set(tr!(demo_step_preparing()).resolve_now());
        self.phase.set(Phase::Running);
        *self.run.borrow_mut() = Some(Run {
            language,
            path,
            step: Step::CreateFolder,
            pending: Vec::new(),
            cursor: 0,
            written: 0,
            manifest_lines: 0,
            operation: None,
        });
    }

    /// Put the next-step command on the clipboard, and say so.
    pub fn copy_next_command(&self, ctx: &mut EventContext) {
        let command = self.next_command();
        let Some(clipboard) = ctx.app_state::<teksilo::platform::clipboard::ClipboardHandle>()
        else {
            log::error!("no clipboard is installed; the command was not copied");
            return;
        };
        if let Err(e) = clipboard.set_text(&command) {
            log::error!("could not write to the clipboard: {e}");
            return;
        }
        self.copied.set_if_changed(true);
    }

    /// Show the generated project in the desktop's file manager.
    pub fn open_result_folder(&self) {
        let path = self.result_path.get();
        if path.is_empty() {
            return;
        }
        // `that_detached`, not `that`: the latter waits on the file manager it
        // spawned, which would freeze the UI thread for as long as one takes to
        // start.
        if let Err(e) = open::that_detached(&path) {
            log::warn!("could not open {path}: {e}");
        }
    }

    // ── the pipeline ─────────────────────────────────────────────────────────

    /// Advance the run by one step. Returns how long to wait before the next call,
    /// or `None` when nothing is running.
    pub fn tick(&self) -> Option<Duration> {
        let (step, language) = self
            .run
            .borrow()
            .as_ref()
            .map(|run| (run.step, run.language))?;

        self.progress.set(self.step_progress(step));
        self.message.set(step.message().resolve_now());

        match self.run_step(step) {
            Err(message) => {
                self.fail(message);
                None
            }
            // Still inside a step that reports its own progress.
            Ok(false) => Some(step.retry_delay()),
            Ok(true) => {
                self.progress.set(step.progress_end());
                match step.next(language) {
                    Some(next) => {
                        if let Some(run) = self.run.borrow_mut().as_mut() {
                            run.step = next;
                        }
                        // The next step runs on the next frame rather than in this
                        // one, so the message the user just read is actually
                        // painted before it is replaced.
                        Some(Duration::ZERO)
                    }
                    None => {
                        self.succeed();
                        None
                    }
                }
            }
        }
    }

    /// Where the bar stands right now, which for the writing step is somewhere
    /// inside the step rather than at its start.
    fn step_progress(&self, step: Step) -> f32 {
        let run = self.run.borrow();
        let Some(run) = run.as_ref() else {
            return step.progress();
        };
        match step {
            Step::WriteFiles if !run.pending.is_empty() => {
                let done = run.cursor as f32 / run.pending.len() as f32;
                lerp(step.progress(), step.progress_end(), done)
            }
            // The rendering step writes its own progress as the backend reports it,
            // so re-reading the signal is what keeps it rather than resetting it.
            _ => self.progress.get().max(step.progress()),
        }
    }

    /// Run one step. `Ok(true)` means it is finished, `Ok(false)` that it needs
    /// another turn.
    fn run_step(&self, step: Step) -> Result<bool, String> {
        match step {
            Step::CreateFolder => self.create_folder(),
            Step::CreateManifest => self.create_manifest(),
            Step::LoadManifest => self.load_manifest(),
            Step::RunChecks => self.run_checks(),
            Step::FillFiles => self.fill_files(),
            Step::GenerateCode => self.render_code(),
            Step::CompareWithDisk => self.compare_with_disk(),
            Step::WriteFiles => self.write_chunk(),
            Step::InitGit => self.init_git(),
        }
    }

    fn create_folder(&self) -> Result<bool, String> {
        let path = self.path().ok_or("the run is gone")?;
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        Ok(true)
    }

    fn create_manifest(&self) -> Result<bool, String> {
        let (language, path) = {
            let run = self.run.borrow();
            let run = run.as_ref().ok_or("the run is gone")?;
            (run.language, run.path.clone())
        };
        let manifest_path = path.join("qleany.yaml");
        let options = demo_targets(language)
            .into_iter()
            .map(|t| option_name(t).to_string())
            .collect();
        let dto = CreateDto {
            manifest_path: manifest_path.to_string_lossy().to_string(),
            language: create_language(language),
            application_name: APPLICATION_NAME.to_string(),
            organization_name: ORGANISATION_NAME.to_string(),
            manifest_template: ManifestTemplate::DataManagement,
            options,
        };
        handling_manifest_commands::create(&self.app_ctx, &dto).map_err(|e| e.to_string())?;

        let lines = std::fs::read_to_string(&manifest_path)
            .map_err(|e| e.to_string())?
            .lines()
            .count();
        if let Some(run) = self.run.borrow_mut().as_mut() {
            run.manifest_lines = lines;
        }
        self.manifest_lines.set_if_changed(lines);
        Ok(true)
    }

    fn load_manifest(&self) -> Result<bool, String> {
        let path = self.path().ok_or("the run is gone")?.join("qleany.yaml");
        self.manifest.try_open_path(&path.to_string_lossy())?;
        Ok(true)
    }

    fn run_checks(&self) -> Result<bool, String> {
        let report = handling_manifest_commands::check(&self.app_ctx).map_err(|e| e.to_string())?;
        for warning in &report.warnings {
            log::warn!("demo manifest: {warning}");
        }
        if report.critical_errors.is_empty() {
            return Ok(true);
        }
        // The whole list rather than a count: the manifest is Qleany's own template,
        // so a critical error here is a Qleany bug and the text is the bug report.
        Err(report.critical_errors.join("; "))
    }

    fn fill_files(&self) -> Result<bool, String> {
        // Each arm returns its own language's report, and neither is read: the
        // file rows the report describes are in the store, which is where the
        // writing step reads them from.
        match self.language_of_run() {
            Language::Rust => {
                rust_file_generation_commands::fill_rust_files(
                    &self.app_ctx,
                    &FillRustFilesDto {
                        only_list_already_existing: false,
                    },
                )
                .map_err(|e| e.to_string())?;
            }
            Language::CppQt => {
                cpp_qt_file_generation_commands::fill_cpp_qt_files(
                    &self.app_ctx,
                    &FillCppQtFilesDto {
                        only_list_already_existing: false,
                    },
                )
                .map_err(|e| e.to_string())?;
            }
        }
        Ok(true)
    }

    /// Render every file. The one step that takes real time, and the only one that
    /// runs anywhere but this thread.
    fn render_code(&self) -> Result<bool, String> {
        let operation = self
            .run
            .borrow()
            .as_ref()
            .and_then(|run| run.operation.clone());
        let Some(id) = operation else {
            let id = match self.language_of_run() {
                Language::Rust => {
                    rust_file_generation_commands::fill_code_in_rust_files(&self.app_ctx)
                }
                Language::CppQt => {
                    cpp_qt_file_generation_commands::fill_code_in_cpp_qt_files(&self.app_ctx)
                }
            }
            .map_err(|e| e.to_string())?;
            if let Some(run) = self.run.borrow_mut().as_mut() {
                run.operation = Some(id);
            }
            return Ok(false);
        };

        if let Some(progress) = long_operation_commands::get_operation_progress(&self.app_ctx, &id)
        {
            self.progress.set(lerp(
                Step::GenerateCode.progress(),
                Step::GenerateCode.progress_end(),
                progress.percentage / 100.0,
            ));
            if let Some(message) = progress.message.clone()
                && !message.is_empty()
            {
                self.message.set(message);
            }
        }

        // An operation the manager has forgotten counts as finished: waiting on one
        // that no longer exists would leave the dialog on screen for good, and the
        // dialog cannot be dismissed while it is running.
        let finished =
            long_operation_commands::is_operation_finished(&self.app_ctx, &id).unwrap_or(true);
        if !finished {
            return Ok(false);
        }
        if let Some(run) = self.run.borrow_mut().as_mut() {
            run.operation = None;
        }
        Ok(true)
    }

    fn compare_with_disk(&self) -> Result<bool, String> {
        file_generation_shared_steps_commands::fill_status_in_files(&self.app_ctx)
            .map_err(|e| e.to_string())?;

        // Collect what there is to write while the list is fresh. Ids only: the
        // rows carry every rendered body between them.
        let system_id = bootstrap::system_id(&self.app_ctx).ok_or("no System in the store")?;
        let ids = system_commands::get_system_relationship(
            &self.app_ctx,
            &system_id,
            &SystemRelationshipField::Files,
        )
        .map_err(|e| e.to_string())?;
        if let Some(run) = self.run.borrow_mut().as_mut() {
            run.pending = ids;
            run.cursor = 0;
        }
        Ok(true)
    }

    /// Write the next batch. `Ok(true)` once there is nothing left.
    fn write_chunk(&self) -> Result<bool, String> {
        let (path, chunk, cursor, total) = {
            let run = self.run.borrow();
            let run = run.as_ref().ok_or("the run is gone")?;
            let end = (run.cursor + WRITE_CHUNK).min(run.pending.len());
            (
                run.path.clone(),
                run.pending[run.cursor..end].to_vec(),
                end,
                run.pending.len(),
            )
        };
        if chunk.is_empty() {
            return Ok(true);
        }

        let rows =
            file_commands::get_file_multi(&self.app_ctx, &chunk).map_err(|e| e.to_string())?;
        let mut written = 0;
        for row in rows.into_iter().flatten() {
            // A file with nothing rendered is not an error: the generator lists
            // some files it deliberately leaves alone.
            let Some(code) = row.generated_code else {
                continue;
            };
            let mut directory = path.clone();
            if !row.relative_path.is_empty() {
                directory = directory.join(&row.relative_path);
            }
            std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            std::fs::write(directory.join(&row.name), code).map_err(|e| e.to_string())?;
            written += 1;
        }

        let done = if let Some(run) = self.run.borrow_mut().as_mut() {
            run.cursor = cursor;
            run.written += written;
            self.written.set_if_changed(run.written);
            run.cursor >= total
        } else {
            true
        };
        Ok(done)
    }

    fn init_git(&self) -> Result<bool, String> {
        init_git_and_tag(&self.path().ok_or("the run is gone")?)?;
        Ok(true)
    }

    // ── endings ──────────────────────────────────────────────────────────────

    fn succeed(&self) {
        self.run.borrow_mut().take();
        self.progress.set(1.0);
        self.message.set(tr!(demo_step_done()).resolve_now());
        self.phase.set(Phase::Done);
    }

    /// Stop, say why, and put the form back so the user can change something.
    ///
    /// The form rather than a dead end: every reason a demo fails is something the
    /// user can act on, and half of them are the destination.
    fn fail(&self, message: String) {
        log::error!("the demo failed: {message}");
        self.run.borrow_mut().take();
        self.error
            .set(Some(tr!(demo_error_failed(message = message))));
        self.progress.set(0.0);
        self.phase.set(Phase::Form);
    }

    // ── internals ────────────────────────────────────────────────────────────

    fn path(&self) -> Option<PathBuf> {
        self.run.borrow().as_ref().map(|run| run.path.clone())
    }

    fn language_of_run(&self) -> Language {
        self.run
            .borrow()
            .as_ref()
            .map(|run| run.language)
            .unwrap_or_default()
    }

    // ── wiring ───────────────────────────────────────────────────────────────

    /// Install the pump. Called from `App::build`, on every build.
    ///
    /// Wired from the application rather than from the dialog so the pipeline is
    /// driven by the window, not by the overlay: a modal that stopped building
    /// would otherwise stop a run that has already written files.
    ///
    /// Nothing is asked of the frame loop while no run is in flight, so an
    /// application that has never opened this dialog pays nothing for it.
    pub fn wire(&self, ctx: &mut BuildContext) {
        let wake = ctx.wake_at_handle();
        let tick = ctx.frame_tick();
        let me = self.clone();
        ctx.effect(&tick, move |_| {
            if let Some(delay) = me.tick() {
                wake.set(Some(std::time::Instant::now() + delay));
            }
        });
    }
}

/// Linear interpolation, clamped, for the steps that report from the inside.
fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t.clamp(0.0, 1.0)
}

/// Whether git can do what the C++/Qt demo needs of it.
///
/// Checked before anything is written, because both answers are things the user has
/// to leave the application to fix.
fn git_preflight() -> Result<(), Blocked> {
    if !git_is_present() {
        return Err(Blocked::NoGit);
    }
    if !git_has_identity() {
        return Err(Blocked::NoGitIdentity);
    }
    Ok(())
}

fn git_is_present() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Both halves of the identity `git commit` insists on.
///
/// Read through `git config` rather than from a file, so a value set in the system
/// config, the global one, or the environment all count.
fn git_has_identity() -> bool {
    ["user.name", "user.email"]
        .into_iter()
        .all(git_config_is_set)
}

fn git_config_is_set(key: &str) -> bool {
    Command::new("git")
        .args(["config", "--get", key])
        .output()
        .map(|out| out.status.success() && !String::from_utf8_lossy(&out.stdout).trim().is_empty())
        .unwrap_or(false)
}

/// A repository with one commit and one tag, which a Qt build reads its version
/// from. Idempotent: a folder that already has a `vX.Y.Z` tag is left alone.
fn init_git_and_tag(path: &Path) -> Result<(), String> {
    let run = |args: &[&str]| -> Result<bool, String> {
        Command::new("git")
            .args(args)
            .current_dir(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .map_err(|e| e.to_string())
    };

    if !path.join(".git").exists() && !run(&["init", "-b", "main"])? {
        return Err("git init failed".to_string());
    }

    let tags = Command::new("git")
        .args(["tag", "--list", "v*.*.*"])
        .current_dir(path)
        .output()
        .map_err(|e| e.to_string())?;
    if tags.status.success() && !String::from_utf8_lossy(&tags.stdout).trim().is_empty() {
        return Ok(());
    }

    if !run(&["add", "."])? {
        return Err("git add failed".to_string());
    }
    if !run(&["commit", "-m", "initial commit"])? {
        return Err("git commit failed".to_string());
    }
    if !run(&["tag", "v0.0.1"])? {
        return Err("git tag failed".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every step of the run, in order, for the language given.
    fn chain(language: Language) -> Vec<Step> {
        let mut steps = vec![Step::CreateFolder];
        while let Some(next) = steps.last().expect("never empty").next(language) {
            steps.push(next);
            assert!(steps.len() < 32, "the pipeline does not terminate");
        }
        steps
    }

    /// US-DEMO-03: a bar that went backwards would read as a restart.
    #[test]
    fn the_bar_only_ever_moves_forwards() {
        for language in Language::ALL {
            let mut last = 0.0_f32;
            for step in chain(language) {
                assert!(
                    step.progress() >= last,
                    "{step:?} goes backwards for {language:?}"
                );
                assert!(step.progress_end() >= step.progress(), "{step:?}");
                last = step.progress_end();
            }
            assert!(last < 1.0, "only a finished run is at the end of the bar");
        }
    }

    /// The git step is C++/Qt's alone: a Rust demo has no tag to make, and a step
    /// that did nothing would still hold the bar for a frame.
    #[test]
    fn only_the_cpp_qt_run_touches_git() {
        assert!(!chain(Language::Rust).contains(&Step::InitGit));
        assert!(chain(Language::CppQt).contains(&Step::InitGit));
    }

    /// Every step has to be reachable, or it is dead code that reads as a feature.
    #[test]
    fn every_step_is_in_a_run() {
        let mut seen: Vec<Step> = chain(Language::Rust);
        seen.extend(chain(Language::CppQt));
        for step in [
            Step::CreateFolder,
            Step::CreateManifest,
            Step::LoadManifest,
            Step::RunChecks,
            Step::FillFiles,
            Step::GenerateCode,
            Step::CompareWithDisk,
            Step::WriteFiles,
            Step::InitGit,
        ] {
            assert!(seen.contains(&step), "{step:?} is unreachable");
        }
    }

    /// Only the step that watches a thread is worth waiting on.
    #[test]
    fn only_the_rendering_step_polls() {
        for language in Language::ALL {
            for step in chain(language) {
                let expected = if step == Step::GenerateCode {
                    POLL_INTERVAL
                } else {
                    Duration::ZERO
                };
                assert_eq!(step.retry_delay(), expected, "{step:?}");
            }
        }
    }

    /// Nine steps, nine sentences: a shared one would have to say "working".
    #[test]
    fn each_step_says_something_different() {
        let mut said: Vec<String> = chain(Language::CppQt)
            .into_iter()
            .map(|s| s.message().resolve_now())
            .collect();
        let count = said.len();
        said.sort();
        said.dedup();
        assert_eq!(said.len(), count);
    }

    /// US-DEMO-02.
    #[test]
    fn a_leading_tilde_becomes_the_home_directory() {
        let home = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(expand_tilde("~/qleany-demo"), home.join("qleany-demo"));
        // Only a leading one, and only as a whole component: a folder genuinely
        // called `~backup` is a folder, not a home directory.
        assert_eq!(expand_tilde("/tmp/~/x"), PathBuf::from("/tmp/~/x"));
        assert_eq!(expand_tilde("~backup"), PathBuf::from("~backup"));
    }

    /// US-DEMO-02: the field defaults to `~/qleany-demo`, and clearing it means the
    /// same thing rather than meaning nothing.
    #[test]
    fn an_empty_destination_is_the_default_one() {
        let default = expand_tilde(DEFAULT_DESTINATION);
        assert_eq!(resolve_destination(""), default);
        assert_eq!(resolve_destination("   "), default);
        assert_eq!(resolve_destination(DEFAULT_DESTINATION), default);
        assert_eq!(resolve_destination("  /tmp/x  "), PathBuf::from("/tmp/x"));
    }

    /// US-DEMO-02: Browse picks the parent, not the project.
    #[test]
    fn browse_appends_the_project_folder() {
        assert_eq!(
            browsed_destination(Path::new("/home/me/code")),
            PathBuf::from("/home/me/code/qleany-demo")
        );
    }

    /// US-DEMO-05: the command runs the thing that was generated.
    #[test]
    fn the_next_command_matches_the_language() {
        let path = Path::new("/tmp/qleany-demo");
        let rust = next_command(Language::Rust, path);
        assert!(rust.starts_with("cd /tmp/qleany-demo"), "{rust}");
        assert!(rust.contains("cargo run --bin demo"), "{rust}");

        let cpp = next_command(Language::CppQt, path);
        assert!(cpp.contains("cmake"), "{cpp}");
        assert!(!cpp.contains("cargo"), "{cpp}");
    }

    /// The demo asks the backend for two frontends per language, in its vocabulary.
    #[test]
    fn the_demo_asks_for_the_frontends_it_advertises() {
        let rust: Vec<&str> = demo_targets(Language::Rust)
            .into_iter()
            .map(option_name)
            .collect();
        assert_eq!(rust, vec!["rust_cli", "rust_teksilo"]);

        let cpp: Vec<&str> = demo_targets(Language::CppQt)
            .into_iter()
            .map(option_name)
            .collect();
        assert_eq!(cpp, vec!["cpp_qt_qtquick", "cpp_qt_qtwidgets"]);
    }

    /// US-DEMO-04, and the reason the sentence is derived rather than written out:
    /// the CLI and the Slint UI both still say "CLI and Slint" for a Rust demo that
    /// has generated a CLI and a Teksilo window since the Teksilo templates landed.
    #[test]
    fn the_summary_names_the_frontends_that_were_generated() {
        let rust = ui_line(Language::Rust).resolve_now();
        assert!(rust.contains("CLI"), "{rust}");
        assert!(rust.contains("Teksilo"), "{rust}");
        assert!(!rust.contains("Slint"), "{rust}");

        let cpp = ui_line(Language::CppQt).resolve_now();
        assert!(cpp.contains("Qt Quick"), "{cpp}");
        assert!(cpp.contains("Qt Widgets"), "{cpp}");
    }

    /// The bullet must not carry the User Interface screen's parenthetical.
    #[test]
    fn the_summary_does_not_recommend_anything() {
        assert!(
            !ui_line(Language::Rust)
                .resolve_now()
                .contains("recommended")
        );
    }

    /// US-DEMO-06: three reasons, three sentences, and the one about a path says
    /// which path.
    #[test]
    fn a_blocked_run_says_which_thing_blocked_it() {
        let exists = Blocked::ManifestExists("/home/me/qleany-demo".to_string())
            .message()
            .resolve_now();
        assert!(exists.contains("/home/me/qleany-demo"), "{exists}");

        let no_git = Blocked::NoGit.message().resolve_now();
        assert!(no_git.to_lowercase().contains("git"), "{no_git}");

        let identity = Blocked::NoGitIdentity.message().resolve_now();
        assert!(identity.contains("user.name"), "{identity}");
        assert!(identity.contains("user.email"), "{identity}");

        let mut said = vec![exists, no_git, identity];
        said.sort();
        said.dedup();
        assert_eq!(said.len(), 3);
    }

    /// The failure sentence carries the backend's words rather than replacing them.
    #[test]
    fn a_failure_quotes_the_reason() {
        let said = tr!(demo_error_failed(message = "no space left on device")).resolve_now();
        assert!(said.contains("no space left on device"), "{said}");
    }

    /// US-DEMO-04: both numbers, in a sentence.
    #[test]
    fn the_summary_counts_files_and_lines() {
        let said = tr!(demo_summary_stats(files = 312_i64, lines = 470_i64)).resolve_now();
        assert!(said.contains("312"), "{said}");
        assert!(said.contains("470"), "{said}");
    }

    /// A backend that reported more than 100 per cent, or a negative, must not push
    /// the bar past the step it is in.
    #[test]
    fn the_inner_progress_stays_inside_its_step() {
        assert_eq!(lerp(0.3, 0.8, 0.0), 0.3);
        assert_eq!(lerp(0.3, 0.8, 1.0), 0.8);
        assert_eq!(lerp(0.3, 0.8, 2.0), 0.8);
        assert_eq!(lerp(0.3, 0.8, -1.0), 0.3);
    }
}
