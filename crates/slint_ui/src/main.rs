mod app_context;
mod cli;
mod cli_handlers;
mod commands;
mod common_tools;
mod event_hub_client;
mod tabs;

use crate::cli::run_cli;
use crate::commands::handling_manifest_commands;
use app_context::AppContext;
use event_hub_client::EventHubClient;
use std::sync::Arc;

slint::include_modules!();

fn main() {
    // Initialize logging
    env_logger::init();

    // Create the application context (backend state)
    let app_context = Arc::new(AppContext::new());

    // Initialize the application (e.g. prepare database, load settings)
    match commands::handling_app_lifecycle_commands::initialize_app(&app_context) {
        Ok(()) => {
            log::info!("Application initialized successfully");
        }
        Err(e) => {
            log::error!("Failed to initialize application: {}", e);
            return;
        }
    }

    if let Some(_args) = run_cli(&app_context) {
        run_slint(&app_context);
    }

    // Cleanup on exit
    log::info!("Shutting down");
    match commands::handling_app_lifecycle_commands::clean_up_before_exit(&app_context) {
        Ok(()) => {
            log::info!("Application cleaned up successfully");
        }
        Err(e) => {
            log::error!("Failed to clean up application: {}", e);
        }
    }
    app_context.shutdown();
}

fn run_slint(app_context: &Arc<AppContext>) {
    log::info!("Starting Qleany Slint UI");
    // Create the event hub client for backend-to-UI event passing
    let event_hub_client = EventHubClient::new(&app_context.event_hub);
    event_hub_client.start(app_context.shutdown_rx.clone());

    // Create the Slint UI
    let app = App::new().unwrap();

    // Initialize common tools callbacks
    common_tools::init(&app, app_context);

    // Initialize home tab callbacks (manifest operations)
    tabs::home_tab::init(&event_hub_client, &app, app_context);

    // Initialize demo wizard callbacks
    tabs::demo_wizard::init(&app, app_context);

    // Initialize entities tab subscriptions and callbacks
    tabs::entities_tab::init(&event_hub_client, &app, app_context);

    // Initialize features tab subscriptions and callbacks
    tabs::features::init(&event_hub_client, &app, app_context);

    // Initialize project tab callbacks (project settings)
    tabs::project_tab::init(&event_hub_client, &app, app_context);

    // Initialize user interface tab callbacks
    tabs::user_interface_tab::init(&event_hub_client, &app, app_context);

    // Initialize generate tab callbacks (rust file generation)
    tabs::generate_tab::init(&event_hub_client, &app, app_context);

    // Initialize check widget (debounced manifest validation)
    tabs::check_widget::init(&event_hub_client, &app, app_context);

    app.window().on_close_requested({
        let app_weak = app.as_weak();
        let ctx = Arc::clone(app_context);

        move || {
            log::info!("Window close requested");

            let app = match app_weak.upgrade() {
                Some(app) => app,
                None => return slint::CloseRequestResponse::KeepWindowShown,
            };

            if app.global::<AppState>().get_manifest_is_saved()
                || app.global::<AppState>().get_force_exit()
            {
                if app.global::<AppState>().get_manifest_is_open() {
                    log::info!("Closing Manifest before exit");
                    match handling_manifest_commands::close(&ctx) {
                        Ok(()) => {
                            log::info!("Manifest closed successfully");
                        }
                        Err(e) => {
                            log::error!("Failed to close manifest: {}", e);
                        }
                    }
                }
                ctx.shutdown();
                return slint::CloseRequestResponse::HideWindow;
            }
            app.global::<AppState>()
                .set_confirm_dialog_pending_action(slint::SharedString::from("exit"));
            app.global::<AppState>()
                .set_confirm_dialog_message(slint::SharedString::from(
                    "You have unsaved changes. Do you want to close the manifest without saving?",
                ));
            app.global::<AppState>().set_confirm_dialog_visible(true);

            slint::CloseRequestResponse::KeepWindowShown
        }
    });

    app.global::<ManifestCommands>().on_exit_app({
        let app_weak = app.as_weak();
        move || {
            let app = match app_weak.upgrade() {
                Some(app) => app,
                None => return,
            };
            slint::Window::try_dispatch_event(
                app.window(),
                slint::platform::WindowEvent::CloseRequested,
            )
            .expect("Failed to dispatch close requested event");
        }
    });

    // Wire up UndoRedoCommands callbacks
    app.global::<UndoRedoCommands>().on_undo({
        let ctx = Arc::clone(app_context);
        move || {
            log::info!("Undo clicked");
            // TODO: Implement undo using undo_redo_commands
            let _ = ctx;
        }
    });

    app.global::<UndoRedoCommands>().on_redo({
        let ctx = Arc::clone(app_context);
        move || {
            log::info!("Redo clicked");
            // TODO: Implement redo using undo_redo_commands
            let _ = ctx;
        }
    });

    // Run the application
    log::info!("Running Slint UI");
    app.run().unwrap();
}
