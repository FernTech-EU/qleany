//! Entities Tab module
//!
//! This module contains the logic specific to the Entities tab,
//! including event subscriptions and callback handlers for entity management.

use crate::app_context::AppContext;
use crate::commands::{
    entity_commands, field_commands, handling_manifest_commands, workspace_commands,
};
use crate::event_hub_client::EventHubClient;
use crate::{App, AppState, EntitiesTabState, ListItem};
use common::direct_access::entity::EntityRelationshipField;
use common::direct_access::workspace::WorkspaceRelationshipField;
use common::entities::{FieldRelationshipType, FieldType};
use common::event::{DirectAccessEntity, EntityEvent, HandlingManifestEvent, Origin};
use common::types::EntityId;
use direct_access::WorkspaceRelationshipDto;
use direct_access::{EntityRelationshipDto, UpdateEntityDto};
use slint::{ComponentHandle, Model, Timer};
use std::sync::Arc;

fn create_new_undo_stack(app: &App, app_context: &Arc<AppContext>) {
    let ctx = Arc::clone(app_context);
    let app_weak = app.as_weak();

    if let Some(app) = app_weak.upgrade() {
        let stack_id = ctx.undo_redo_manager.lock().unwrap().create_new_stack();
        log::info!("New undo stack created with ID: {}", stack_id);
        app.global::<EntitiesTabState>()
            .set_entities_undo_stack_id(stack_id as i32);
    }
}

fn delete_undo_stack(app: &App, app_context: &Arc<AppContext>) {
    let ctx = Arc::clone(app_context);
    let app_weak = app.as_weak();

    if let Some(app) = app_weak.upgrade() {
        let stack_id = app
            .global::<EntitiesTabState>()
            .get_entities_undo_stack_id() as u64;
        let result = ctx.undo_redo_manager.lock().unwrap().delete_stack(stack_id);
        match result {
            Ok(()) => {
                log::info!("Undo stack with ID {} deleted", stack_id);
                app.global::<EntitiesTabState>()
                    .set_entities_undo_stack_id(-1);
            }
            Err(e) => {
                log::error!("Failed to delete undo stack {}: {}", stack_id, e);
            }
        }
    }
}

fn subscribe_close_manifest_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(Origin::HandlingManifest(HandlingManifestEvent::Close), {
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |event| {
            log::info!("Manifest closed event received: {:?}", event);
            let ctx = Arc::clone(&ctx);
            let app_weak = app_weak.clone();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade() {
                    clear_entity_list(&app, &ctx);
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
            log::info!("New manifest created event received");
            let ctx = Arc::clone(&ctx);
            let app_weak = app_weak.clone();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade()
                    && app.global::<AppState>().get_manifest_is_open()
                {
                    fill_entity_list(&app, &ctx);
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
            log::info!("Manifest loaded event received");
            let ctx = Arc::clone(&ctx);
            let app_weak = app_weak.clone();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade()
                    && app.global::<AppState>().get_manifest_is_open()
                {
                    fill_entity_list(&app, &ctx);
                    create_new_undo_stack(&app, &ctx);
                }
            });
        }
    });
}

/// Subscribe to Workspace update events to refresh entity_cr_list
fn subscribe_workspace_updated_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(
        Origin::DirectAccess(DirectAccessEntity::Workspace(EntityEvent::Updated)),
        {
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |event| {
                log::info!("Workspace updated event received: {:?}", event);
                let ctx = Arc::clone(&ctx);
                let app_weak = app_weak.clone();

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak.upgrade()
                        && app.global::<AppState>().get_manifest_is_open()
                    {
                        fill_entity_list(&app, &ctx);
                        fill_entity_options(&app, &ctx);
                        app.global::<AppState>().set_manifest_is_saved(false);
                    }
                });
            }
        },
    );
}

/// Subscribe to Entity update events to refresh entity_cr_list
fn subscribe_entity_updated_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(
        Origin::DirectAccess(DirectAccessEntity::Entity(EntityEvent::Updated)),
        {
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |event| {
                log::info!("Entity updated event received: {:?}", event);
                let ctx = Arc::clone(&ctx);
                let app_weak = app_weak.clone();

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak.upgrade()
                        && app.global::<AppState>().get_manifest_is_open()
                    {
                        fill_entity_list(&app, &ctx);
                        fill_entity_options(&app, &ctx);
                        fill_field_list(&app, &ctx);
                        app.global::<AppState>().set_manifest_is_saved(false);
                    }
                });
            }
        },
    )
}

/// Subscribe to Entity deletion events
fn subscribe_entity_deleted_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(
        Origin::DirectAccess(DirectAccessEntity::Entity(EntityEvent::Removed)),
        {
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |event| {
                log::info!("Entity updated event received: {:?}", event);
                let _ctx = Arc::clone(&ctx);
                let app_weak = app_weak.clone();

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak.upgrade()
                        && app.global::<AppState>().get_manifest_is_open()
                    {
                        app.global::<AppState>().set_manifest_is_saved(false);
                    }
                });
            }
        },
    )
}

/// Subscribe to Entity update events to refresh entity_cr_list
fn subscribe_field_updated_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(
        Origin::DirectAccess(DirectAccessEntity::Field(EntityEvent::Updated)),
        {
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |event| {
                log::info!("Field updated event received: {:?}", event);
                let ctx = Arc::clone(&ctx);
                let app_weak = app_weak.clone();

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak.upgrade()
                        && app.global::<AppState>().get_manifest_is_open()
                    {
                        fill_field_list(&app, &ctx);
                        app.global::<AppState>().set_manifest_is_saved(false);
                    }
                });
            }
        },
    )
}

/// Subscribe to Entity deletion events
fn subscribe_field_deleted_event(
    event_hub_client: &EventHubClient,
    app: &App,
    app_context: &Arc<AppContext>,
) {
    event_hub_client.subscribe(
        Origin::DirectAccess(DirectAccessEntity::Field(EntityEvent::Removed)),
        {
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |event| {
                log::info!("Field updated event received: {:?}", event);
                let _ctx = Arc::clone(&ctx);
                let app_weak = app_weak.clone();

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak.upgrade()
                        && app.global::<AppState>().get_manifest_is_open()
                    {
                        app.global::<AppState>().set_manifest_is_saved(false);
                    }
                });
            }
        },
    )
}

