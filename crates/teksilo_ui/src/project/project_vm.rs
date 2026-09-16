//! What the Project settings screen knows.
//!
//! One `Global` row, read and written through the generated [`SingleGlobal`]. The
//! handle already holds a signal per field, tracks its own dirtiness and writes
//! itself back, so this view-model is the three things it cannot know: which
//! `Global` to point at, when a field is finished being edited, and what the
//! manifest's language code means to a user.

use std::rc::Rc;

use teksilo::prelude::*;
use teksilo::widgets::ValidationState;

use frontend::AppContext;
use frontend::EntityId;

use crate::app_ids::AppIds;
use crate::edit::{UndoAction, labeled};
use crate::shared::validation::required;
use crate::singles::SingleGlobal;

/// The languages Qleany generates for.
///
/// The manifest stores a code, the combo shows a label, and the two are kept apart
/// on purpose: the codes are part of the file format and must never move, while the
/// labels are display text.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Language {
    #[default]
    Rust,
    CppQt,
}

impl Language {
    /// In the order the combo offers them.
    pub const ALL: [Language; 2] = [Language::Rust, Language::CppQt];

    /// What the manifest stores.
    pub fn code(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::CppQt => "cpp-qt",
        }
    }

    /// Read a manifest's language code.
    ///
    /// Anything unrecognised reads as Rust rather than failing: a manifest written
    /// by a newer Qleany, or by hand, still opens, and the user sees a language they
    /// can change. `cpp_qt` is accepted beside `cpp-qt` because both spellings are
    /// in the wild.
    pub fn from_code(code: &str) -> Language {
        match code.trim().to_lowercase().as_str() {
            "cpp-qt" | "cpp_qt" | "cppqt" => Language::CppQt,
            _ => Language::Rust,
        }
    }

    /// What the combo shows.
    pub fn label(self) -> LocalizedString {
        match self {
            Language::Rust => tr!(project_language_rust()),
            Language::CppQt => tr!(project_language_cpp_qt()),
        }
    }

    /// The prefix path the generator uses when the field is left empty.
    ///
    /// Not a fallback this screen applies: the field is genuinely allowed to be
    /// empty, and the backend picks this. Saying which one it will pick is the whole
    /// point of the placeholder.
    pub fn default_prefix_path(self) -> &'static str {
        match self {
            Language::Rust => "crates",
            Language::CppQt => "src",
        }
    }
}

#[derive(Clone)]
pub struct ProjectViewModel {
    app_ctx: Rc<AppContext>,
    ids: AppIds,
    single: SingleGlobal,
    /// What the combo is showing. Derived from the handle's `language` code, and
    /// written by the combo itself, which is why it cannot simply be a `map`.
    language: Signal<Option<Language>>,
}

impl std::fmt::Debug for ProjectViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectViewModel")
            .field("global", &self.single.id())
            .finish_non_exhaustive()
    }
}

