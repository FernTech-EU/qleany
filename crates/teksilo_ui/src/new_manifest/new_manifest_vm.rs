//! What the new-manifest wizard collects, and what it does with it.
//!
//! Four steps: the target language, the two names, a starting template, and which
//! frontends to scaffold. Only the second can be got wrong, so only the second
//! gates Next.

use std::rc::Rc;

use teksilo::prelude::*;

use frontend::AppContext;
use frontend::commands::handling_manifest_commands;
use frontend::handling_manifest::dtos::{CreateDto, CreateLanguage, ManifestTemplate};

use crate::project::Language;
use crate::shared::validation::is_pascal_case;
use crate::user_interface::Target;

/// The manifest a wizard run starts from.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Template {
    #[default]
    Blank,
    Minimal,
    DocumentEditor,
    DataManagement,
}

impl Template {
    /// In the order the step offers them, simplest first.
    pub const ALL: [Template; 4] = [
        Template::Blank,
        Template::Minimal,
        Template::DocumentEditor,
        Template::DataManagement,
    ];

    pub fn title(self) -> LocalizedString {
        match self {
            Template::Blank => tr!(wizard_template_blank()),
            Template::Minimal => tr!(wizard_template_minimal()),
            Template::DocumentEditor => tr!(wizard_template_document_editor()),
            Template::DataManagement => tr!(wizard_template_data_management()),
        }
    }

    pub fn description(self) -> LocalizedString {
        match self {
            Template::Blank => tr!(wizard_template_blank_blurb()),
            Template::Minimal => tr!(wizard_template_minimal_blurb()),
            Template::DocumentEditor => tr!(wizard_template_document_editor_blurb()),
            Template::DataManagement => tr!(wizard_template_data_management_blurb()),
        }
    }

    /// What the backend calls it.
    pub fn dto(self) -> ManifestTemplate {
        match self {
            Template::Blank => ManifestTemplate::Blank,
            Template::Minimal => ManifestTemplate::Minimal,
            Template::DocumentEditor => ManifestTemplate::DocumentEditor,
            Template::DataManagement => ManifestTemplate::DataManagement,
        }
    }

    pub fn of_index(index: usize) -> Template {
        Template::ALL.get(index).copied().unwrap_or_default()
    }
}

/// What the backend calls a language in a `CreateDto`.
///
/// A second enum rather than [`Language`] reused: the manifest stores `rust` and
/// `cpp-qt` as strings, and the create command takes a typed enum. Mapping between
/// them here is what keeps that difference out of the wizard's steps.
pub fn create_language(language: Language) -> CreateLanguage {
    match language {
        Language::Rust => CreateLanguage::Rust,
        Language::CppQt => CreateLanguage::CppQt,
    }
}

/// The option strings `CreateDto` understands.
///
/// The backend validates them against a fixed list and fails the whole create on an
/// unknown one, so these are its vocabulary rather than a display choice.
pub fn option_name(target: Target) -> &'static str {
    match target {
        Target::RustCli => "rust_cli",
        Target::RustTeksilo => "rust_teksilo",
        Target::RustSlint => "rust_slint",
        Target::RustIos => "rust_ios",
        Target::RustAndroid => "rust_android",
        Target::CppQtWidgets => "cpp_qt_qtwidgets",
        Target::CppQtQuick => "cpp_qt_qtquick",
    }
}

/// Which frontends a fresh manifest starts with.
///
/// Teksilo for Rust and Qt Quick for C++/Qt: each language's own toolkit, which is
/// the choice a user who has not thought about it yet would want.
pub fn default_targets(language: Language) -> Vec<Target> {
    match language {
        Language::Rust => vec![Target::RustTeksilo],
        Language::CppQt => vec![Target::CppQtQuick],
    }
}

/// The frontends a wizard run may offer.
///
/// Fewer than the User Interface screen: iOS and Android are UniFFI bindings over
/// an existing project rather than something to start one with.
pub fn offered_targets(language: Language) -> Vec<Target> {
    match language {
        Language::Rust => vec![Target::RustCli, Target::RustTeksilo, Target::RustSlint],
        Language::CppQt => vec![Target::CppQtQuick, Target::CppQtWidgets],
    }
}

#[derive(Clone)]
pub struct NewManifestViewModel {
    app_ctx: Rc<AppContext>,
    /// Step 1.
    language: Signal<usize>,
    /// Step 2.
    application_name: Signal<String>,
    organisation_name: Signal<String>,
    /// Step 3.
    template: Signal<usize>,
    /// Step 4, as a tick per offered target.
    targets: Signal<Vec<Target>>,
    /// The language the ticks were chosen for.
    ///
    /// The radio group on step 1 writes `language` itself, so nothing is watching
    /// it, and an observer would be the wrong answer anyway: an `ObserverHandle`
    /// dropped at the end of the statement that made it unregisters immediately,
    /// which is a silent no-op rather than an error. The ticks are reconciled when
    /// they are read instead, which is when step 4 is built.
    targets_language: Rc<std::cell::Cell<Language>>,
    /// What went wrong on the last attempt, shown on the final step.
    error: Signal<Option<LocalizedString>>,
}