fn fill_entity_list(app: &App, app_context: &Arc<AppContext>) {
    log::info!("Filling entity list...");
    let ctx = Arc::clone(app_context);
    let app_weak = app.as_weak();

    if let Some(app) = app_weak.upgrade() {
        let workspace_id = app.global::<AppState>().get_workspace_id() as common::types::EntityId;

        // Only refresh if we have a valid workspace_id
        if workspace_id > 0 {
            // Get entities attached to the workspace
            let entity_ids_res = workspace_commands::get_workspace_relationship(
                &ctx,
                &workspace_id,
                &WorkspaceRelationshipField::Entities,
            );

            match entity_ids_res {
                Ok(entity_ids) => {
                    // empty entity list if no entities
                    if entity_ids.is_empty() {
                        let model = std::rc::Rc::new(slint::VecModel::from(Vec::<ListItem>::new()));
                        app.global::<EntitiesTabState>()
                            .set_entity_cr_list(model.into());
                        log::info!("Entity list cleared (no entities)");
                        return;
                    }

                    // Fetch entities details to obtain names
                    match entity_commands::get_entity_multi(&ctx, &entity_ids) {
                        Ok(entities_opt) => {
                            // Build a name lookup for inherits_from resolution
                            let entities: Vec<_> = entities_opt.into_iter().flatten().collect();
                            let name_of = |id: common::types::EntityId| -> Option<String> {
                                entities.iter().find(|e| e.id == id).map(|e| e.name.clone())
                            };

                            // Map to ListItem (id + text)
                            let mut list: Vec<ListItem> = Vec::new();
                            for e in &entities {
                                let mut flags: Vec<String> = Vec::new();
                                if e.only_for_heritage {
                                    flags.push("abstract".into());
                                }
                                if let Some(parent_id) = e.inherits_from {
                                    let parent = name_of(parent_id).unwrap_or("?".into());
                                    flags.push(format!("extends {}", parent));
                                }
                                let subtitle = flags.join(" · ");
                                list.push(ListItem {
                                    id: e.id as i32,
                                    text: slint::SharedString::from(e.name.clone()),
                                    subtitle: slint::SharedString::from(subtitle),
                                    checked: false,
                                    gradient_color: slint::Color::default(),
                                });
                            }

                            // Apply to AppState
                            let model = std::rc::Rc::new(slint::VecModel::from(list));
                            app.global::<EntitiesTabState>()
                                .set_entity_cr_list(model.into());
                            log::info!("Entity list refreshed");
                        }
                        Err(e) => {
                            log::error!("Failed to fetch entities: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to get workspace entities: {}", e);
                }
            }
        }
    }
}

fn clear_entity_list(app: &App, app_context: &Arc<AppContext>) {
    let _ctx = Arc::clone(app_context);
    let app_weak = app.as_weak();

    if let Some(app) = app_weak.upgrade() {
        // Clear entity list
        let model = std::rc::Rc::new(slint::VecModel::from(Vec::<ListItem>::new()));
        app.global::<EntitiesTabState>()
            .set_entity_cr_list(model.into());
        log::info!("Entity list cleared");
    }
}

fn fill_field_list(app: &App, app_context: &Arc<AppContext>) {
    let ctx = Arc::clone(app_context);
    let app_weak = app.as_weak();

    if let Some(app) = app_weak.upgrade() {
        let entity_id =
            app.global::<EntitiesTabState>().get_selected_entity_id() as common::types::EntityId;

        // Only refresh if we have a valid workspace_id
        if entity_id > 0 {
            // Get entities attached to the workspace
            let field_ids_res = entity_commands::get_entity_relationship(
                &ctx,
                &entity_id,
                &EntityRelationshipField::Fields,
            );

            match field_ids_res {
                Ok(field_ids) => {
                    // empty field list if no fields
                    if field_ids.is_empty() {
                        let model = std::rc::Rc::new(slint::VecModel::from(Vec::<ListItem>::new()));
                        app.global::<EntitiesTabState>()
                            .set_field_cr_list(model.into());
                        log::info!("Field list cleared (no fields)");
                        return;
                    }

                    // Fetch entities details to obtain names
                    match field_commands::get_field_multi(&ctx, &field_ids) {
                        Ok(fields_opt) => {
                            // Map to ListItem (id + text)
                            let mut list: Vec<ListItem> = Vec::new();
                            for e in fields_opt.into_iter().flatten() {
                                let opt = if e.optional { " (opt)" } else { "" };
                                let list_flag = if e.is_list { " (list)" } else { "" };
                                let subtitle = if e.field_type == FieldType::Entity {
                                    format!(
                                        "{} ({}){}",
                                        field_type_to_string(&e.field_type),
                                        field_relationship_type_to_string(&e.relationship),
                                        opt
                                    )
                                } else if e.field_type == FieldType::Enum {
                                    format!(
                                        "{}: {}{}{}",
                                        field_type_to_string(&e.field_type),
                                        e.enum_name.as_deref().unwrap_or("?"),
                                        opt,
                                        list_flag
                                    )
                                } else {
                                    format!(
                                        "{}{}{}",
                                        field_type_to_string(&e.field_type),
                                        opt,
                                        list_flag
                                    )
                                };
                                list.push(ListItem {
                                    id: e.id as i32,
                                    text: slint::SharedString::from(e.name),
                                    subtitle: slint::SharedString::from(subtitle),
                                    checked: false,
                                    gradient_color: slint::Color::default(),
                                });
                            }

                            // Apply to AppState
                            let model = std::rc::Rc::new(slint::VecModel::from(list));
                            app.global::<EntitiesTabState>()
                                .set_field_cr_list(model.into());
                            log::info!("Field list refreshed");
                        }
                        Err(e) => {
                            log::error!("Failed to fetch fields: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to get workspace fields: {}", e);
                }
            }
        }
    }
}

fn clear_field_list(app: &App, app_context: &Arc<AppContext>) {
    let _ctx = Arc::clone(app_context);
    let app_weak = app.as_weak();

    if let Some(app) = app_weak.upgrade() {
        // Clear field list
        let model = std::rc::Rc::new(slint::VecModel::from(Vec::<ListItem>::new()));
        app.global::<EntitiesTabState>()
            .set_field_cr_list(model.into());
        log::info!("Field list cleared");
    }
}

/// Wire up the on_request_entities_reorder callback on AppState
fn setup_entities_reorder_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_request_entities_reorder({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |from_index, to_index| {
                let from = from_index as usize;
                let to = to_index as usize;

                if let Some(app) = app_weak.upgrade() {
                    // 1) Get entities attached to the workspace

                    let workspace_id =
                        app.global::<AppState>().get_workspace_id() as common::types::EntityId;
                    let entity_ids_res = workspace_commands::get_workspace_relationship(
                        &ctx,
                        &workspace_id,
                        &WorkspaceRelationshipField::Entities,
                    );
                    let mut entity_ids = entity_ids_res.unwrap_or_default();

                    if from == to || from >= entity_ids.len() {
                        return;
                    }

                    let moving_entity_id = entity_ids.remove(from);
                    // Adjust target slot when moving downwards because removing shifts indices left
                    let mut insert_at = if to > from { to - 1 } else { to };
                    if insert_at > entity_ids.len() {
                        insert_at = entity_ids.len();
                    }
                    entity_ids.insert(insert_at, moving_entity_id);

                    let result = workspace_commands::set_workspace_relationship(
                        &ctx,
                        Some(
                            app.global::<EntitiesTabState>()
                                .get_entities_undo_stack_id() as u64,
                        ),
                        &WorkspaceRelationshipDto {
                            id: workspace_id,
                            field: WorkspaceRelationshipField::Entities,
                            right_ids: entity_ids,
                        },
                    );

                    match result {
                        Ok(()) => {
                            log::info!("Entities reordered successfully");
                        }
                        Err(e) => {
                            log::error!("Failed to reorder entities: {}", e);
                        }
                    }
                }
            }
        });
}

fn setup_fields_reorder_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_request_fields_reorder({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |from_index, to_index| {
            let from = from_index as usize;
            let to = to_index as usize;

            if let Some(app) = app_weak.upgrade() {
                // 1) Get fields attached to the entity

                let entity_id = app.global::<EntitiesTabState>().get_selected_entity_id()
                    as common::types::EntityId;
                let field_ids_res = entity_commands::get_entity_relationship(
                    &ctx,
                    &entity_id,
                    &EntityRelationshipField::Fields,
                );
                let mut field_ids = field_ids_res.unwrap_or_default();

                if from == to || from >= field_ids.len() {
                    return;
                }

                let moving_field_id = field_ids.remove(from);
                // Adjust target slot when moving downwards because removing shifts indices left
                let mut insert_at = if to > from { to - 1 } else { to };
                if insert_at > field_ids.len() {
                    insert_at = field_ids.len();
                }
                field_ids.insert(insert_at, moving_field_id);
                let result = entity_commands::set_entity_relationship(
                    &ctx,
                    Some(
                        app.global::<EntitiesTabState>()
                            .get_entities_undo_stack_id() as u64,
                    ),
                    &EntityRelationshipDto {
                        id: entity_id,
                        field: EntityRelationshipField::Fields,
                        right_ids: field_ids,
                    },
                );
                match result {
                    Ok(()) => {
                        log::info!("Fields reordered successfully");
                    }
                    Err(e) => {
                        log::error!("Failed to reorder fields: {}", e);
                    }
                }
            }
        }
    });
}

fn setup_field_deletion_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_request_field_deletion({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |field_id| {
            if let Some(app) = app_weak.upgrade() {
                let result = field_commands::remove_field(
                    &ctx,
                    Some(
                        app.global::<EntitiesTabState>()
                            .get_entities_undo_stack_id() as u64,
                    ),
                    &(field_id as common::types::EntityId),
                );
                match result {
                    Ok(()) => {
                        log::info!("Field deleted successfully");
                        // Refresh field list
                        fill_field_list(&app, &ctx);
                        // Clear field form
                        clear_field_form(&app);
                    }
                    Err(e) => {
                        log::error!("Failed to delete field: {}", e);
                    }
                }
            }
        }
    });
}

fn setup_entity_deletion_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_request_entity_deletion({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |entity_id| {
                if let Some(app) = app_weak.upgrade() {
                    log::info!("Entity deletion");
                    let result = entity_commands::remove_entity(
                        &ctx,
                        Some(
                            app.global::<EntitiesTabState>()
                                .get_entities_undo_stack_id() as u64,
                        ),
                        &(entity_id as common::types::EntityId),
                    );
                    match result {
                        Ok(()) => {
                            log::info!("Entity deleted successfully");
                            // Refresh entity list
                            fill_entity_list(&app, &ctx);
                            // Clear field list and form
                            clear_field_list(&app, &ctx);
                            clear_field_form(&app);
                        }
                        Err(e) => {
                            log::error!("Failed to delete entity: {}", e);
                        }
                    }
                }
            }
        });
}

