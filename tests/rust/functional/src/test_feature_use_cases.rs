// Functional tests for feature use cases (project_management and task_management).
//
// The generated use cases contain `unimplemented!()` stubs, so these tests
// verify that the controller wiring, DTO construction, and UoW factory setup
// are correct by confirming the expected panic from the stub.

use crate::helpers::{self, TestContext};
use direct_access::*;

fn setup() -> (TestContext, helpers::Scaffold) {
    let mut ctx = TestContext::new();
    let scaffold = helpers::create_scaffold(&mut ctx);
    (ctx, scaffold)
}

// ===========================================================================
// project_management
// ===========================================================================

#[test]
#[should_panic(expected = "unimplemented")]
fn test_create_project() {
    let (mut ctx, _s) = setup();
    let dto = project_management::CreateProjectDto {
        title: "Feature Project".into(),
        is_active: true,
        priority: 3,
        ..Default::default()
    };
    let _ = project_management::project_management_controller::create_project(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &dto,
    );
}

#[test]
#[should_panic(expected = "unimplemented")]
fn test_get_project_stats() {
    let (mut ctx, s) = setup();
    helpers::create_task(&mut ctx, s.project_id, "Task1");

    let dto = project_management::GetProjectStatsDto {
        project_id: s.project_id,
    };
    let _ = project_management::project_management_controller::get_project_stats(
        &ctx.db, &ctx.hub, &dto,
    );
}

#[test]
#[should_panic(expected = "unimplemented")]
fn test_archive_project() {
    let (ctx, s) = setup();
    let dto = project_management::ArchiveProjectDto {
        project_id: s.project_id,
        reason: Some("Done".into()),
        archive_priority: Some(1),
    };
    let _ =
        project_management::project_management_controller::archive_project(&ctx.db, &ctx.hub, &dto);
}

#[test]
fn test_export_project_data_starts() {
    let (ctx, s) = setup();
    let mut long_op = common::long_operation::LongOperationManager::new();
    let dto = project_management::ExportProjectDataDto {
        project_id: s.project_id,
        format: "json".into(),
        include_comments: true,
        project_uuid: None,
    };
    let operation_id = project_management::project_management_controller::export_project_data(
        &ctx.db,
        &ctx.hub,
        &mut long_op,
        &dto,
    )
    .unwrap();
    assert!(!operation_id.is_empty());
}

#[test]
fn test_import_project_starts() {
    let (ctx, s) = setup();
    let mut long_op = common::long_operation::LongOperationManager::new();
    let dto = project_management::ImportProjectDto {
        data: "{}".into(),
        target_workspace_id: s.workspace_id,
        tag_names: vec!["imported".into()],
    };
    let operation_id = project_management::project_management_controller::import_project(
        &ctx.db,
        &ctx.hub,
        &mut long_op,
        &dto,
    )
    .unwrap();
    assert!(!operation_id.is_empty());
}

// ===========================================================================
// task_management
// ===========================================================================

#[test]
#[should_panic(expected = "unimplemented")]
fn test_batch_assign_tasks() {
    let (mut ctx, s) = setup();
    let t1 = helpers::create_task(&mut ctx, s.project_id, "Assign1");
    let member = helpers::create_team_member(&mut ctx, s.workspace_id, "Assignee", "a@t.com");

    let dto = task_management::BatchAssignTasksDto {
        task_ids: vec![t1],
        team_member_id: member,
        difficulty_filter: None,
    };
    let _ =
        task_management::task_management_controller::batch_assign_tasks(&ctx.db, &ctx.hub, &dto);
}

#[test]
#[should_panic(expected = "unimplemented")]
fn test_get_task_summary() {
    let (mut ctx, s) = setup();
    helpers::create_task(&mut ctx, s.project_id, "Summary1");

    let _ = task_management::task_management_controller::get_task_summary(&ctx.db, &ctx.hub);
}

#[test]
#[should_panic(expected = "unimplemented")]
fn test_cleanup_completed() {
    let (mut ctx, s) = setup();
    helpers::create_task(&mut ctx, s.project_id, "Cleanup1");

    let _ = task_management::task_management_controller::cleanup_completed(&ctx.db, &ctx.hub);
}
