//! The named commands anything in the app can fire.
//!
//! An intent is a name, not a method call. A menu entry, a shortcut, a title-bar
//! button and an automation probe all raise the same name, and one `Action`
//! registered in `App::build` consumes it. That indirection is not decoration: the
//! title-bar menu renders in an *overlay*, a sibling of `App` rather than a
//! descendant, so a handler that called a view-model method directly would have to
//! capture one, and every button would need its own clone of every view-model.
//!
//! Actions are registered with `register_action_global` for the same reason: intents
//! walk source widget to root, and a plain `register_action` only fires when the
//! registering widget happens to be on that path.

use teksilo::IntentKind;

#[derive(Debug, IntentKind)]
pub enum AppIntent {
    // Manifest lifecycle
    #[name = "manifest.new"]
    NewManifest,
    #[name = "manifest.open"]
    OpenManifest,
    #[name = "manifest.save"]
    SaveManifest,
    #[name = "manifest.save_as"]
    SaveManifestAs,
    #[name = "manifest.close"]
    CloseManifest,
    #[name = "manifest.open_qleany"]
    OpenQleanyManifest,

    // Edit
    #[name = "edit.undo"]
    Undo,
    #[name = "edit.redo"]
    Redo,

    // View
    #[name = "view.theme.toggle"]
    ToggleTheme,
    #[name = "view.screen.home"]
    ShowHome,
    #[name = "view.screen.project"]
    ShowProject,
    #[name = "view.screen.entities"]
    ShowEntities,
    #[name = "view.screen.features"]
    ShowFeatures,
    #[name = "view.screen.user_interface"]
    ShowUserInterface,
    #[name = "view.screen.generate"]
    ShowGenerate,

    // Tools
    #[name = "demo.run"]
    RunDemo,
    #[name = "check.run"]
    RunCheck,
    #[name = "entities.export_mermaid"]
    ExportMermaid,

    // Help
    #[name = "help.about"]
    About,

    // Application
    #[name = "app.quit"]
    Quit,
}

/// The intent names as `&'static str`.
///
/// `MenuEntry::intent` and `Shortcut::new` both want a literal, and the derive
/// exposes no name accessor, so the strings live here. The test below asserts they
/// still match what the derive emits, which is what keeps a rename from silently
/// disconnecting a menu row from its action.
pub mod name {
    pub const NEW_MANIFEST: &str = "manifest.new";
    pub const OPEN_MANIFEST: &str = "manifest.open";
    pub const SAVE_MANIFEST: &str = "manifest.save";
    pub const SAVE_MANIFEST_AS: &str = "manifest.save_as";
    pub const CLOSE_MANIFEST: &str = "manifest.close";
    pub const OPEN_QLEANY_MANIFEST: &str = "manifest.open_qleany";
    pub const UNDO: &str = "edit.undo";
    pub const REDO: &str = "edit.redo";
    pub const TOGGLE_THEME: &str = "view.theme.toggle";
    pub const SHOW_HOME: &str = "view.screen.home";
    pub const SHOW_PROJECT: &str = "view.screen.project";
    pub const SHOW_ENTITIES: &str = "view.screen.entities";
    pub const SHOW_FEATURES: &str = "view.screen.features";
    pub const SHOW_USER_INTERFACE: &str = "view.screen.user_interface";
    pub const SHOW_GENERATE: &str = "view.screen.generate";
    pub const RUN_DEMO: &str = "demo.run";
    pub const RUN_CHECK: &str = "check.run";
    pub const EXPORT_MERMAID: &str = "entities.export_mermaid";
    pub const ABOUT: &str = "help.about";
    pub const QUIT: &str = "app.quit";
}

#[cfg(test)]
mod tests {
    use super::*;
    use teksilo::prelude::IntentKind;

