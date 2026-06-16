// Functional tests for Task controller (entity with owner, has children)

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
    let task = task_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTaskDto {
            title: "MyTask".into(),
            content: "Body".into(),
            ..Default::default()
        },
        s.project_id,
        -1,
    )
    .unwrap();
    assert!(task.id > 0);
    assert_eq!(task.title, "MyTask");
}

#[test]
fn test_create_multiple() {
    let (mut ctx, s) = setup();
    let tasks = task_controller::create_multi(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &[
            CreateTaskDto {
                title: "T1".into(),
                ..Default::default()
            },
            CreateTaskDto {
                title: "T2".into(),
                ..Default::default()
            },
        ],
        s.project_id,
        -1,
    )
    .unwrap();
    assert_eq!(tasks.len(), 2);
    assert_ne!(tasks[0].id, tasks[1].id);
}

#[test]
fn test_create_at_index() {
    let (mut ctx, s) = setup();
    helpers::create_task(&mut ctx, s.project_id, "First");
    let inserted = task_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTaskDto {
            title: "Inserted".into(),
            ..Default::default()
        },
        s.project_id,
        0,
    )
    .unwrap();

    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert_eq!(rel[0], inserted.id);
}

// ---------------------------------------------------------------------------
// get
// ---------------------------------------------------------------------------

#[test]
fn test_get_by_id() {
    let (mut ctx, s) = setup();
    let id = helpers::create_task(&mut ctx, s.project_id, "GetMe");
    let fetched = task_controller::get(&ctx.db, &id).unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().title, "GetMe");
}

#[test]
fn test_get_non_existent() {
    let (ctx, _) = setup();
    assert!(task_controller::get(&ctx.db, &999999).unwrap().is_none());
}

#[test]
fn test_get_all() {
    let (mut ctx, s) = setup();
    helpers::create_task(&mut ctx, s.project_id, "All");
    let all = task_controller::get_all(&ctx.db).unwrap();
    assert!(all.len() >= 1);
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

#[test]
fn test_update_string_fields() {
    let (mut ctx, s) = setup();
    let id = helpers::create_task(&mut ctx, s.project_id, "OldTitle");
    let dto = task_controller::get(&ctx.db, &id).unwrap().unwrap();
    let mut update_dto: UpdateTaskDto = dto.into();
    update_dto.title = "NewTitle".into();
    update_dto.content = "NewContent".into();
    let updated =
        task_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.title, "NewTitle");
    assert_eq!(updated.content, "NewContent");
}

#[test]
fn test_update_bool_field() {
    let (mut ctx, s) = setup();
    let id = helpers::create_task(&mut ctx, s.project_id, "Bool");
    let dto = task_controller::get(&ctx.db, &id).unwrap().unwrap();
    assert!(!dto.is_done);
    let mut update_dto: UpdateTaskDto = dto.into();
    update_dto.is_done = true;
    let updated =
        task_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert!(updated.is_done);
}

#[test]
fn test_update_numeric_fields() {
    let (mut ctx, s) = setup();
    let id = helpers::create_task(&mut ctx, s.project_id, "Num");
    let dto = task_controller::get(&ctx.db, &id).unwrap().unwrap();
    let mut update_dto: UpdateTaskDto = dto.into();
    update_dto.weight = 9.5;
    update_dto.effort_points = 42;
    let updated =
        task_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.weight, 9.5);
    assert_eq!(updated.effort_points, 42);
}

#[test]
fn test_update_enum_field() {
    let (mut ctx, s) = setup();
    let id = helpers::create_task(&mut ctx, s.project_id, "Enum");
    let dto = task_controller::get(&ctx.db, &id).unwrap().unwrap();
    let mut update_dto: UpdateTaskDto = dto.into();
    update_dto.difficulty = common::entities::TaskDifficulty::Expert;
    let updated =
        task_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.difficulty, common::entities::TaskDifficulty::Expert);
}

// ---------------------------------------------------------------------------
// remove
// ---------------------------------------------------------------------------

#[test]
fn test_remove() {
    let (mut ctx, s) = setup();
    let id = helpers::create_task(&mut ctx, s.project_id, "ToDelete");
    task_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &id).unwrap();
    assert!(task_controller::get(&ctx.db, &id).unwrap().is_none());
}

#[test]
fn test_remove_cascades_children() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "Parent");
    let comment_id = helpers::create_comment(&mut ctx, task_id, "Child");

    task_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &task_id).unwrap();
    assert!(task_controller::get(&ctx.db, &task_id).unwrap().is_none());
    assert!(
        comment_controller::get(&ctx.db, &comment_id)
            .unwrap()
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// relationships
// ---------------------------------------------------------------------------

#[test]
fn test_create_comment_as_child() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "WithComment");
    let comment_id = helpers::create_comment(&mut ctx, task_id, "Hello");

    let rel =
        task_controller::get_relationship(&ctx.db, &task_id, &TaskRelationshipField::Comments)
            .unwrap();
    assert!(rel.contains(&comment_id));
}

#[test]
fn test_set_and_get_relationship_tags() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "Tagged");
    let tag_id = helpers::create_tag(&mut ctx, s.workspace_id, "Tag1", "#000");

    task_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TaskRelationshipDto {
            id: task_id,
            field: TaskRelationshipField::Tags,
            right_ids: vec![tag_id],
        },
    )
    .unwrap();

    let rel =
        task_controller::get_relationship(&ctx.db, &task_id, &TaskRelationshipField::Tags).unwrap();
    assert_eq!(rel, vec![tag_id]);
}