fn setup_select_entity_callbacks(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_entity_selected({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |selected_entity_id| {
            if selected_entity_id < 0 {
                return;
            }
            if let Some(app) = app_weak.upgrade()
                && selected_entity_id >= 0
            {
                let entity_res = entity_commands::get_entity(
                    &ctx,
                    &(selected_entity_id as common::types::EntityId),
                );
                // Update ALL dependent properties here
                match entity_res {
                    Ok(Some(entity)) => {
                        app.global::<EntitiesTabState>()
                            .set_selected_entity_id(selected_entity_id);
                        app.global::<EntitiesTabState>()
                            .set_selected_entity_name(entity.name.into());
                        app.global::<EntitiesTabState>()
                            .set_selected_entity_only_for_heritage(entity.only_for_heritage);
                        app.global::<EntitiesTabState>()
                            .set_selected_entity_single_model(entity.single_model);
                        app.global::<EntitiesTabState>()
                            .set_selected_entity_undoable(entity.undoable);

                        // Fill inherits_from options and set the selected index
                        fill_inherits_from_options(&app, &ctx, entity.inherits_from);
                        fill_field_list(&app, &ctx);

                        fill_entity_options(&app, &ctx);

                        // clear other forms when an entity is selected
                        clear_field_form(&app);
                    }
                    _ => {
                        app.global::<EntitiesTabState>().set_selected_entity_id(-1);
                        app.global::<EntitiesTabState>()
                            .set_selected_entity_name("".into());
                        app.global::<EntitiesTabState>()
                            .set_selected_entity_only_for_heritage(false);
                        app.global::<EntitiesTabState>()
                            .set_selected_entity_inherits_from(-1);
                    }
                };
            };
        }
    });
}