    /// Every constant in `name` is the name the derive actually emits.
    #[test]
    fn the_name_constants_match_the_derive() {
        let pairs: &[(AppIntent, &str)] = &[
            (AppIntent::NewManifest, name::NEW_MANIFEST),
            (AppIntent::OpenManifest, name::OPEN_MANIFEST),
            (AppIntent::SaveManifest, name::SAVE_MANIFEST),
            (AppIntent::SaveManifestAs, name::SAVE_MANIFEST_AS),
            (AppIntent::CloseManifest, name::CLOSE_MANIFEST),
            (AppIntent::OpenQleanyManifest, name::OPEN_QLEANY_MANIFEST),
            (AppIntent::Undo, name::UNDO),
            (AppIntent::Redo, name::REDO),
            (AppIntent::ToggleTheme, name::TOGGLE_THEME),
            (AppIntent::ShowHome, name::SHOW_HOME),
            (AppIntent::ShowProject, name::SHOW_PROJECT),
            (AppIntent::ShowEntities, name::SHOW_ENTITIES),
            (AppIntent::ShowFeatures, name::SHOW_FEATURES),
            (AppIntent::ShowUserInterface, name::SHOW_USER_INTERFACE),
            (AppIntent::ShowGenerate, name::SHOW_GENERATE),
            (AppIntent::RunDemo, name::RUN_DEMO),
            (AppIntent::RunCheck, name::RUN_CHECK),
            (AppIntent::ExportMermaid, name::EXPORT_MERMAID),
            (AppIntent::About, name::ABOUT),
            (AppIntent::Quit, name::QUIT),
        ];
        for (variant, expected) in pairs {
            let emitted = variant_name(variant);
            assert_eq!(
                emitted, *expected,
                "the derive emits {emitted:?} where `name` says {expected:?}"
            );
        }
    }

    /// The pair table above covers every variant, so adding one without a constant
    /// fails here rather than at the first menu row that needs it.
    #[test]
    fn every_variant_has_a_constant() {
        let src = include_str!("intents.rs");
        let declared = src.matches("#[name = \"").count();
        let constants = src
            .lines()
            .filter(|l| l.trim_start().starts_with("pub const ") && l.contains(": &str = "))
            .count();
        assert_eq!(
            declared, constants,
            "{declared} variants but {constants} name constants"
        );
    }

    fn variant_name(variant: &AppIntent) -> &'static str {
        // `into_intent` consumes, and these are unit variants, so rebuild one.
        match variant {
            AppIntent::NewManifest => AppIntent::NewManifest.into_intent().name,
            AppIntent::OpenManifest => AppIntent::OpenManifest.into_intent().name,
            AppIntent::SaveManifest => AppIntent::SaveManifest.into_intent().name,
            AppIntent::SaveManifestAs => AppIntent::SaveManifestAs.into_intent().name,
            AppIntent::CloseManifest => AppIntent::CloseManifest.into_intent().name,
            AppIntent::OpenQleanyManifest => AppIntent::OpenQleanyManifest.into_intent().name,
            AppIntent::Undo => AppIntent::Undo.into_intent().name,
            AppIntent::Redo => AppIntent::Redo.into_intent().name,
            AppIntent::ToggleTheme => AppIntent::ToggleTheme.into_intent().name,
            AppIntent::ShowHome => AppIntent::ShowHome.into_intent().name,
            AppIntent::ShowProject => AppIntent::ShowProject.into_intent().name,
            AppIntent::ShowEntities => AppIntent::ShowEntities.into_intent().name,
            AppIntent::ShowFeatures => AppIntent::ShowFeatures.into_intent().name,
            AppIntent::ShowUserInterface => AppIntent::ShowUserInterface.into_intent().name,
            AppIntent::ShowGenerate => AppIntent::ShowGenerate.into_intent().name,
            AppIntent::RunDemo => AppIntent::RunDemo.into_intent().name,
            AppIntent::RunCheck => AppIntent::RunCheck.into_intent().name,
            AppIntent::ExportMermaid => AppIntent::ExportMermaid.into_intent().name,
            AppIntent::About => AppIntent::About.into_intent().name,
            AppIntent::Quit => AppIntent::Quit.into_intent().name,
        }
    }
}
