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
pub mod about;
pub mod app;
pub mod app_ids;
pub mod bootstrap;
pub mod check;
pub mod cli;
pub mod cli_handlers;
pub mod demo;
pub mod edit;
pub mod entities;
pub mod features;
pub mod generate;
pub mod home;
pub mod icons;
pub mod intents;
pub mod manifest;
pub mod new_manifest;
pub mod project;
pub mod settings_keys;
pub mod shared;
pub mod shell;
pub mod style;
pub mod user_interface;

#[cfg(test)]
mod test_support;

use std::rc::Rc;

use teksilo::core::window::CloseResponse;
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

/// The binary's entry point: a subcommand, or the window.
///
/// `initialize_app` runs before the command line is dispatched, not after. Every
/// CLI handler resolves `Root` or `System`, so a subcommand against an unseeded
/// store fails on an empty database rather than doing anything useful. The Slint
/// binary ordered it the same way and for the same reason.
pub fn entry() {
    env_logger::init();

    let app_ctx = Rc::new(AppContext::new());
    if let Err(e) = handling_app_lifecycle_commands::initialize_app(&app_ctx) {
        log::error!("could not initialize the application: {e:?}");
        std::process::exit(1);
    }

    // `None` means no subcommand was given, so this is a GUI launch. Anything
    // else has already run and reported for itself.
    if crate::cli::run_cli(&app_ctx).is_none() {
        cleanup(&app_ctx);
        return;
    }

    run_with(app_ctx.clone());
    cleanup(&app_ctx);
}

/// Close the store down and let the background dispatch thread exit.
fn cleanup(app_ctx: &Rc<AppContext>) {
    if let Err(e) = handling_app_lifecycle_commands::clean_up_before_exit(app_ctx) {
        log::error!("could not clean up: {e:?}");
    }
    app_ctx.shutdown();
}

/// Build and run the application on an already-seeded context.
pub fn run_with(app_ctx: Rc<AppContext>) {
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

    let manifest = crate::manifest::ManifestViewModel::new(app_ctx.clone(), ids.clone());
    let guard_manifest = manifest.clone();
    let root_session = session.clone();
    let root_ids = ids.clone();
    // Whether there is work that is not on disk. Written by `App`, read by the
    // window's close guard below, which is built before any view-model exists.
    let unsaved: Signal<bool> = Signal::new(false);
    let guard_unsaved = unsaved.clone();

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
                // The same key the title reverts to when the manifest is closed,
                // rather than a second copy of the product name.
                .title(tr!(window_title_no_manifest()).resolve_now())
                .size(1280, 820)
                .min_size(960, 640)
                .decorations(DecorationsMode::CustomChrome)
                // The window's own close button goes through the same question the
                // Quit menu row asks. Present directly: a window callback has no
                // input dispatch to drain an intent on Teksilo 0.14.3.
                .on_close_requested(move |ctx| {
                    if !guard_unsaved.get() {
                        return CloseResponse::Close;
                    }
                    crate::manifest::guard::confirm_quit(&guard_manifest, ctx);
                    CloseResponse::Veto
                })
                .root(move |tree, _state| {
                    crate::shell::window::build_root(
                        tree,
                        root_session.clone(),
                        root_ids.clone(),
                        unsaved.clone(),
                        manifest.clone(),
                    )
                }),
        )
        .run();

    app_ctx.shutdown();
}
