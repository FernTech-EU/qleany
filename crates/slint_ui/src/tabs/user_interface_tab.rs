//! User Interface Tab module
//!
//! This module contains the logic specific to the User Interface tab,
//! including callback handlers for user interface settings management.

use std::sync::Arc;

use crate::app_context::AppContext;
use crate::commands::{user_interface_commands, workspace_commands};
use crate::event_hub_client::EventHubClient;
use crate::{App, AppState, UserInterfaceTabState};
use common::event::{DirectAccessEntity, EntityEvent, HandlingManifestEvent, Origin};
use slint::ComponentHandle;

fn create_new_undo_stack(app: &App, app_context: &Arc<AppContext>) {
    let ctx = Arc::clone(app_context);
    let app_weak = app.as_weak();

    if let Some(app) = app_weak.upgrade() {
        let stack_id = ctx.undo_redo_manager.lock().unwrap().create_new_stack();
        log::info!(
            "New undo stack created for UserInterface with ID: {}",
            stack_id
        );
        app.global::<UserInterfaceTabState>()
            .set_user_interface_undo_stack_id(stack_id as i32);
    }
}

fn delete_undo_stack(app: &App, app_context: &Arc<AppContext>) {
    let ctx = Arc::clone(app_context);
    let app_weak = app.as_weak();

    if let Some(app) = app_weak.upgrade() {
        let stack_id = app
            .global::<UserInterfaceTabState>()
            .get_user_interface_undo_stack_id() as u64;
        let result = ctx.undo_redo_manager.lock().unwrap().delete_stack(stack_id);
        match result {
            Ok(()) => {
                log::info!("Undo stack with ID {} deleted for UserInterface", stack_id);
                app.global::<UserInterfaceTabState>()
                    .set_user_interface_undo_stack_id(-1);
            }
            Err(e) => {
                log::error!("Failed to delete undo stack {}: {}", stack_id, e);
            }
        }
    }
}

/// Fill the UserInterfaceTabState with data from the UserInterface entity
fn fill_user_interface_tab(app: &App, app_context: &Arc<AppContext>) {
    log::info!("Filling UserInterfaceTabState with data from UserInterface entity");

    if let Some(ui_id) = get_user_interface_id(app, app_context)
        && let Ok(Some(ui)) = user_interface_commands::get_user_interface(app_context, &ui_id)
    {
        log::info!("Filling UserInterfaceTabState with UI data: {:?}", ui);
        let state = app.global::<UserInterfaceTabState>();
        state.set_rust_cli(ui.rust_cli);
        state.set_rust_teksilo(ui.rust_teksilo);
        state.set_rust_slint(ui.rust_slint);
        state.set_cpp_qt_qtwidgets(ui.cpp_qt_qtwidgets);
        state.set_cpp_qt_qtquick(ui.cpp_qt_qtquick);
        state.set_rust_ios(ui.rust_ios);
        state.set_rust_android(ui.rust_android);
    }
}

fn clear_user_interface_tab(app: &App) {
    log::info!("Clearing UserInterfaceTabState data");
    let state = app.global::<UserInterfaceTabState>();
    state.set_rust_cli(false);
    state.set_rust_teksilo(false);
    state.set_rust_slint(false);
    state.set_cpp_qt_qtwidgets(false);
    state.set_cpp_qt_qtquick(false);
    state.set_rust_ios(false);
    state.set_rust_android(false);
}

fn subscribe_close_manifest_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(Origin::HandlingManifest(HandlingManifestEvent::Close), {
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |_event| {
            let ctx = Arc::clone(&ctx);
            let app_weak = app_weak.clone();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade() {
                    clear_user_interface_tab(&app);
                    delete_undo_stack(&app, &ctx);
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
        move |_event| {
            let ctx = Arc::clone(&ctx);
            let app_weak = app_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade()
                    && app.global::<AppState>().get_manifest_is_open()
                {
                    fill_user_interface_tab(&app, &ctx);
                    create_new_undo_stack(&app, &ctx);
                }
            });
        }
    });
}

fn subscribe_load_manifest_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(Origin::HandlingManifest(HandlingManifestEvent::Load), {
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |_event| {
            let ctx = Arc::clone(&ctx);
            let app_weak = app_weak.clone();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade()
                    && app.global::<AppState>().get_manifest_is_open()
                {
                    fill_user_interface_tab(&app, &ctx);
                    create_new_undo_stack(&app, &ctx);
                }
            });
        }
    });
}

fn subscribe_ui_created_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(
        Origin::DirectAccess(DirectAccessEntity::UserInterface(EntityEvent::Created)),
        {
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |_event| {
                let ctx = Arc::clone(&ctx);
                let app_weak = app_weak.clone();

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak.upgrade()
                        && app.global::<AppState>().get_manifest_is_open()
                    {
                        fill_user_interface_tab(&app, &ctx);
                    }
                });
            }
        },
    );
}

