// Functional tests for Project controller (full CRUD, all field types, all relationship types)

use crate::helpers::{self, TestContext};
use direct_access::*;

fn setup() -> (TestContext, helpers::Scaffold) {
    let mut ctx = TestContext::new();
    let scaffold = helpers::create_scaffold(&mut ctx);
    (ctx, scaffold)
}

// ---------------------------------------------------------------------------
// create
// ---------------------------------------------------------------------------

#[test]
fn test_create_with_owner() {
    let (mut ctx, s) = setup();
    let proj_id = helpers::create_project(&mut ctx, s.workspace_id, "MyProject");
    let proj = project_controller::get(&ctx.db, &proj_id).unwrap().unwrap();
    assert_eq!(proj.title, "MyProject");
}

#[test]
fn test_create_at_index() {
    let (mut ctx, s) = setup();
    let _second = helpers::create_project(&mut ctx, s.workspace_id, "Second");

    // Insert at index 0 (raw create, then add settings)
    let inserted = project_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateProjectDto {
            title: "Inserted".into(),
            ..Default::default()
        },
        s.workspace_id,
        0,
    )
    .unwrap();
    helpers::create_project_settings(&mut ctx, inserted.id);

    let rel = workspace_controller::get_relationship(
        &ctx.db,
        &s.workspace_id,
        &WorkspaceRelationshipField::Projects,
    )
    .unwrap();
    assert_eq!(rel[0], inserted.id);
}

// ---------------------------------------------------------------------------
// get
// ---------------------------------------------------------------------------

#[test]
fn test_get_by_id() {
    let (ctx, s) = setup();
    let fetched = project_controller::get(&ctx.db, &s.project_id).unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().title, "TestProject");
}

#[test]
fn test_get_non_existent() {
    let (ctx, _) = setup();
    assert!(project_controller::get(&ctx.db, &999999).unwrap().is_none());
}

#[test]
fn test_get_all() {
    let (ctx, _) = setup();
    let all = project_controller::get_all(&ctx.db).unwrap();
    assert!(all.len() >= 1);
}

// ---------------------------------------------------------------------------
// update — all field types
// ---------------------------------------------------------------------------

#[test]
fn test_update_string_fields() {
    let (mut ctx, s) = setup();
    let dto = project_controller::get(&ctx.db, &s.project_id)
        .unwrap()
        .unwrap();
    let mut update_dto: UpdateProjectDto = dto.into();
    update_dto.title = "Updated Title".into();
    update_dto.description = "Updated Desc".into();
    let updated =
        project_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.description, "Updated Desc");
}

#[test]
fn test_update_bool_field() {
    let (mut ctx, s) = setup();
    let dto = project_controller::get(&ctx.db, &s.project_id)
        .unwrap()
        .unwrap();
    let mut update_dto: UpdateProjectDto = dto.into();
    update_dto.is_active = true;
    let updated =
        project_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert!(updated.is_active);
}

#[test]
fn test_update_numeric_fields() {
    let (mut ctx, s) = setup();
    let dto = project_controller::get(&ctx.db, &s.project_id)
        .unwrap()
        .unwrap();
    let mut update_dto: UpdateProjectDto = dto.into();
    update_dto.priority = 99;
    update_dto.budget = 42.5;
    let updated =
        project_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.priority, 99);
    assert_eq!(updated.budget, 42.5);
}

#[test]
fn test_update_enum_field() {
    let (mut ctx, s) = setup();
    let dto = project_controller::get(&ctx.db, &s.project_id)
        .unwrap()
        .unwrap();
    let mut update_dto: UpdateProjectDto = dto.into();
    update_dto.status = common::entities::ProjectStatus::Archived;
    let updated =
        project_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.status, common::entities::ProjectStatus::Archived);
}

