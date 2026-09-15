//! Home Tab module
//!
//! This module contains the logic specific to the Home tab,
//! including manifest-related callback handlers (new, open, save, close, open_qleany).

use std::sync::Arc;

use common::event::{HandlingManifestEvent, Origin};
use handling_manifest::LoadDto;
use slint::ComponentHandle;

use crate::app_context::AppContext;
use crate::commands::{handling_manifest_commands, workspace_commands};
use crate::event_hub_client::EventHubClient;
use crate::{App, AppState, ManifestCommands, NewManifestWizardState};
use handling_manifest::{CreateLanguage, ManifestTemplate};
use slint::Timer;

fn subscribe_loaded_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(Origin::HandlingManifest(HandlingManifestEvent::Load), {
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |event| {
            log::info!("Manifest loaded event received: {:?}", event);
            let _ctx = Arc::clone(&ctx);
            let app_weak = app_weak.clone();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade() {
                    // clear any previous error
                    app.global::<AppState>()
                        .set_error_message(slint::SharedString::from(""));
                    // set loading
                    app.global::<AppState>().set_is_loading(true);
                    // set workspace_id
                    app.global::<AppState>()
                        .set_workspace_id(event.ids[0] as i32);

                    app.global::<AppState>().set_manifest_is_saved(true);
                    app.global::<AppState>().set_is_loading(false);
                    app.global::<AppState>().set_manifest_is_open(true);
                    if let Some(data) = event.data {
                        app.global::<AppState>()
                            .set_manifest_path(slint::SharedString::from(data));
                    }
                    log::info!("Manifest UI state updated after load");
                }
            });
        }
    });
}

fn subscribe_closed_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(Origin::HandlingManifest(HandlingManifestEvent::Close), {
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |event| {
            log::info!("Manifest closed event received: {:?}", event);
            let _ctx = Arc::clone(&ctx);
            let app_weak = app_weak.clone();

            // Use invoke_from_event_loop to safely update UI from background thread
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade() {
                    // clear any previous error
                    app.global::<AppState>()
                        .set_error_message(slint::SharedString::from(""));
                    // set workspace_id
                    app.global::<AppState>().set_workspace_id(-1);

                    app.global::<AppState>().set_manifest_is_saved(true);
                    app.global::<AppState>().set_is_loading(false);
                    app.global::<AppState>().set_manifest_is_open(false);
                    app.global::<AppState>()
                        .set_manifest_path(slint::SharedString::from(""));
                }
            });
        }
    });
}

fn subscribe_new_manifest_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(Origin::HandlingManifest(HandlingManifestEvent::Create), {
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |event| {
            log::info!("New manifest created event received");
            let _ctx = Arc::clone(&ctx);
            let app_weak = app_weak.clone();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade() {
                    // clear any previous error
                    app.global::<AppState>()
                        .set_error_message(slint::SharedString::from(""));
                    // set workspace_id
                    app.global::<AppState>()
                        .set_workspace_id(event.ids[0] as i32);

                    app.global::<AppState>().set_manifest_is_saved(true);
                    app.global::<AppState>().set_is_loading(false);
                    app.global::<AppState>().set_manifest_is_open(true);
                    app.global::<AppState>()
                        .set_manifest_path(slint::SharedString::from(""));
                }
            });
        }
    });
}

/// Wire up the on_new_manifest callback — opens the wizard
pub fn setup_new_manifest_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<ManifestCommands>().on_new_manifest({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move || {
            log::info!("New Manifest clicked — opening wizard");

            if let Some(app) = app_weak.upgrade() {
                if app.global::<AppState>().get_manifest_is_open() {
                    log::info!("A manifest is already open, closing it first");
                    match handling_manifest_commands::close(&ctx) {
                        Ok(()) => {
                            log::info!("Manifest closed successfully");
                        }
                        Err(e) => {
                            log::error!("Failed to close manifest: {}", e);
                            return;
                        }
                    }
                }

                // Reset wizard state and show it
                let wiz = app.global::<NewManifestWizardState>();
                wiz.set_step(0);
                wiz.set_language_index(0);
                wiz.set_application_name(slint::SharedString::from(""));
                wiz.set_organization_name(slint::SharedString::from(""));
                wiz.set_template_index(0);
                wiz.set_opt_rust_cli(false);
                wiz.set_opt_rust_teksilo(true);
                wiz.set_opt_rust_slint(false);
                wiz.set_opt_cpp_qt_qtwidgets(false);
                wiz.set_opt_cpp_qt_qtquick(true);
                wiz.set_name_error(slint::SharedString::from(""));
                wiz.set_org_error(slint::SharedString::from(""));
                wiz.set_visible(true);
            }
        }
    });
}