impl std::fmt::Debug for NewManifestViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewManifestViewModel")
            .field("language", &self.chosen_language())
            .finish_non_exhaustive()
    }
}

impl NewManifestViewModel {
    pub fn new(app_ctx: Rc<AppContext>) -> Self {
        let me = Self {
            app_ctx,
            language: Signal::new(0),
            application_name: Signal::new(String::new()),
            organisation_name: Signal::new(String::new()),
            template: Signal::new(0),
            targets: Signal::new(Vec::new()),
            targets_language: Rc::new(std::cell::Cell::new(Language::Rust)),
            error: Signal::new(None),
        };
        me.targets.set(default_targets(me.chosen_language()));
        me
    }

    /// Bring the ticks in line with the chosen language.
    ///
    /// A Rust manifest carrying `cpp_qt_qtquick` fails the backend's own option
    /// check, so the ticks cannot simply survive a language change.
    fn reconcile_targets(&self) {
        let language = self.chosen_language();
        if self.targets_language.get() == language {
            return;
        }
        self.targets_language.set(language);
        self.targets.set(default_targets(language));
    }

    // ── state a step binds ───────────────────────────────────────────────────

    pub fn language_index(&self) -> Signal<usize> {
        self.language.clone()
    }

    pub fn template_index(&self) -> Signal<usize> {
        self.template.clone()
    }

    pub fn application_name(&self) -> Signal<String> {
        self.application_name.clone()
    }

    pub fn organisation_name(&self) -> Signal<String> {
        self.organisation_name.clone()
    }

    pub fn error(&self) -> Signal<Option<LocalizedString>> {
        self.error.clone()
    }

    /// `RadioTileGroup` selects by index, so the enum lives behind one.
    pub fn chosen_language(&self) -> Language {
        match self.language.get() {
            0 => Language::Rust,
            _ => Language::CppQt,
        }
    }

    pub fn chosen_template(&self) -> Template {
        Template::of_index(self.template.get())
    }

    pub fn targets(&self) -> Vec<Target> {
        self.reconcile_targets();
        self.targets.get()
    }

    pub fn has_target(&self, target: Target) -> bool {
        self.targets().contains(&target)
    }

    /// Whether step 2 is answered. The only step that can be wrong, and therefore
    /// the only one that gates Next.
    pub fn names_are_valid(&self) -> Signal<bool> {
        self.application_name
            .zip(&self.organisation_name)
            .map(|(app, org)| name_error(app).is_none() && org_error(org).is_none())
    }

    pub fn application_name_validation(&self) -> Signal<teksilo::widgets::ValidationState> {
        self.application_name.map(|name| match name_error(name) {
            Some(message) => teksilo::widgets::ValidationState::Error(message),
            None => teksilo::widgets::ValidationState::None,
        })
    }

    pub fn organisation_name_validation(&self) -> Signal<teksilo::widgets::ValidationState> {
        self.organisation_name.map(|name| match org_error(name) {
            Some(message) => teksilo::widgets::ValidationState::Error(message),
            None => teksilo::widgets::ValidationState::None,
        })
    }

    // ── commands ─────────────────────────────────────────────────────────────

    /// Choosing a language replaces the ticks, because the targets it offers are a
    /// different set: a Rust manifest carrying `cpp_qt_qtquick` would fail the
    /// backend's own option check.
    pub fn set_language(&self, index: usize) {
        self.language.set_if_changed(index);
        self.reconcile_targets();
    }

    pub fn set_target(&self, target: Target, wanted: bool) {
        let mut targets = self.targets();
        targets.retain(|t| *t != target);
        if wanted {
            targets.push(target);
        }
        self.targets.set(targets);
    }

    /// Create the manifest at `path` and load it.
    ///
    /// The path comes from a save dialog the caller ran: a wizard step cannot open
    /// one and wait for it, because the dialog is asynchronous and the step would
    /// have to finish first.
    pub fn create(&self, path: &str) -> Result<String, String> {
        let language = self.chosen_language();
        let options: Vec<String> = offered_targets(language)
            .into_iter()
            .filter(|t| self.has_target(*t))
            .map(|t| option_name(t).to_string())
            .collect();

        let dto = CreateDto {
            manifest_path: path.to_string(),
            language: create_language(language),
            application_name: self.application_name.get().trim().to_string(),
            organization_name: self.organisation_name.get().trim().to_string(),
            manifest_template: self.chosen_template().dto(),
            options,
        };

        match handling_manifest_commands::create(&self.app_ctx, &dto) {
            Ok(created) => {
                self.error.set(None);
                Ok(created.manifest_path)
            }
            Err(e) => {
                let message = e.to_string();
                self.error
                    .set(Some(tr!(status_error(message = message.clone()))));
                Err(message)
            }
        }
    }
}