#[test]
fn test_update_uuid_field() {
    let (mut ctx, s) = setup();
    let new_uuid = uuid::Uuid::new_v4();
    let dto = project_controller::get(&ctx.db, &s.project_id)
        .unwrap()
        .unwrap();
    let mut update_dto: UpdateProjectDto = dto.into();
    update_dto.uuid = new_uuid;
    let updated =
        project_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.uuid, new_uuid);

    let fetched = project_controller::get(&ctx.db, &s.project_id)
        .unwrap()
        .unwrap();
    assert_eq!(fetched.uuid, new_uuid);
}

#[test]
fn test_update_datetime_field() {
    let (mut ctx, s) = setup();
    let new_date = chrono::Utc::now() + chrono::Duration::days(30);
    let dto = project_controller::get(&ctx.db, &s.project_id)
        .unwrap()
        .unwrap();
    let mut update_dto: UpdateProjectDto = dto.into();
    update_dto.deadline = new_date;
    let updated =
        project_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert!((updated.deadline - new_date).num_milliseconds().abs() < 10);
}

// ---------------------------------------------------------------------------
// remove
// ---------------------------------------------------------------------------

#[test]
fn test_remove() {
    let (mut ctx, s) = setup();
    project_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &s.project_id).unwrap();
    assert!(
        project_controller::get(&ctx.db, &s.project_id)
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_remove_cascades_tasks() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "Child");
    project_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &s.project_id).unwrap();
    assert!(task_controller::get(&ctx.db, &task_id).unwrap().is_none());
}

// ---------------------------------------------------------------------------
// relationships: Tasks (ordered_one_to_many strong)
// ---------------------------------------------------------------------------

#[test]
fn test_get_relationship_tasks() {
    let (mut ctx, s) = setup();
    let t1 = helpers::create_task(&mut ctx, s.project_id, "T1");
    let t2 = helpers::create_task(&mut ctx, s.project_id, "T2");

    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert_eq!(rel, vec![t1, t2]);
}

#[test]
fn test_get_relationship_tasks_count() {
    let (mut ctx, s) = setup();
    helpers::create_task(&mut ctx, s.project_id, "A");
    helpers::create_task(&mut ctx, s.project_id, "B");
    helpers::create_task(&mut ctx, s.project_id, "C");

    let count = project_controller::get_relationship_count(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert_eq!(count, 3);
}

// ---------------------------------------------------------------------------
// relationships: Tags (many_to_many)
// ---------------------------------------------------------------------------

#[test]
fn test_set_and_get_relationship_tags() {
    let (mut ctx, s) = setup();
    let tag_a = helpers::create_tag(&mut ctx, s.workspace_id, "TagA", "#F00");
    let tag_b = helpers::create_tag(&mut ctx, s.workspace_id, "TagB", "#0F0");

    project_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &ProjectRelationshipDto {
            id: s.project_id,
            field: ProjectRelationshipField::Tags,
            right_ids: vec![tag_a, tag_b],
        },
    )
    .unwrap();

    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tags,
    )
    .unwrap();
    assert_eq!(rel.len(), 2);
    assert!(rel.contains(&tag_a));
    assert!(rel.contains(&tag_b));
}

// ---------------------------------------------------------------------------
// relationships: Settings (one_to_one strong required)
// ---------------------------------------------------------------------------

#[test]
fn test_project_has_settings() {
    let (ctx, s) = setup();
    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Settings,
    )
    .unwrap();
    assert_eq!(rel.len(), 1);
    assert_eq!(rel[0], s.project_settings_id);
}

// ---------------------------------------------------------------------------
// relationships: Lead (one_to_one weak optional)
// ---------------------------------------------------------------------------

#[test]
fn test_set_and_get_relationship_lead() {
    let (mut ctx, s) = setup();
    let member = helpers::create_team_member(&mut ctx, s.workspace_id, "Alice", "alice@test.com");

    project_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &ProjectRelationshipDto {
            id: s.project_id,
            field: ProjectRelationshipField::Lead,
            right_ids: vec![member],
        },
    )
    .unwrap();

    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Lead,
    )
    .unwrap();
    assert_eq!(rel, vec![member]);
}