/// Helper function to convert FieldType to string
fn field_type_to_string(field_type: &FieldType) -> &'static str {
    match field_type {
        FieldType::Boolean => "Boolean",
        FieldType::Integer => "Integer",
        FieldType::UInteger => "UInteger",
        FieldType::Float => "Float",
        FieldType::String => "String",
        FieldType::Uuid => "Uuid",
        FieldType::DateTime => "DateTime",
        FieldType::Entity => "Entity",
        FieldType::Enum => "Enum",
    }
}

/// Helper function to convert string to FieldType
fn string_to_field_type(s: &str) -> FieldType {
    match s {
        "Boolean" => FieldType::Boolean,
        "Integer" => FieldType::Integer,
        "UInteger" => FieldType::UInteger,
        "Float" => FieldType::Float,
        "Uuid" => FieldType::Uuid,
        "DateTime" => FieldType::DateTime,
        "Entity" => FieldType::Entity,
        "Enum" => FieldType::Enum,
        _ => FieldType::String,
    }
}

/// Helper function to convert FieldRelationshipType to string for UI
fn field_relationship_type_to_string(rel_type: &FieldRelationshipType) -> &'static str {
    match rel_type {
        FieldRelationshipType::OneToOne => "one_to_one",
        FieldRelationshipType::OneToMany => "one_to_many",
        FieldRelationshipType::OrderedOneToMany => "ordered_one_to_many",
        FieldRelationshipType::ManyToOne => "many_to_one",
        FieldRelationshipType::ManyToMany => "many_to_many",
    }
}

/// Helper function to convert string to FieldRelationshipType
fn string_to_field_relationship_type(s: &str) -> FieldRelationshipType {
    match s {
        "one_to_one" => FieldRelationshipType::OneToOne,
        "one_to_many" => FieldRelationshipType::OneToMany,
        "ordered_one_to_many" => FieldRelationshipType::OrderedOneToMany,
        "many_to_one" => FieldRelationshipType::ManyToOne,
        "many_to_many" => FieldRelationshipType::ManyToMany,
        _ => FieldRelationshipType::OneToOne,
    }
}

/// Helper function to fill all field form properties from a FieldDto
fn fill_field_form(app: &App, field: &direct_access::FieldDto) {
    let state = app.global::<EntitiesTabState>();
    state.set_selected_field_id(field.id as i32);
    state.set_selected_field_name(field.name.clone().into());
    state.set_selected_field_type(field_type_to_string(&field.field_type).into());

    // get entity index
    let field_entity_id = field.entity.map(|e| e as i32).unwrap_or(-1);
    let field_entity_index: i32 = if field_entity_id > 0 {
        get_option_index_from_entity_id(app, field_entity_id)
            .unwrap_or(0)
            .try_into()
            .unwrap_or(0)
    } else {
        -1
    };

    log::debug!("Field entity index: {}", field_entity_index);

    state.set_selected_field_entity_index(field_entity_index);
    state.set_selected_field_relationship(
        field_relationship_type_to_string(&field.relationship).into(),
    );
    state.set_selected_field_optional(field.optional);
    state.set_selected_field_is_list(field.is_list);
    state.set_selected_field_strong(field.strong);
    state.set_selected_field_list_model(field.list_model);
    state.set_selected_field_list_model_displayed_field(
        field
            .list_model_displayed_field
            .clone()
            .unwrap_or_default()
            .into(),
    );
    state.set_selected_field_enum_name(field.enum_name.clone().unwrap_or_default().into());
    state.set_selected_field_enum_values(field.enum_values.join("\n").into());
}

/// Helper function to clear field form
fn clear_field_form(app: &App) {
    let state = app.global::<EntitiesTabState>();
    state.set_selected_field_id(-1);
    state.set_selected_field_name("".into());
    state.set_selected_field_type("String".into());
    state.set_selected_field_entity_index(-1);
    state.set_selected_field_relationship("one_to_one".into());
    state.set_selected_field_optional(false);
    state.set_selected_field_is_list(false);
    state.set_selected_field_strong(true);
    state.set_selected_field_list_model(false);
    state.set_selected_field_list_model_displayed_field("".into());
    state.set_selected_field_enum_name("".into());
    state.set_selected_field_enum_values("".into());
}

/// Helper function to populate entity options for the Referenced Entity ComboBox
fn fill_entity_options(app: &App, app_context: &Arc<AppContext>) {
    let workspace_id = app.global::<AppState>().get_workspace_id() as common::types::EntityId;
    if workspace_id > 0 {
        let entity_ids_res = workspace_commands::get_workspace_relationship(
            app_context,
            &workspace_id,
            &WorkspaceRelationshipField::Entities,
        );

        if let Ok(entity_ids) = entity_ids_res
            && let Ok(entities_opt) = entity_commands::get_entity_multi(app_context, &entity_ids)
        {
            let mut names: Vec<slint::SharedString> = Vec::new();
            let mut ids: Vec<i32> = Vec::new();
            for e in entities_opt.into_iter().flatten() {
                if e.only_for_heritage {
                    continue;
                }
                names.push(e.name.into());
                ids.push(e.id as i32);
            }
            let names_model = std::rc::Rc::new(slint::VecModel::from(names));
            let ids_model = std::rc::Rc::new(slint::VecModel::from(ids));
            app.global::<EntitiesTabState>()
                .set_entity_options(names_model.into());
            app.global::<EntitiesTabState>()
                .set_entity_option_ids(ids_model.into());
        }
    }
}

fn get_entity_id_from_option_index(app: &App) -> Option<EntityId> {
    let index = app
        .global::<EntitiesTabState>()
        .get_selected_field_entity_index();
    if index == -1 {
        return None;
    }
    let slint_ids: slint::ModelRc<i32> = app.global::<EntitiesTabState>().get_entity_option_ids();
    let ids: Vec<i32> = slint_ids.iter().collect();
    ids.get(index as usize).copied().map(|id| id as EntityId)
}

