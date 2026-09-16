//! The generated reactive layer, driven without a window.
//!
//! This replaces the generator's `demo_headless.rs`, which tested the demo window it
//! also generated. The window is gone; the layer under it is what the whole UI rests
//! on, and these tests pin it directly.
//!
//! Every handle loads synchronously on `set_id` / `set_owner_id`; `wire` only ever
//! *re*-loads, in response to an event. So none of this needs a `BuildContext`, an
//! event source, or a display.

use std::rc::Rc;

use teksilo::prelude::Signal;

use frontend::AppContext;
use frontend::commands::{entity_commands, handling_app_lifecycle_commands, workspace_commands};
use frontend::direct_access::{CreateEntityDto, CreateWorkspaceDto};

use teksilo_ui::models::WorkspaceEntitiesListModel;
use teksilo_ui::session::Session;
use teksilo_ui::singles::{LoadingStatus, SingleEntity};

/// A fresh backend with Root and System seeded, exactly as `run()` leaves it.
///
/// Each `AppContext` owns its own store, so tests never see each other. No
/// `EventHubClient` is started: the hub's channel is unbounded, so publishing with
/// nobody listening neither blocks nor drops anything read here.
fn booted() -> Rc<AppContext> {
    let ctx = Rc::new(AppContext::new());
    handling_app_lifecycle_commands::initialize_app(&ctx).expect("initialize_app");
    ctx
}

/// A workspace to hang entities off, returned by id.
fn workspace(ctx: &AppContext) -> u64 {
    let now = chrono::Utc::now();
    workspace_commands::create_orphan_workspace(
        ctx,
        None,
        &CreateWorkspaceDto {
            manifest_absolute_path: "/tmp/qleany-test/qleany.yaml".to_string(),
            created_at: now,
            updated_at: now,
            ..Default::default()
        },
    )
    .expect("create workspace")
    .id
}

fn new_entity(name: &str) -> CreateEntityDto {
    let now = chrono::Utc::now();
    CreateEntityDto {
        name: name.to_string(),
        undoable: true,
        created_at: now,
        updated_at: now,
        ..Default::default()
    }
}

fn entities_of(ctx: &Rc<AppContext>, workspace_id: u64) -> WorkspaceEntitiesListModel {
    WorkspaceEntitiesListModel::new(ctx.clone(), Signal::new(Some(workspace_id)))
}

#[test]
fn a_session_hands_out_every_generated_handle() {
    let session = Session::new(booted());
    // Reading them is the assertion: a handle the generator forgot to mint would not
    // compile, and the eight here are exactly the eight `single_model` flags in the
    // manifest.
    assert!(session.single_entity.id().is_none());
    assert!(session.single_field.id().is_none());
    assert!(session.single_feature.id().is_none());
    assert!(session.single_use_case.id().is_none());
    assert!(session.single_dto.id().is_none());
    assert!(session.single_dto_field.id().is_none());
    assert!(session.single_global.id().is_none());
    assert!(session.single_user_interface.id().is_none());
    assert!(session.workspace_entities.is_empty());
}

#[test]
fn a_list_model_with_no_owner_is_empty_rather_than_everything() {
    let ctx = booted();
    let ws = workspace(&ctx);
    let model = WorkspaceEntitiesListModel::new(ctx.clone(), Signal::new(None));

    entity_commands::create_entity(&ctx, None, &new_entity("Alpha"), ws, -1).expect("create");
    model.refresh();

    // No owner means no rows. A model that fell back to `get_all` would show this
    // entity, and in a real app would show every other owner's rows too.
    assert_eq!(model.len(), 0);
}

#[test]
fn a_list_model_loads_its_owner_rows_in_relationship_order() {
    let ctx = booted();
    let ws = workspace(&ctx);
    let model = entities_of(&ctx, ws);

    for name in ["Alpha", "Beta", "Gamma"] {
        model.create(&new_entity(name), -1, None).expect("create");
    }

    let names: Vec<String> = model.rows().into_iter().map(|r| r.name).collect();
    assert_eq!(names, vec!["Alpha", "Beta", "Gamma"]);
}

#[test]
fn move_to_reorders_through_the_backend_and_survives_a_refresh() {
    let ctx = booted();
    let ws = workspace(&ctx);
    let model = entities_of(&ctx, ws);

    let mut ids = Vec::new();
    for name in ["Alpha", "Beta", "Gamma"] {
        ids.push(model.create(&new_entity(name), -1, None).expect("create"));
    }

    // Move the first row to the end. The store removes the moved id *before*
    // inserting, so the index is into the list without it.
    model.move_to(ids[0], 2, None);
    model.refresh();

    let names: Vec<String> = model.rows().into_iter().map(|r| r.name).collect();
    assert_eq!(
        names,
        vec!["Beta", "Gamma", "Alpha"],
        "a reorder that only moved the local ListModel would read correct before the \
         refresh and revert after it"
    );
}

#[test]
fn a_single_loads_the_row_its_id_points_at() {
    let ctx = booted();
    let ws = workspace(&ctx);
    let model = entities_of(&ctx, ws);
    let alpha = model
        .create(&new_entity("Alpha"), -1, None)
        .expect("create");
    let beta = model.create(&new_entity("Beta"), -1, None).expect("create");

    let single = SingleEntity::new(ctx.clone());
    single.set_id(Some(alpha));
    assert_eq!(single.loading_status().get(), LoadingStatus::Loaded);
    assert_eq!(single.name().get(), "Alpha");

    // Re-pointing is the whole contract: a handle that loaded once and then ignored
    // `set_id` would still report "Alpha" here, and every detail pane in the app
    // would show the first row a user ever clicked.
    single.set_id(Some(beta));
    assert_eq!(single.name().get(), "Beta");

    single.set_id(None);
    assert_eq!(single.loading_status().get(), LoadingStatus::Unloaded);
}

#[test]
fn a_single_writes_a_scalar_back() {
    let ctx = booted();
    let ws = workspace(&ctx);
    let model = entities_of(&ctx, ws);
    let id = model
        .create(&new_entity("Alpha"), -1, None)
        .expect("create");

    let single = SingleEntity::new(ctx.clone());
    single.set_id(Some(id));
    single.set_name("Renamed".to_string());
    single.save(None);

    let stored = entity_commands::get_entity(&ctx, &id)
        .expect("get")
        .expect("present");
    assert_eq!(stored.name, "Renamed");
}

#[test]
fn removing_a_row_drops_it_from_the_list() {
    let ctx = booted();
    let ws = workspace(&ctx);
    let model = entities_of(&ctx, ws);
    let a = model
        .create(&new_entity("Alpha"), -1, None)
        .expect("create");
    model.create(&new_entity("Beta"), -1, None).expect("create");

    model.remove(a, None);
    model.refresh();

    let names: Vec<String> = model.rows().into_iter().map(|r| r.name).collect();
    assert_eq!(names, vec!["Beta"]);
}