/// Wire up the create_wizard_confirmed callback — save dialog + create + load
pub fn setup_create_wizard_confirmed_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<ManifestCommands>()
        .on_create_wizard_confirmed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move || {
                log::info!("Wizard confirmed — proceeding to create manifest");

                let Some(app) = app_weak.upgrade() else {
                    return;
                };

                // Read wizard state
                let wiz = app.global::<NewManifestWizardState>();
                let language = if wiz.get_language_index() == 0 {
                    CreateLanguage::CppQt
                } else {
                    CreateLanguage::Rust
                };
                let application_name = wiz.get_application_name().to_string();
                let organization_name = wiz.get_organization_name().to_string();
                let manifest_template = match wiz.get_template_index() {
                    1 => ManifestTemplate::Minimal,
                    2 => ManifestTemplate::DocumentEditor,
                    3 => ManifestTemplate::DataManagement,
                    _ => ManifestTemplate::Blank,
                };

                let mut options = Vec::new();
                if wiz.get_opt_rust_cli() {
                    options.push("rust_cli".to_string());
                }
                if wiz.get_opt_rust_teksilo() {
                    options.push("rust_teksilo".to_string());
                }
                if wiz.get_opt_rust_slint() {
                    options.push("rust_slint".to_string());
                }
                if wiz.get_opt_cpp_qt_qtwidgets() {
                    options.push("cpp_qt_qtwidgets".to_string());
                }
                if wiz.get_opt_cpp_qt_qtquick() {
                    options.push("cpp_qt_qtquick".to_string());
                }

                // Step 1: Save file dialog
                let default_path = dirs::home_dir().unwrap_or_default();
                let file_dialog = rfd::FileDialog::new()
                    .add_filter("YAML files", &["yaml", "yml"])
                    .set_directory(&default_path)
                    .set_file_name("qleany.yaml");

                let path = match file_dialog.save_file() {
                    Some(path) => path,
                    None => {
                        log::info!("Save file dialog cancelled");
                        return;
                    }
                };

                let manifest_path = path.to_string_lossy().to_string();
                log::info!("Selected save path: {}", manifest_path);

                app.global::<AppState>().set_is_loading(true);

                // Step 2: Create manifest via use case
                let create_dto = handling_manifest::CreateDto {
                    manifest_path: manifest_path.clone(),
                    language,
                    application_name,
                    organization_name,
                    manifest_template,
                    options,
                };
                match handling_manifest_commands::create(&ctx, &create_dto) {
                    Ok(_result) => {
                        log::info!("New manifest created successfully at {}", manifest_path);
                    }
                    Err(e) => {
                        log::error!("Failed to create new manifest: {}", e);
                        app.global::<AppState>().set_is_loading(false);
                        app.global::<AppState>()
                            .set_error_message(slint::SharedString::from(e));
                        return;
                    }
                }

                // Step 3: Open the newly created manifest
                let load_dto = LoadDto {
                    manifest_path: manifest_path.clone(),
                };
                match handling_manifest_commands::load(&ctx, &load_dto) {
                    Ok(result) => {
                        log::info!("Manifest loaded successfully: {:?}", result);
                        if let Some(app) = app_weak.upgrade() {
                            app.global::<AppState>()
                                .set_error_message(slint::SharedString::from(""));
                            app.global::<AppState>().set_manifest_is_saved(true);
                            app.global::<AppState>()
                                .set_manifest_path(slint::SharedString::from(manifest_path));

                            app.global::<AppState>().set_success_message(
                                slint::SharedString::from(
                                    "Manifest created and loaded successfully",
                                ),
                            );
                            app.global::<AppState>().set_success_message_visible(true);

                            let app_weak_timer = app.as_weak();
                            Timer::single_shot(std::time::Duration::from_secs(3), move || {
                                if let Some(app) = app_weak_timer.upgrade() {
                                    app.global::<AppState>().set_success_message_visible(false);
                                }
                            });
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to load created manifest: {}", e);
                        app.global::<AppState>().set_is_loading(false);
                        if let Some(app) = app_weak.upgrade() {
                            app.global::<AppState>()
                                .set_error_message(slint::SharedString::from(e));
                        }
                    }
                }
            }
        });
}