fn get_option_index_from_entity_id(app: &App, entity_id: i32) -> Option<usize> {
    let slint_ids: slint::ModelRc<i32> = app.global::<EntitiesTabState>().get_entity_option_ids();
    let ids: Vec<i32> = slint_ids.iter().collect();
    ids.iter().position(|&id| id == entity_id)
}

/// Helper function to populate inherits_from options for the Inherits From ComboBox
/// Also sets the selected_entity_inherits_from_value directly to ensure proper synchronization
fn fill_inherits_from_options(
    app: &App,
    app_context: &Arc<AppContext>,
    current_inherits_from: Option<common::types::EntityId>,
) {
    let workspace_id = app.global::<AppState>().get_workspace_id() as common::types::EntityId;
    let mut selected_index: i32 = 0; // Default to "None"
    let mut selected_value: String = "None".to_string(); // Default to "None"

    if workspace_id > 0 {
        let entity_ids_res = workspace_commands::get_workspace_relationship(
            app_context,
            &workspace_id,
            &WorkspaceRelationshipField::Entities,
        );

        if let Ok(entity_ids) = entity_ids_res
            && let Ok(entities_opt) = entity_commands::get_entity_multi(app_context, &entity_ids)
        {
            // Start with "None" option
            let mut names: Vec<slint::SharedString> = vec!["None".into()];
            let mut ids: Vec<i32> = vec![-1];

            for maybe_entity in entities_opt.into_iter() {
                if let Some(e) = maybe_entity
                    && e.only_for_heritage
                {
                    names.push(e.name.clone().into());
                    ids.push(e.id as i32);

                    // Check if this is the currently selected inherits_from
                    if let Some(inherits_id) = current_inherits_from
                        && e.id == inherits_id
                    {
                        selected_index = (names.len() - 1) as i32;
                        selected_value = e.name.clone();
                    }
                }
            }

            let names_model = std::rc::Rc::new(slint::VecModel::from(names));
            let ids_model = std::rc::Rc::new(slint::VecModel::from(ids));
            app.global::<EntitiesTabState>()
                .set_inherits_from_options(names_model.into());
            app.global::<EntitiesTabState>()
                .set_inherits_from_option_ids(ids_model.into());
            // Set the selected index for the callback to use
            app.global::<EntitiesTabState>()
                .set_selected_entity_inherits_from(selected_index);
            // Set the selected value for the ComboBox current-value binding
            app.global::<EntitiesTabState>()
                .set_selected_entity_inherits_from_value(selected_value.into());
        }
    } else {
        // No workspace, set default "None" option and value
        app.global::<EntitiesTabState>()
            .set_selected_entity_inherits_from(0);
        app.global::<EntitiesTabState>()
            .set_selected_entity_inherits_from_value("None".into());
    }
}

/// Helper function to update a field with new values
fn update_field_helper<F>(app: &App, app_context: &Arc<AppContext>, field_id: i32, update_fn: F)
where
    F: FnOnce(&mut direct_access::FieldDto),
{
    if field_id < 0 {
        return;
    }

    let field_res = field_commands::get_field(app_context, &(field_id as common::types::EntityId));

    if let Ok(Some(mut field)) = field_res {
        update_fn(&mut field);
        match field_commands::update_field_with_relationships(
            app_context,
            Some(
                app.global::<EntitiesTabState>()
                    .get_entities_undo_stack_id() as u64,
            ),
            &field,
        ) {
            Ok(_) => {
                log::info!("Field updated successfully");
            }
            Err(e) => {
                log::error!("Failed to update field: {}", e);
            }
        }
    }
}

/// Helper function to update a field with new values
fn update_entity_helper<F>(app: &App, app_context: &Arc<AppContext>, entity_id: i32, update_fn: F)
where
    F: FnOnce(&mut direct_access::EntityDto),
{
    if entity_id < 0 {
        return;
    }

    let entity_res =
        entity_commands::get_entity(app_context, &(entity_id as common::types::EntityId));

    if let Ok(Some(mut entity)) = entity_res {
        update_fn(&mut entity);
        match entity_commands::update_entity_with_relationships(
            app_context,
            Some(
                app.global::<EntitiesTabState>()
                    .get_entities_undo_stack_id() as u64,
            ),
            &entity,
        ) {
            Ok(_) => {
                log::info!("Field updated successfully");
            }
            Err(e) => {
                log::error!("Failed to update field: {}", e);
            }
        }
    }
}

fn setup_select_field_callbacks(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_field_selected({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |selected_field_id| {
            if selected_field_id < 0 {
                return;
            }
            if let Some(app) = app_weak.upgrade() {
                if selected_field_id >= 0 {
                    let field_res = field_commands::get_field(
                        &ctx,
                        &(selected_field_id as common::types::EntityId),
                    );

                    if let Ok(Some(field)) = field_res {
                        fill_field_form(&app, &field);
                    } else {
                        clear_field_form(&app);
                    }
                } else {
                    clear_field_form(&app);
                }
            }
        }
    })
}

fn setup_field_name_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_field_name_changed({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |new_name| {
            if let Some(app) = app_weak.upgrade() {
                let field_id = app.global::<EntitiesTabState>().get_selected_field_id();
                let name_str = new_name.to_string();
                if !name_str.is_empty() {
                    update_field_helper(&app, &ctx, field_id, |field| {
                        field.name = name_str;
                    });
                }
            }
        }
    });
}

fn setup_field_type_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_field_type_changed({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |new_type| {
            if let Some(app) = app_weak.upgrade() {
                let field_id = app.global::<EntitiesTabState>().get_selected_field_id();
                let type_str = new_type.to_string();
                update_field_helper(&app, &ctx, field_id, |field| {
                    field.field_type = string_to_field_type(&type_str);
                    // Clear entity reference if not Entity type
                    if field.field_type != FieldType::Entity {
                        field.entity = None;
                        field.relationship = FieldRelationshipType::OneToOne;
                        field.strong = false;
                        field.list_model = false;
                        field.list_model_displayed_field = None;
                        app.global::<EntitiesTabState>()
                            .set_selected_field_entity_index(-1);
                        app.global::<EntitiesTabState>()
                            .set_selected_field_strong(false);
                        app.global::<EntitiesTabState>()
                            .set_selected_field_relationship("one_to_one".into());
                        app.global::<EntitiesTabState>()
                            .set_selected_field_list_model(false);
                        app.global::<EntitiesTabState>()
                            .set_selected_field_list_model_displayed_field("".into());
                    }
                    if field.field_type != FieldType::Enum {
                        field.enum_values = vec![];
                        field.enum_name = None;
                        app.global::<EntitiesTabState>()
                            .set_selected_field_enum_name("".into());
                        app.global::<EntitiesTabState>()
                            .set_selected_field_enum_values("".into());
                    }
                });
            }
        }
    });
}