fn subscribe_ui_updated_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(
        Origin::DirectAccess(DirectAccessEntity::UserInterface(EntityEvent::Updated)),
        {
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |_event| {
                let ctx = Arc::clone(&ctx);
                let app_weak = app_weak.clone();

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak.upgrade()
                        && app.global::<AppState>().get_manifest_is_open()
                    {
                        app.global::<AppState>().set_manifest_is_saved(false);
                        fill_user_interface_tab(&app, &ctx);
                    }
                });
            }
        },
    );
}

/// Helper function to get the user_interface_id from workspace
fn get_user_interface_id(
    app: &App,
    app_context: &Arc<AppContext>,
) -> Option<common::types::EntityId> {
    let workspace_id = app.global::<AppState>().get_workspace_id() as common::types::EntityId;
    if workspace_id > 0
        && let Ok(Some(workspace)) = workspace_commands::get_workspace(app_context, &workspace_id)
        && workspace.user_interface > 0
    {
        return Some(workspace.user_interface);
    }
    None
}

/// Helper function to update a user interface field with new value
fn update_user_interface_helper<F>(app: &App, app_context: &Arc<AppContext>, update_fn: F)
where
    F: FnOnce(&mut direct_access::UserInterfaceDto),
{
    if let Some(ui_id) = get_user_interface_id(app, app_context) {
        let ui_res = user_interface_commands::get_user_interface(app_context, &ui_id);

        if let Ok(Some(mut ui)) = ui_res {
            update_fn(&mut ui);
            match user_interface_commands::update_user_interface(
                app_context,
                Some(
                    app.global::<UserInterfaceTabState>()
                        .get_user_interface_undo_stack_id() as u64,
                ),
                &ui.into(),
            ) {
                Ok(_) => {
                    log::info!("UserInterface updated successfully");
                }
                Err(e) => {
                    log::error!("Failed to update user interface: {}", e);
                }
            }
        }
    }
}

fn setup_rust_cli_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<UserInterfaceTabState>().on_rust_cli_changed({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |new_value| {
            if let Some(app) = app_weak.upgrade() {
                update_user_interface_helper(&app, &ctx, |ui| {
                    ui.rust_cli = new_value;
                });
            }
        }
    });
}

fn setup_rust_teksilo_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<UserInterfaceTabState>()
        .on_rust_teksilo_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |new_value| {
                if let Some(app) = app_weak.upgrade() {
                    update_user_interface_helper(&app, &ctx, |ui| {
                        ui.rust_teksilo = new_value;
                    });
                }
            }
        });
}

fn setup_rust_slint_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<UserInterfaceTabState>()
        .on_rust_slint_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |new_value| {
                if let Some(app) = app_weak.upgrade() {
                    update_user_interface_helper(&app, &ctx, |ui| {
                        ui.rust_slint = new_value;
                    });
                }
            }
        });
}

fn setup_cpp_qt_qtwidgets_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<UserInterfaceTabState>()
        .on_cpp_qt_qtwidgets_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |new_value| {
                if let Some(app) = app_weak.upgrade() {
                    update_user_interface_helper(&app, &ctx, |ui| {
                        ui.cpp_qt_qtwidgets = new_value;
                    });
                }
            }
        });
}

fn setup_cpp_qt_qtquick_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<UserInterfaceTabState>()
        .on_cpp_qt_qtquick_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |new_value| {
                if let Some(app) = app_weak.upgrade() {
                    update_user_interface_helper(&app, &ctx, |ui| {
                        ui.cpp_qt_qtquick = new_value;
                    });
                }
            }
        });
}

fn setup_rust_ios_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<UserInterfaceTabState>().on_rust_ios_changed({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |new_value| {
            if let Some(app) = app_weak.upgrade() {
                update_user_interface_helper(&app, &ctx, |ui| {
                    ui.rust_ios = new_value;
                });
            }
        }
    });
}

fn setup_rust_android_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<UserInterfaceTabState>()
        .on_rust_android_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |new_value| {
                if let Some(app) = app_weak.upgrade() {
                    update_user_interface_helper(&app, &ctx, |ui| {
                        ui.rust_android = new_value;
                    });
                }
            }
        });
}

/// Initialize all user interface tab related callbacks
pub fn init(event_hub_client: &EventHubClient, app: &App, app_context: &Arc<AppContext>) {
    subscribe_ui_created_event(event_hub_client, app, app_context);
    subscribe_ui_updated_event(event_hub_client, app, app_context);
    subscribe_new_manifest_event(event_hub_client, app, app_context);
    subscribe_close_manifest_event(event_hub_client, app, app_context);
    subscribe_load_manifest_event(event_hub_client, app, app_context);
    setup_rust_cli_callback(app, app_context);
    setup_rust_teksilo_callback(app, app_context);
    setup_rust_slint_callback(app, app_context);
    setup_cpp_qt_qtwidgets_callback(app, app_context);
    setup_cpp_qt_qtquick_callback(app, app_context);
    setup_rust_ios_callback(app, app_context);
    setup_rust_android_callback(app, app_context);
}