/// Wire up the on_open_manifest callback
pub fn setup_open_manifest_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<ManifestCommands>().on_open_manifest({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move || {
            log::info!("Open Manifest clicked");

            // Get the user's home directory as the default path
            let default_path = dirs::home_dir().unwrap_or_default();

            // Open file dialog using rfd
            let file_dialog = rfd::FileDialog::new()
                .add_filter("YAML files", &["yaml", "yml"])
                .set_directory(&default_path)
                .set_file_name("qleany.yaml");

            if let Some(app) = app_weak.upgrade() {
                if let Some(path) = file_dialog.pick_file() {
                    let manifest_path = path.to_string_lossy().to_string();
                    log::info!("Selected manifest file: {}", manifest_path);

                    if app.global::<AppState>().get_manifest_is_open() {
                        log::info!("A manifest is already open, closing it first");

                        // Close any currently open manifest first
                        match handling_manifest_commands::close(&ctx) {
                            Ok(()) => {
                                log::info!("Manifest closed successfully");
                            }
                            Err(e) => {
                                log::error!("Failed to close manifest: {}", e);
                                return;
                            }
                        }
                    }
                    let load_dto = LoadDto { manifest_path };
                    match handling_manifest_commands::load(&ctx, &load_dto) {
                        Ok(result) => {
                            log::info!("Manifest loaded successfully: {:?}", result);
                            if let Some(app) = app_weak.upgrade() {
                                app.global::<AppState>().set_manifest_path(
                                    slint::SharedString::from(load_dto.manifest_path),
                                );
                            }
                        }

                        Err(e) => {
                            log::error!("Failed to load manifest: {}", e);
                            if let Some(app) = app_weak.upgrade() {
                                app.global::<AppState>().set_is_loading(false);
                                app.global::<AppState>()
                                    .set_error_message(slint::SharedString::from(e));
                            }
                        }
                    }
                }
            } else {
                log::info!("File dialog cancelled");
            }
        }
    });
}

/// Wire up the on_save_manifest callback
pub fn setup_save_manifest_as_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<ManifestCommands>().on_save_manifest_as({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move || {
            log::info!("Save Manifest As clicked");

            // Get the user's home directory as the default path
            let default_path = dirs::home_dir().unwrap_or_default();

            // Open save file dialog using rfd
            let file_dialog = rfd::FileDialog::new()
                .add_filter("YAML files", &["yaml", "yml"])
                .set_directory(&default_path)
                .set_file_name("qleany.yaml");

            if let Some(path) = file_dialog.save_file() {
                let manifest_path = path.to_string_lossy().to_string();
                log::info!("Selected save path: {}", manifest_path);

                let save_dto = handling_manifest::SaveDto { manifest_path };
                match handling_manifest_commands::save(&ctx, &save_dto) {
                    Ok(()) => {
                        log::info!("Manifest saved successfully");
                        if let Some(app) = app_weak.upgrade() {
                            app.global::<AppState>()
                                .set_error_message(slint::SharedString::from(""));
                            app.global::<AppState>().set_manifest_is_saved(true);
                            app.global::<AppState>()
                                .set_manifest_path(slint::SharedString::from(
                                    save_dto.manifest_path,
                                ));

                            // Show success message
                            app.global::<AppState>().set_success_message(
                                slint::SharedString::from("Manifest saved successfully"),
                            );
                            app.global::<AppState>().set_success_message_visible(true);

                            // Hide after 3 seconds
                            let app_weak_timer = app.as_weak();
                            Timer::single_shot(std::time::Duration::from_secs(3), move || {
                                if let Some(app) = app_weak_timer.upgrade() {
                                    app.global::<AppState>().set_success_message_visible(false);
                                }
                            });
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to save manifest: {}", e);
                        if let Some(app) = app_weak.upgrade() {
                            app.global::<AppState>()
                                .set_error_message(slint::SharedString::from(e));
                        }
                    }
                }
            } else {
                log::info!("Save file dialog cancelled");
            }
        }
    });
}