#[test]
fn test_get_relationship_count() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "CountComments");
    helpers::create_comment(&mut ctx, task_id, "C1");
    helpers::create_comment(&mut ctx, task_id, "C2");

    let count = task_controller::get_relationship_count(
        &ctx.db,
        &task_id,
        &TaskRelationshipField::Comments,
    )
    .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_move_relationship_comments() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "MoveComments");
    let c1 = helpers::create_comment(&mut ctx, task_id, "First");
    let c2 = helpers::create_comment(&mut ctx, task_id, "Second");
    let c3 = helpers::create_comment(&mut ctx, task_id, "Third");

    // Move c3 to front
    task_controller::move_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &task_id,
        &TaskRelationshipField::Comments,
        &[c3],
        0,
    )
    .unwrap();

    let rel =
        task_controller::get_relationship(&ctx.db, &task_id, &TaskRelationshipField::Comments)
            .unwrap();
    assert_eq!(rel, vec![c3, c1, c2]);
}

#[test]
fn test_set_relationship_tags_overwrite() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "Overwrite");
    let t1 = helpers::create_tag(&mut ctx, s.workspace_id, "old", "#000");
    let t2 = helpers::create_tag(&mut ctx, s.workspace_id, "new", "#FFF");

    task_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TaskRelationshipDto {
            id: task_id,
            field: TaskRelationshipField::Tags,
            right_ids: vec![t1],
        },
    )
    .unwrap();

    // Overwrite with t2
    task_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TaskRelationshipDto {
            id: task_id,
            field: TaskRelationshipField::Tags,
            right_ids: vec![t2],
        },
    )
    .unwrap();

    let rel =
        task_controller::get_relationship(&ctx.db, &task_id, &TaskRelationshipField::Tags).unwrap();
    assert_eq!(rel, vec![t2]);
}

// ---------------------------------------------------------------------------
// update_with_relationships
// ---------------------------------------------------------------------------

#[test]
fn test_update_with_relationships_changes_scalars_and_tags() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "Original");
    let t1 = helpers::create_tag(&mut ctx, s.workspace_id, "Tag1", "#000");
    let t2 = helpers::create_tag(&mut ctx, s.workspace_id, "Tag2", "#FFF");

    // Set initial tags via set_relationship
    task_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TaskRelationshipDto {
            id: task_id,
            field: TaskRelationshipField::Tags,
            right_ids: vec![t1],
        },
    )
    .unwrap();

    // Now update both scalar and relationship fields via update_with_relationships
    let mut dto = task_controller::get(&ctx.db, &task_id).unwrap().unwrap();
    assert_eq!(dto.tags, vec![t1]);
    dto.title = "Updated".into();
    dto.tags = vec![t1, t2];
    let updated =
        task_controller::update_with_relationships(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto)
            .unwrap();

    // Verify scalar changed
    assert_eq!(updated.title, "Updated");
    // Verify relationship changed
    assert_eq!(updated.tags, vec![t1, t2]);

    // Verify persisted
    let fetched = task_controller::get(&ctx.db, &task_id).unwrap().unwrap();
    assert_eq!(fetched.title, "Updated");
    assert_eq!(fetched.tags, vec![t1, t2]);
}

#[test]
fn test_update_with_relationships_does_not_affect_unmodified_relationships() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "WithComments");
    let c1 = helpers::create_comment(&mut ctx, task_id, "Comment1");

    // Verify comment exists
    let rel =
        task_controller::get_relationship(&ctx.db, &task_id, &TaskRelationshipField::Comments)
            .unwrap();
    assert_eq!(rel, vec![c1]);

    // Update with relationships, keeping comments the same
    let mut dto = task_controller::get(&ctx.db, &task_id).unwrap().unwrap();
    dto.title = "NewTitle".into();
    // dto.comments stays as [c1]
    let updated =
        task_controller::update_with_relationships(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto)
            .unwrap();

    assert_eq!(updated.title, "NewTitle");
    assert_eq!(updated.comments, vec![c1]);
}

#[test]
fn test_scalar_update_does_not_change_relationships() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "ScalarOnly");
    let t1 = helpers::create_tag(&mut ctx, s.workspace_id, "Keep", "#000");

    task_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TaskRelationshipDto {
            id: task_id,
            field: TaskRelationshipField::Tags,
            right_ids: vec![t1],
        },
    )
    .unwrap();

    // Scalar-only update via UpdateTaskDto
    let dto = task_controller::get(&ctx.db, &task_id).unwrap().unwrap();
    let mut update_dto: UpdateTaskDto = dto.into();
    update_dto.title = "Changed".into();
    task_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();

    // Tags should be unchanged
    let rel =
        task_controller::get_relationship(&ctx.db, &task_id, &TaskRelationshipField::Tags).unwrap();
    assert_eq!(rel, vec![t1]);
}

// ---------------------------------------------------------------------------
// list model reorder via different methods
// ---------------------------------------------------------------------------

#[test]
fn test_reorder_by_move_relationship() {
    let (mut ctx, s) = setup();
    let a = helpers::create_task(&mut ctx, s.project_id, "A");
    let b = helpers::create_task(&mut ctx, s.project_id, "B");
    let c = helpers::create_task(&mut ctx, s.project_id, "C");

    project_controller::move_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
        &[c],
        0,
    )
    .unwrap();

    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert_eq!(rel, vec![c, a, b]);
}

#[test]
fn test_reorder_by_set_relationship() {
    let (mut ctx, s) = setup();
    let a = helpers::create_task(&mut ctx, s.project_id, "A");
    let b = helpers::create_task(&mut ctx, s.project_id, "B");
    let c = helpers::create_task(&mut ctx, s.project_id, "C");

    project_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &ProjectRelationshipDto {
            id: s.project_id,
            field: ProjectRelationshipField::Tasks,
            right_ids: vec![c, a, b],
        },
    )
    .unwrap();

    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert_eq!(rel, vec![c, a, b]);
}