impl ProjectViewModel {
    pub fn new(app_ctx: Rc<AppContext>, single: SingleGlobal, ids: AppIds) -> Self {
        let language = Signal::new(None);
        let me = Self {
            app_ctx,
            ids,
            single,
            language,
        };
        me.sync_language();
        me
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn language(&self) -> Signal<Option<Language>> {
        self.language.clone()
    }

    /// The language as a plain value, for the one thing that cannot take a signal.
    ///
    /// `TextInput::placeholder` is not reactive, so the prefix-path placeholder is
    /// resolved when the form is built and the form is rebuilt when this changes.
    /// [`crate::project::ProjectPage`] is what arranges the rebuild.
    pub fn language_now(&self) -> Language {
        self.language.get().unwrap_or_default()
    }

    pub fn application_name(&self) -> Signal<String> {
        self.single.application_name()
    }

    pub fn organisation_name(&self) -> Signal<String> {
        self.single.organisation_name()
    }

    pub fn organisation_domain(&self) -> Signal<String> {
        self.single.organisation_domain()
    }

    pub fn prefix_path(&self) -> Signal<String> {
        self.single.prefix_path()
    }

    pub fn application_name_validation(&self) -> Signal<ValidationState> {
        required(
            &self.single.application_name(),
            tr!(project_application_name_required()),
        )
    }

    pub fn organisation_name_validation(&self) -> Signal<ValidationState> {
        required(
            &self.single.organisation_name(),
            tr!(project_organisation_name_required()),
        )
    }

    pub fn organisation_domain_validation(&self) -> Signal<ValidationState> {
        required(
            &self.single.organisation_domain(),
            tr!(project_organisation_domain_required()),
        )
    }

    // ── commands ─────────────────────────────────────────────────────────────

    /// Write the edited fields back to the store, as one undo entry.
    ///
    /// Called when a field is finished with, on Enter or on losing focus, rather
    /// than on every keystroke: the handle records one undo entry per write, so a
    /// per-keystroke commit would put "Project settings" thirty deep in the stack
    /// for one typed name. The handle no-ops when nothing changed, so a blur that
    /// edited nothing costs a comparison.
    pub fn commit(&self) {
        let stack = self.ids.project_stack.get();
        labeled(&self.app_ctx, stack, UndoAction::EditProject, || {
            self.single.save(stack)
        });
    }

    /// Pick the target language, and write it at once.
    ///
    /// A combo has no "finished editing" moment to wait for: choosing is the whole
    /// interaction.
    pub fn set_language(&self, language: Language) {
        self.single.set_language(language.code().to_string());
        self.commit();
        // Not left to `wire`'s effect. The combo writes its own selection, so on
        // screen this looks the same either way, but a view-model that only agrees
        // with itself while it happens to be mounted is a trap for the next caller.
        self.sync_language();
    }

    // ── wiring ───────────────────────────────────────────────────────────────

    /// Install the subscriptions. Called from `build`, on **every** build: an effect
    /// registered on a `BuildContext` lives exactly one build cycle, so a guard here
    /// would leave the screen deaf after the first rebuild.
    pub fn wire(&self, ctx: &mut BuildContext) {
        // Follow whichever `Global` the open manifest owns. A manifest that is
        // closed sets this to `None`, which empties the handle and with it the form.
        let me = self.clone();
        let global_id = self.ids.global_id.clone();
        ctx.effect(&global_id, move |id| me.point_at(*id));
        self.point_at(global_id.get());

        // The handle's code to the combo's value. One way only: the reverse is
        // `set_language`, called from the combo's own selection handler, and a
        // second effect running the other way would be a loop rather than a bridge.
        // Captures the combo's signal alone, never `self`. The signal observed
        // here belongs to the generated handle, and that handle keeps an
        // `ObserverHandle` on it for its own dirty tracking. A closure holding
        // the view-model would therefore hold the handle, so tearing the window
        // down would drop the handle from inside the signal's own borrow and
        // abort with "RefCell already borrowed" in a destructor.
        let language = self.language.clone();
        let code = self.single.language();
        ctx.effect(&code, move |code| {
            language.set_if_changed(Some(Language::from_code(code)));
        });
        self.sync_language();
    }

    /// Point the handle at a `Global`, if it is not already there.
    ///
    /// The guard matters: `set_id` reloads every field signal from the store, so
    /// calling it with the id the handle already holds would throw away whatever the
    /// user has typed but not yet committed.
    fn point_at(&self, id: Option<EntityId>) {
        if self.single.id() == id {
            return;
        }
        self.single.set_id(id);
    }

    /// Bring the combo's value in line with the stored code.
    fn sync_language(&self) {
        let code = self.single.language().get();
        self.language
            .set_if_changed(Some(Language::from_code(&code)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_code_round_trips() {
        for language in Language::ALL {
            assert_eq!(Language::from_code(language.code()), language);
        }
    }

    /// A manifest written by hand, or by a newer Qleany, still opens.
    #[test]
    fn an_unknown_language_code_reads_as_rust() {
        assert_eq!(Language::from_code("elvish"), Language::Rust);
        assert_eq!(Language::from_code(""), Language::Rust);
        // Both spellings of the C++ target are in the wild.
        assert_eq!(Language::from_code("CPP-QT"), Language::CppQt);
        assert_eq!(Language::from_code("cpp_qt"), Language::CppQt);
    }

    #[test]
    fn each_language_names_its_own_default_prefix() {
        assert_eq!(Language::Rust.default_prefix_path(), "crates");
        assert_eq!(Language::CppQt.default_prefix_path(), "src");
    }

    /// The rule itself belongs to `shared::validation`; what this pins is that the
    /// screen wired each field to its own message rather than to one shared one.
    #[test]
    fn each_required_field_carries_its_own_message() {
        let ids = AppIds::new();
        let ctx = Rc::new(AppContext::new());
        let vm = ProjectViewModel::new(ctx.clone(), SingleGlobal::new(ctx), ids);
        // Emptied rather than assumed empty: under `mocks` the handle starts with a
        // fabricated row, and this test is about the messages, not about what a
        // fresh handle happens to hold.
        for field in [
            vm.application_name(),
            vm.organisation_name(),
            vm.organisation_domain(),
        ] {
            field.set(String::new());
        }

        let messages: Vec<String> = [
            vm.application_name_validation(),
            vm.organisation_name_validation(),
            vm.organisation_domain_validation(),
        ]
        .iter()
        .map(|state| match state.get() {
            ValidationState::Error(m) => m.resolve_now(),
            other => panic!("an empty required field must be an error, got {other:?}"),
        })
        .collect();

        let mut unique = messages.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 3, "two fields share a message: {messages:?}");
    }

    /// Everything below drives the real store.
    ///
    /// Under `mocks` the generated handle is a fabricator with nothing behind it:
    /// `set_id` invents a row, `save` only clears the dirty flag, and asserting that
    /// an edit reached a backend would be asserting against a stub. The mock surface
    /// gets its own assertions in `tests/mocks_arm.rs`.
    #[cfg(not(feature = "mocks"))]
    mod with_a_backend {
        use super::*;
        use std::rc::Rc;

        use frontend::AppContext;
        use frontend::commands::{global_commands, handling_app_lifecycle_commands};
        use frontend::direct_access::CreateGlobalDto;

        /// A view-model over a real backend, with a `Global` row to edit and the ids
        /// pointed at it, exactly as a manifest load leaves them.
        fn vm() -> (Rc<AppContext>, ProjectViewModel, EntityId) {
            let ctx = Rc::new(AppContext::new());
            handling_app_lifecycle_commands::initialize_app(&ctx).expect("initialize_app");

            let now = chrono::Utc::now();
            let global = global_commands::create_orphan_global(
                &ctx,
                None,
                &CreateGlobalDto {
                    language: "rust".to_string(),
                    application_name: "Demo".to_string(),
                    organisation_name: "FernTech".to_string(),
                    organisation_domain: "eu.ferntech".to_string(),
                    prefix_path: String::new(),
                    created_at: now,
                    updated_at: now,
                },
            )
            .expect("create global");

            let ids = AppIds::new();
            ids.global_id.set(Some(global.id));
            let vm = ProjectViewModel::new(ctx.clone(), SingleGlobal::new(ctx.clone()), ids);
            vm.point_at(Some(global.id));
            vm.sync_language();
            (ctx, vm, global.id)
        }

        #[test]
        fn the_form_starts_at_what_the_store_holds() {
            let (_ctx, vm, _id) = vm();
            assert_eq!(vm.application_name().get(), "Demo");
            assert_eq!(vm.organisation_domain().get(), "eu.ferntech");
            assert_eq!(vm.language_now(), Language::Rust);
        }

        /// The whole point of the screen: an edit, committed, reaches the store.
        #[test]
        fn a_committed_edit_reaches_the_backend() {
            let (ctx, vm, id) = vm();
            vm.application_name().set("Renamed".to_string());
            vm.commit();

            let stored = global_commands::get_global(&ctx, &id)
                .expect("read back")
                .expect("the global is still there");
            assert_eq!(stored.application_name, "Renamed");
        }

        /// An edit that was never committed is not in the store, which is what makes
        /// Enter and blur meaningful rather than decorative.
        #[test]
        fn an_uncommitted_edit_stays_out_of_the_backend() {
            let (ctx, vm, id) = vm();
            vm.application_name()
                .set("Typed but not finished".to_string());

            let stored = global_commands::get_global(&ctx, &id)
                .expect("read back")
                .expect("the global is still there");
            assert_eq!(stored.application_name, "Demo");
        }

        #[test]
        fn choosing_a_language_writes_its_code_not_its_label() {
            let (ctx, vm, id) = vm();
            vm.set_language(Language::CppQt);

            let stored = global_commands::get_global(&ctx, &id)
                .expect("read back")
                .expect("the global is still there");
            assert_eq!(stored.language, "cpp-qt");
            assert_eq!(vm.language().get(), Some(Language::CppQt));
        }

        /// Closing a manifest empties the form rather than leaving the previous
        /// project's settings on screen.
        #[test]
        fn losing_the_global_empties_the_form() {
            let (_ctx, vm, _id) = vm();
            assert_eq!(vm.application_name().get(), "Demo");

            vm.point_at(None);
            assert_eq!(vm.application_name().get(), "");
            assert!(vm.single.id().is_none());
        }

        /// Re-pointing at the id the handle already holds must not reload, or a user who
        /// typed into a field and then triggered any id write would watch their text
        /// vanish.
        #[test]
        fn re_pointing_at_the_same_global_keeps_uncommitted_text() {
            let (_ctx, vm, id) = vm();
            vm.application_name().set("Half typed".to_string());
            vm.point_at(Some(id));
            assert_eq!(vm.application_name().get(), "Half typed");
        }
    }
}