/// Wire up the on_save_manifest callback
pub fn setup_save_manifest_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<ManifestCommands>().on_save_manifest({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move || {
            log::info!("Save Manifest clicked");

            let workspace_id = app_weak
                .upgrade()
                .map_or(0, |app| app.global::<AppState>().get_workspace_id() as u64);
            if workspace_id < 1 {
                log::error!("No manifest is currently loaded, cannot save");
                return;
            }
            let workspace =
                workspace_commands::get_workspace(&ctx, &workspace_id as &common::types::EntityId)
                    .ok()
                    .flatten();
            if workspace.is_none() {
                log::error!("Failed to get workspace entity for id {}", workspace_id);
                return;
            }
            let manifest_absolute_path = workspace.unwrap().manifest_absolute_path;
            log::info!("Saving manifest to path: {}", manifest_absolute_path);

            // add "qleany.yaml" if manifest_absolute_path is a directory
            let manifest_absolute_path = {
                let path = std::path::Path::new(&manifest_absolute_path);
                if path.is_dir() {
                    path.join("qleany.yaml").to_string_lossy().to_string()
                } else {
                    manifest_absolute_path
                }
            };

            let save_dto = handling_manifest::SaveDto {
                manifest_path: manifest_absolute_path,
            };
            match handling_manifest_commands::save(&ctx, &save_dto) {
                Ok(()) => {
                    log::info!("Manifest saved successfully");
                    if let Some(app) = app_weak.upgrade() {
                        app.global::<AppState>()
                            .set_error_message(slint::SharedString::from(""));
                        app.global::<AppState>().set_manifest_is_saved(true);
                        app.global::<AppState>()
                            .set_manifest_path(slint::SharedString::from(save_dto.manifest_path));

                        // Show success message
                        app.global::<AppState>()
                            .set_success_message(slint::SharedString::from(
                                "Manifest saved successfully",
                            ));
                        app.global::<AppState>().set_success_message_visible(true);

                        // Hide after 3 seconds
                        let app_weak_timer = app.as_weak();
                        Timer::single_shot(std::time::Duration::from_secs(3), move || {
                            if let Some(app) = app_weak_timer.upgrade() {
                                app.global::<AppState>().set_success_message_visible(false);
                            }
                        });
                    }
                }
                Err(e) => {
                    log::error!("Failed to save manifest: {}", e);
                    if let Some(app) = app_weak.upgrade() {
                        app.global::<AppState>()
                            .set_error_message(slint::SharedString::from(e));
                    }
                }
            }
        }
    });
}

/// Wire up the on_close_manifest callback
pub fn setup_close_manifest_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<ManifestCommands>().on_close_manifest({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move || {
            log::info!("Close Manifest clicked");
            match handling_manifest_commands::close(&ctx) {
                Ok(()) => {
                    log::info!("Manifest closed successfully");
                    if let Some(app) = app_weak.upgrade() {
                        app.global::<AppState>().set_manifest_is_saved(true);
                        app.global::<AppState>().set_manifest_is_open(false);
                    }
                }
                Err(e) => {
                    log::error!("Failed to close manifest: {}", e);
                }
            }
        }
    });
}

/// Wire up the on_open_qleany_manifest callback
pub fn setup_open_qleany_manifest_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<ManifestCommands>().on_open_qleany_manifest({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move || {
            log::info!("Open Qleany Manifest clicked");
            if let Some(app) = app_weak.upgrade() {
                if app.global::<AppState>().get_manifest_is_open() {
                    log::info!("A manifest is already open, closing it first");

                    // Close any currently open manifest first
                    match handling_manifest_commands::close(&ctx) {
                        Ok(()) => {
                            log::info!("Manifest closed successfully");
                        }
                        Err(e) => {
                            log::error!("Failed to close manifest: {}", e);
                        }
                    }
                }

                // Load the qleany.yaml from the project workspace
                let load_dto = LoadDto {
                    manifest_path: "qleany.yaml".to_string(),
                };
                match handling_manifest_commands::load(&ctx, &load_dto) {
                    Ok(result) => {
                        log::info!("Qleany manifest loaded: {:?}", result);
                        // clear any previous error
                        app.global::<AppState>()
                            .set_error_message(slint::SharedString::from(""));

                        // set workspace_id
                        app.global::<AppState>()
                            .set_workspace_id(result.workspace_id as i32);

                        // set loading
                        app.global::<AppState>().set_is_loading(true);
                        Timer::single_shot(std::time::Duration::from_millis(100), move || {
                            app.global::<AppState>().set_manifest_is_saved(true);
                            app.global::<AppState>().set_is_loading(false);
                            app.global::<AppState>().set_manifest_is_open(true);
                            app.global::<AppState>()
                                .set_manifest_path(slint::SharedString::from("qleany.yaml"));
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to load qleany manifest: {}", e);
                        if let Some(app) = app_weak.upgrade() {
                            app.global::<AppState>()
                                .set_error_message(slint::SharedString::from(e));
                        }
                    }
                }
            }
        }
    });
}

/// Initialize all home tab related callbacks
pub fn init(event_hub_client: &EventHubClient, app: &App, app_context: &Arc<AppContext>) {
    subscribe_loaded_event(event_hub_client, app, app_context);
    subscribe_closed_event(event_hub_client, app, app_context);
    subscribe_new_manifest_event(event_hub_client, app, app_context);
    setup_new_manifest_callback(app, app_context);
    setup_create_wizard_confirmed_callback(app, app_context);
    setup_open_manifest_callback(app, app_context);
    setup_save_manifest_callback(app, app_context);
    setup_save_manifest_as_callback(app, app_context);
    setup_close_manifest_callback(app, app_context);
    setup_open_qleany_manifest_callback(app, app_context);
}