/// What is wrong with an application name, if anything.
///
/// Qleany turns it into a crate name and a set of type names, so an empty one or
/// one that is not PascalCase produces a project that does not build.
pub fn name_error(name: &str) -> Option<LocalizedString> {
    if name.trim().is_empty() {
        Some(tr!(wizard_required()))
    } else if !is_pascal_case(name) {
        Some(tr!(wizard_must_be_pascal_case()))
    } else {
        None
    }
}

/// The organisation only has to be there: it becomes a domain fragment rather than
/// a type name, so no case rule applies.
pub fn org_error(name: &str) -> Option<LocalizedString> {
    if name.trim().is_empty() {
        Some(tr!(wizard_required()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vm() -> NewManifestViewModel {
        NewManifestViewModel::new(Rc::new(AppContext::new()))
    }

    /// US-WIZ-02: Rust first, and the choice is exclusive.
    #[test]
    fn the_language_is_one_of_two() {
        let vm = vm();
        assert_eq!(vm.chosen_language(), Language::Rust);
        vm.set_language(1);
        assert_eq!(vm.chosen_language(), Language::CppQt);
    }

    /// US-WIZ-05: the ticks are the new language's, not the old one's. A Rust
    /// manifest carrying a Qt option fails the backend's own check on create.
    #[test]
    fn changing_the_language_replaces_the_targets() {
        let vm = vm();
        assert_eq!(vm.targets(), vec![Target::RustTeksilo]);

        vm.set_target(Target::RustCli, true);
        assert!(vm.has_target(Target::RustCli));

        vm.set_language(1);
        assert_eq!(vm.targets(), vec![Target::CppQtQuick]);
        assert!(
            !vm.has_target(Target::RustCli),
            "a Qt manifest has no Rust CLI"
        );
    }

    #[test]
    fn a_target_can_be_ticked_and_unticked() {
        let vm = vm();
        vm.set_target(Target::RustSlint, true);
        assert!(vm.has_target(Target::RustSlint));
        vm.set_target(Target::RustSlint, false);
        assert!(!vm.has_target(Target::RustSlint));

        // Ticking twice does not tick twice.
        vm.set_target(Target::RustCli, true);
        vm.set_target(Target::RustCli, true);
        assert_eq!(
            vm.targets()
                .iter()
                .filter(|t| **t == Target::RustCli)
                .count(),
            1
        );
    }

    /// US-WIZ-05: each language offers its own, and never the mobile bindings,
    /// which are wrapped around an existing project rather than starting one.
    #[test]
    fn each_language_offers_its_own_frontends() {
        let rust = offered_targets(Language::Rust);
        assert_eq!(
            rust,
            vec![Target::RustCli, Target::RustTeksilo, Target::RustSlint]
        );
        assert!(!rust.contains(&Target::RustIos));

        assert_eq!(
            offered_targets(Language::CppQt),
            vec![Target::CppQtQuick, Target::CppQtWidgets]
        );
    }

    /// US-WIZ-03: the two messages are told apart, and Next is gated on both.
    #[test]
    fn the_names_are_validated_separately() {
        let vm = vm();
        let valid = vm.names_are_valid();
        assert!(!valid.get(), "an empty wizard is not ready");

        vm.application_name().set("my app".to_string());
        assert_eq!(
            name_error("my app").map(|m| m.resolve_now()),
            Some("Must be PascalCase".to_string())
        );
        assert!(!valid.get());

        vm.application_name().set("MyApp".to_string());
        assert!(!valid.get(), "the organisation is still empty");

        vm.organisation_name().set("FernTech".to_string());
        assert!(valid.get());
    }

    /// The organisation becomes a domain fragment, not a type name, so it carries
    /// no case rule: "Fern Tech" is a name a user may legitimately have.
    #[test]
    fn the_organisation_has_no_case_rule() {
        assert!(org_error("Fern Tech").is_none());
        assert!(org_error("   ").is_some());
    }

    /// US-WIZ-04: four templates, each with its own words.
    #[test]
    fn every_template_is_offered_and_described() {
        assert_eq!(Template::ALL.len(), 4);
        let mut titles: Vec<String> = Template::ALL
            .iter()
            .map(|t| t.title().resolve_now())
            .collect();
        titles.sort();
        titles.dedup();
        assert_eq!(titles.len(), 4, "two templates share a title");

        assert_eq!(Template::of_index(3), Template::DataManagement);
        assert_eq!(
            Template::of_index(99),
            Template::Blank,
            "out of range is Blank"
        );
    }

    /// The option strings are the backend's vocabulary: it validates them against a
    /// fixed list and fails the whole create on an unknown one.
    #[test]
    fn the_option_names_are_the_backends_own() {
        assert_eq!(option_name(Target::RustTeksilo), "rust_teksilo");
        assert_eq!(option_name(Target::CppQtWidgets), "cpp_qt_qtwidgets");
        assert_eq!(option_name(Target::CppQtQuick), "cpp_qt_qtquick");
    }
}