fn setup_field_entity_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_field_entity_changed({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |_field_entity_index| {
            if let Some(app) = app_weak.upgrade() {
                let field_id = app.global::<EntitiesTabState>().get_selected_field_id();
                // Get the entity id from the index
                let entity_id = get_entity_id_from_option_index(&app);

                update_field_helper(&app, &ctx, field_id, |field| {
                    field.entity = entity_id;
                });
            }
        }
    });
}

fn setup_field_relationship_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_field_relationship_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |value| {
                if let Some(app) = app_weak.upgrade() {
                    let field_id = app.global::<EntitiesTabState>().get_selected_field_id();
                    let relationship_type = string_to_field_relationship_type(value.as_str());
                    update_field_helper(&app, &ctx, field_id, |field| {
                        field.relationship = relationship_type.clone();
                        if field.relationship == FieldRelationshipType::ManyToOne
                            || field.relationship == FieldRelationshipType::ManyToMany
                        {
                            field.strong = false;
                            app.global::<EntitiesTabState>()
                                .set_selected_field_strong(false);
                        }
                        if field.relationship == FieldRelationshipType::OrderedOneToMany
                            || field.relationship == FieldRelationshipType::OneToMany
                            || field.relationship == FieldRelationshipType::ManyToMany
                        {
                            field.optional = false;
                            app.global::<EntitiesTabState>()
                                .set_selected_field_optional(false);
                        }
                        if field.relationship == FieldRelationshipType::OneToOne
                            || field.relationship == FieldRelationshipType::ManyToOne
                        {
                            field.list_model = false;
                            field.list_model_displayed_field = None;
                            app.global::<EntitiesTabState>()
                                .set_selected_field_list_model(false);
                            app.global::<EntitiesTabState>()
                                .set_selected_field_list_model_displayed_field("".into());
                        }
                    });
                }
            }
        });
}

fn setup_field_optional_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_field_optional_changed({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |optional| {
            if let Some(app) = app_weak.upgrade() {
                let field_id = app.global::<EntitiesTabState>().get_selected_field_id();
                update_field_helper(&app, &ctx, field_id, |field| {
                    field.optional = optional;
                    if optional {
                        field.is_list = false;
                    }
                });
                if optional {
                    app.global::<EntitiesTabState>()
                        .set_selected_field_is_list(false);
                }
            }
        }
    });
}

fn setup_field_is_list_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_field_is_list_changed({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |is_list| {
            if let Some(app) = app_weak.upgrade() {
                let field_id = app.global::<EntitiesTabState>().get_selected_field_id();
                update_field_helper(&app, &ctx, field_id, |field| {
                    field.is_list = is_list;
                    if is_list {
                        field.optional = false;
                    }
                });
                if is_list {
                    app.global::<EntitiesTabState>()
                        .set_selected_field_optional(false);
                }
                fill_field_list(&app, &ctx);
            }
        }
    });
}

fn setup_field_strong_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_field_strong_changed({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |value| {
            if let Some(app) = app_weak.upgrade() {
                let field_id = app.global::<EntitiesTabState>().get_selected_field_id();
                update_field_helper(&app, &ctx, field_id, |field| {
                    field.strong = value;
                });
            }
        }
    });
}

fn setup_entity_single_model_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_entity_single_model_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |value| {
                if let Some(app) = app_weak.upgrade() {
                    let entity_id = app.global::<EntitiesTabState>().get_selected_entity_id();
                    update_entity_helper(&app, &ctx, entity_id, |entity| {
                        entity.single_model = value;
                    });
                }
            }
        });
}

fn setup_entity_undoable_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_entity_undoable_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |value| {
                if let Some(app) = app_weak.upgrade() {
                    let entity_id = app.global::<EntitiesTabState>().get_selected_entity_id();
                    update_entity_helper(&app, &ctx, entity_id, |entity| {
                        entity.undoable = value;
                    });
                }
            }
        });
}

fn setup_field_list_model_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_field_list_model_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |value| {
                if let Some(app) = app_weak.upgrade() {
                    let field_id = app.global::<EntitiesTabState>().get_selected_field_id();
                    update_field_helper(&app, &ctx, field_id, |field| {
                        field.list_model = value;
                        if field.list_model {
                            field.list_model_displayed_field = Some("name".into());
                            app.global::<EntitiesTabState>()
                                .set_selected_field_list_model_displayed_field("name".into());
                        } else {
                            field.list_model_displayed_field = None;
                            app.global::<EntitiesTabState>()
                                .set_selected_field_list_model_displayed_field("".into());
                        }
                    });
                }
            }
        });
}

fn setup_field_list_model_displayed_field_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_field_list_model_displayed_field_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |new_value| {
                if let Some(app) = app_weak.upgrade() {
                    let field_id = app.global::<EntitiesTabState>().get_selected_field_id();
                    let value_str = new_value.to_string();
                    update_field_helper(&app, &ctx, field_id, |field| {
                        field.list_model_displayed_field = if value_str.is_empty() {
                            None
                        } else {
                            Some(value_str)
                        };
                    });
                }
            }
        });
}

fn setup_field_enum_name_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_field_enum_name_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |new_value| {
                if let Some(app) = app_weak.upgrade() {
                    let field_id = app.global::<EntitiesTabState>().get_selected_field_id();
                    let value_str = new_value.to_string();
                    update_field_helper(&app, &ctx, field_id, |field| {
                        field.enum_name = if value_str.is_empty() {
                            None
                        } else {
                            Some(value_str)
                        };
                    });
                }
            }
        });
}

