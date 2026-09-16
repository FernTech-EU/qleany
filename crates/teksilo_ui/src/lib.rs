//! Teksilo desktop UI for Qleany.
//!
//! Layering, top to bottom:
//!
//! - `frontend`, the Qleany-generated backend facade: `AppContext` plus one command
//!   module per entity and per feature.
//! - [`singles`] and [`models`], also generated: reactive handles that hold one
//!   entity by id, or one relationship as a list, and refresh themselves from the
//!   event hub.
//! - the feature modules, each owning a view-model that holds the screen's state and
//!   exposes its commands as plain methods.
//! - the views, thin functions built with `teksu!` that bind a view-model's signals
//!   and call its methods.
//!
//! A view never calls a command module directly, and a view-model is never called a
//! controller: that word is Qleany's backend tier, below `frontend`.

// Generated Layer A. Machine territory: regenerate, never hand-edit.
pub mod event_source;
pub mod models;
pub mod session;
pub mod singles;
pub mod undo_redo;

// Hand-written.
pub mod app;
pub mod app_ids;
pub mod bootstrap;
pub mod home;
pub mod icons;
pub mod intents;
pub mod manifest;
pub mod project;
pub mod settings_keys;
pub mod shared;
pub mod shell;
pub mod style;

use std::rc::Rc;

use teksilo::prelude::*;

use frontend::commands::handling_app_lifecycle_commands;
use frontend::{AppContext, EventHubClient};

use crate::event_source::QleanyEventSource;
use crate::session::Session;

/// The locales compiled into the binary.
///
/// English only, but through the real Fluent pipeline: `tr!` resolves against these
/// files at compile time, so a key that does not exist is a build error rather than
/// a string that renders as its own name.
const SUPPORTED_LOCALES: &[&str] = &["en-US"];

fn app_locales() -> &'static [(&'static str, &'static [&'static str])] {
    teksilo::i18n::compile_in_locales!(
        base = "../locales/",
        locales = ["en-US"],
        files = ["main.ftl"],
    )
}

/// Build and run the application.
pub fn run() {
    env_logger::init();

    let app_ctx = Rc::new(AppContext::new());

    // Seed Root and System before anything reads them. Every manifest operation
    // resolves through Root, so without this a load fails on an empty store and the
    // app simply never opens anything.
    if let Err(e) = handling_app_lifecycle_commands::initialize_app(&app_ctx) {
        log::error!("could not initialize the application: {e:?}");
        return;
    }

    // Background dispatch thread. It exits when `AppContext::shutdown` drops the
    // shutdown sender.
    let client = EventHubClient::new(&app_ctx.event_hub);
    client.start(app_ctx.shutdown_rx.clone());

    let session = Session::new(app_ctx.clone());
    let ids = crate::app_ids::AppIds::new();
    // `System` outlives every manifest: it is seeded by `initialize_app` and holds
    // the generated files. Resolved once here rather than looked up again by each
    // screen that needs it.
    ids.system_id.set(crate::bootstrap::system_id(&app_ctx));

    let i18n = I18nConfig::new()
        .source_locale("en-US".parse().expect("en-US is a valid locale"))
        .supported_locales(
            SUPPORTED_LOCALES
                .iter()
                .map(|l| l.parse().expect("a supported locale is valid")),
        )
        .compile_in(app_locales())
        // Teksilo's own widget strings: window-control names, built-in icon-button
        // labels, the title bar's accessible name. Without this every one of them
        // logs a missing key and renders as its own identifier, which is also what
        // an automation probe would read instead of the label a user sees.
        .framework_locales(teksilo::widgets::framework_locales())
        // One locale, so there is nothing for OS detection to choose between, and
        // letting it run would only add a way for the app to start in a locale it
        // cannot serve.
        .auto_detect_os_locale(false);

    let root_session = session.clone();
    let root_ids = ids.clone();

    TeksiloAppBuilder::new()
        .application("eu", "ferntech", "Qleany")
        .theme(crate::style::light())
        .i18n(i18n)
        // Registered once for the process, before any window: `ctx.subscribe_event`
        // panics when no source is registered.
        .event_source(QleanyEventSource::new(client))
        // Debug-only, a no-op in release. This is what `scripts/automation_*.py`
        // attach to.
        .install_automation_bridge_in_debug()
        .install_inspector_in_debug()
        .install_file_dialog()
        .install_native_menu()
        .install_toast_default()
        .install_async_async_std()
        .initial_window(
            WindowConfig::new()
                .title("Qleany")
                .size(1280, 820)
                .min_size(960, 640)
                .decorations(DecorationsMode::CustomChrome)
                .root(move |tree, _state| {
                    crate::shell::window::build_root(tree, root_session.clone(), root_ids.clone())
                }),
        )
        .run();

    app_ctx.shutdown();
}