fn setup_field_enum_values_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_field_enum_values_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |new_value| {
                if let Some(app) = app_weak.upgrade() {
                    let field_id = app.global::<EntitiesTabState>().get_selected_field_id();
                    let value_str = new_value.to_string();
                    update_field_helper(&app, &ctx, field_id, |field| {
                        if value_str.is_empty() {
                            field.enum_values = vec![];
                        } else {
                            // Split by newlines only (not commas — complex variants
                            // like "Foo { a: u64, b: String }" contain commas)
                            field.enum_values = value_str
                                .split('\n')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }
                    });
                }
            }
        });
}

fn setup_entity_name_callbacks(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_entity_name_changed({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move |new_entity_name| {
            if let Some(app) = app_weak.upgrade()
                && !new_entity_name.is_empty()
            {
                let current_entity_id = app.global::<EntitiesTabState>().get_selected_entity_id();
                let entity_res = entity_commands::get_entity(
                    &ctx,
                    &(current_entity_id as common::types::EntityId),
                );

                // Update
                if let Ok(Some(mut entity)) = entity_res.map(|e| e.map(UpdateEntityDto::from)) {
                    if new_entity_name == entity.name {
                        return;
                    }
                    entity.name = new_entity_name.to_string();

                    let result = entity_commands::update_entity(
                        &ctx,
                        Some(
                            app.global::<EntitiesTabState>()
                                .get_entities_undo_stack_id() as u64,
                        ),
                        &entity,
                    );

                    match result {
                        Ok(_) => {
                            log::info!("Entity name updated successfully");
                        }
                        Err(e) => {
                            log::error!("Failed to update entity name: {}", e);
                        }
                    }
                };
            };
        }
    });
}

fn setup_entity_only_for_heritage_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_entity_only_for_heritage_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |value| {
                if let Some(app) = app_weak.upgrade() {
                    let current_entity_id =
                        app.global::<EntitiesTabState>().get_selected_entity_id();
                    if current_entity_id < 0 {
                        return;
                    }
                    let entity_res = entity_commands::get_entity(
                        &ctx,
                        &(current_entity_id as common::types::EntityId),
                    );

                    if let Ok(Some(mut entity)) = entity_res {
                        if entity.only_for_heritage == value {
                            return;
                        }
                        entity.only_for_heritage = value;
                        if value {
                            entity.undoable = false;
                            entity.single_model = false;
                            entity.inherits_from = None;
                            app.global::<EntitiesTabState>()
                                .set_selected_entity_undoable(false);
                            app.global::<EntitiesTabState>()
                                .set_selected_entity_single_model(false);
                            app.global::<EntitiesTabState>()
                                .set_selected_entity_inherits_from(-1);
                            app.global::<EntitiesTabState>()
                                .set_selected_entity_inherits_from_value("None".into());
                        }
                        let result = entity_commands::update_entity_with_relationships(
                            &ctx,
                            Some(
                                app.global::<EntitiesTabState>()
                                    .get_entities_undo_stack_id()
                                    as u64,
                            ),
                            &entity,
                        );

                        match result {
                            Ok(_) => {
                                log::info!("Entity only_for_heritage updated successfully");
                            }
                            Err(e) => {
                                log::error!("Failed to update entity only_for_heritage: {}", e);
                            }
                        }
                    };
                }
            }
        });
}

fn setup_entity_addition_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_request_entity_addition({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move || {
                if let Some(app) = app_weak.upgrade() {
                    let workspace_id = app.global::<AppState>().get_workspace_id();
                    if workspace_id <= 0 {
                        log::warn!("Cannot add entity: no workspace loaded");
                        return;
                    }

                    // Create a new entity with default values
                    let create_dto = direct_access::CreateEntityDto {
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                        name: "NewEntity".to_string(),
                        only_for_heritage: false,
                        inherits_from: None,
                        single_model: false,
                        fields: vec![],
                        relationships: vec![],
                        undoable: true,
                    };

                    match entity_commands::create_orphan_entity(
                        &ctx,
                        Some(
                            app.global::<EntitiesTabState>()
                                .get_entities_undo_stack_id() as u64,
                        ),
                        &create_dto,
                    ) {
                        Ok(new_entity) => {
                            log::info!("Entity created successfully with id: {}", new_entity.id);

                            // Get current entity ids from workspace
                            let entity_ids_res = workspace_commands::get_workspace_relationship(
                                &ctx,
                                &(workspace_id as common::types::EntityId),
                                &WorkspaceRelationshipField::Entities,
                            );

                            match entity_ids_res {
                                Ok(mut entity_ids) => {
                                    // Add the new entity id to the list
                                    entity_ids.push(new_entity.id);

                                    // Update the workspace relationship
                                    let relationship_dto = WorkspaceRelationshipDto {
                                        id: workspace_id as common::types::EntityId,
                                        field: WorkspaceRelationshipField::Entities,
                                        right_ids: entity_ids,
                                    };

                                    if let Err(e) = workspace_commands::set_workspace_relationship(
                                        &ctx,
                                        Some(
                                            app.global::<EntitiesTabState>()
                                                .get_entities_undo_stack_id()
                                                as u64,
                                        ),
                                        &relationship_dto,
                                    ) {
                                        log::error!(
                                            "Failed to add entity to workspace relationship: {}",
                                            e
                                        );
                                    }
                                }
                                Err(e) => {
                                    log::error!(
                                        "Failed to get workspace entities relationship: {}",
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to create entity: {}", e);
                        }
                    }
                }
            }
        });
}

fn setup_field_addition_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>().on_request_field_addition({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move || {
            if let Some(app) = app_weak.upgrade() {
                let entity_id = app.global::<EntitiesTabState>().get_selected_entity_id();
                if entity_id < 0 {
                    log::warn!("Cannot add field: no entity selected");
                    return;
                }

                // Create a new field with default values
                let create_dto = direct_access::CreateFieldDto {
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    name: "new_field".to_string(),
                    field_type: FieldType::String,
                    entity: None,
                    relationship: FieldRelationshipType::OneToOne,
                    optional: false,
                    is_list: false,
                    strong: true,
                    list_model: false,
                    list_model_displayed_field: None,
                    enum_name: None,
                    enum_values: vec![],
                };

                match field_commands::create_orphan_field(
                    &ctx,
                    Some(
                        app.global::<EntitiesTabState>()
                            .get_entities_undo_stack_id() as u64,
                    ),
                    &create_dto,
                ) {
                    Ok(new_field) => {
                        log::info!("Field created successfully with id: {}", new_field.id);

                        // Get current field ids from entity
                        let field_ids_res = entity_commands::get_entity_relationship(
                            &ctx,
                            &(entity_id as common::types::EntityId),
                            &EntityRelationshipField::Fields,
                        );

                        match field_ids_res {
                            Ok(mut field_ids) => {
                                // Add the new field id to the list
                                field_ids.push(new_field.id);

                                // Update the entity relationship
                                let relationship_dto = EntityRelationshipDto {
                                    id: entity_id as common::types::EntityId,
                                    field: EntityRelationshipField::Fields,
                                    right_ids: field_ids,
                                };

                                if let Err(e) = entity_commands::set_entity_relationship(
                                    &ctx,
                                    Some(
                                        app.global::<EntitiesTabState>()
                                            .get_entities_undo_stack_id()
                                            as u64,
                                    ),
                                    &relationship_dto,
                                ) {
                                    log::error!(
                                        "Failed to add field to entity relationship: {}",
                                        e
                                    );
                                } else {
                                    // Refresh the field list to show the new field
                                    fill_field_list(&app, &ctx);
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to get entity fields relationship: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to create field: {}", e);
                    }
                }
            }
        }
    });
}

fn setup_entity_inherits_from_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_entity_inherits_from_changed({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move |selected_index| {
                if let Some(app) = app_weak.upgrade() {
                    let current_entity_id =
                        app.global::<EntitiesTabState>().get_selected_entity_id();
                    if current_entity_id < 0 {
                        return;
                    }

                    // Get the entity id from the selected index
                    let inherits_from_option_ids = app
                        .global::<EntitiesTabState>()
                        .get_inherits_from_option_ids();
                    let inherits_from_id = if selected_index >= 0
                        && (selected_index as usize) < inherits_from_option_ids.row_count()
                    {
                        let id = inherits_from_option_ids
                            .row_data(selected_index as usize)
                            .unwrap_or(-1);
                        if id < 0 {
                            None // "None" selected
                        } else {
                            Some(id as common::types::EntityId)
                        }
                    } else {
                        None
                    };

                    let entity_res = entity_commands::get_entity(
                        &ctx,
                        &(current_entity_id as common::types::EntityId),
                    );

                    if let Ok(Some(mut entity)) = entity_res {
                        if entity.inherits_from == inherits_from_id {
                            return;
                        }
                        entity.inherits_from = inherits_from_id;

                        let result = entity_commands::update_entity_with_relationships(
                            &ctx,
                            Some(
                                app.global::<EntitiesTabState>()
                                    .get_entities_undo_stack_id()
                                    as u64,
                            ),
                            &entity,
                        );

                        match result {
                            Ok(_) => {
                                log::info!("Entity inherits_from updated successfully");
                            }
                            Err(e) => {
                                log::error!("Failed to update entity inherits_from: {}", e);
                            }
                        }
                    };
                }
            }
        });
}

fn setup_export_to_mermaid_clipboard_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<EntitiesTabState>()
        .on_export_to_mermaid_clipboard({
            let ctx = Arc::clone(app_context);
            let app_weak = app.as_weak();
            move || {
                if let Some(app) = app_weak.upgrade() {
                    let workspace_id = app.global::<AppState>().get_workspace_id();
                    if workspace_id <= 0 {
                        log::warn!("Cannot export to mermaid: no workspace loaded");
                        return;
                    }

                    match handling_manifest_commands::export_to_mermaid(&ctx) {
                        Ok(return_dto) => {
                            super::common::set_clipboard_text(return_dto.mermaid_diagram);
                            log::info!(
                                "Entities exported to mermaid markdown and copied to clipboard"
                            );
                            // Show success message
                            app.global::<AppState>().set_success_message(
                                slint::SharedString::from(
                                    "Entities exported to mermaid markdown and copied to clipboard",
                                ),
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
                        Err(e) => {
                            log::error!("Failed to export entities to mermaid markdown: {}", e);
                        }
                    }
                }
            }
        });
}

/// Initialize all entities tab related subscriptions and callbacks
pub fn init(event_hub_client: &EventHubClient, app: &App, app_context: &Arc<AppContext>) {
    // Event subscriptions
    subscribe_workspace_updated_event(event_hub_client, app, app_context);
    subscribe_new_manifest_event(event_hub_client, app, app_context);
    subscribe_close_manifest_event(event_hub_client, app, app_context);
    subscribe_load_manifest_event(event_hub_client, app, app_context);
    subscribe_entity_updated_event(event_hub_client, app, app_context);
    subscribe_entity_deleted_event(event_hub_client, app, app_context);
    subscribe_field_updated_event(event_hub_client, app, app_context);
    subscribe_field_deleted_event(event_hub_client, app, app_context);

    // common
    setup_export_to_mermaid_clipboard_callback(app, app_context);

    // Entity callbacks
    setup_entities_reorder_callback(app, app_context);
    setup_select_entity_callbacks(app, app_context);
    setup_entity_name_callbacks(app, app_context);
    setup_entity_only_for_heritage_callback(app, app_context);
    setup_entity_single_model_callback(app, app_context);
    setup_entity_undoable_callback(app, app_context);
    setup_entity_inherits_from_callback(app, app_context);
    setup_entity_deletion_callback(app, app_context);
    setup_entity_addition_callback(app, app_context);

    // Field list callbacks
    setup_fields_reorder_callback(app, app_context);
    setup_select_field_callbacks(app, app_context);
    setup_field_deletion_callback(app, app_context);
    setup_field_addition_callback(app, app_context);

    // Field detail callbacks
    setup_field_name_callback(app, app_context);
    setup_field_type_callback(app, app_context);
    setup_field_entity_callback(app, app_context);
    setup_field_relationship_callback(app, app_context);
    setup_field_optional_callback(app, app_context);
    setup_field_is_list_callback(app, app_context);
    setup_field_strong_callback(app, app_context);
    setup_field_list_model_callback(app, app_context);
    setup_field_list_model_displayed_field_callback(app, app_context);
    setup_field_enum_name_callback(app, app_context);
    setup_field_enum_values_callback(app, app_context);
}
