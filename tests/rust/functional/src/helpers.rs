// Shared test helpers: context creation, scaffold builders.

use common::database::db_context::DbContext;
use common::event::EventHub;
use common::undo_redo::UndoRedoManager;
use std::sync::Arc;

use direct_access::*;

/// All the infrastructure a test needs.
pub struct TestContext {
    pub db: DbContext,
    pub hub: Arc<EventHub>,
    pub undo: UndoRedoManager,
}

impl TestContext {
    pub fn new() -> Self {
        let db = DbContext::new().expect("Failed to create in-memory DB");
        let hub = Arc::new(EventHub::new());
        let mut undo = UndoRedoManager::new();
        undo.set_event_hub(&hub);
        TestContext { db, hub, undo }
    }
}

/// IDs returned by `create_scaffold`.
pub struct Scaffold {
    pub root_id: u64,
    pub workspace_id: u64,
    pub project_id: u64,
    pub project_settings_id: u64,
}

/// Build Root → Workspace → Project → ProjectSettings.
pub fn create_scaffold(ctx: &mut TestContext) -> Scaffold {
    // Root (non-undoable orphan)
    let root =
        root_controller::create_orphan(&ctx.db, &ctx.hub, &CreateRootDto::default()).unwrap();

    // Workspace (undoable, child of Root)
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root.id,
        -1,
    )
    .unwrap();

    // Project (undoable, child of Workspace)
    let proj = project_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateProjectDto {
            title: "TestProject".into(),
            ..Default::default()
        },
        ws.id,
        -1,
    )
    .unwrap();

    // ProjectSettings (one-to-one strong required child of Project)
    let settings = project_settings_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateProjectSettingsDto::default(),
        proj.id,
        -1,
    )
    .unwrap();

    Scaffold {
        root_id: root.id,
        workspace_id: ws.id,
        project_id: proj.id,
        project_settings_id: settings.id,
    }
}

/// Create a project (with settings) under the given workspace.
pub fn create_project(ctx: &mut TestContext, ws_id: u64, title: &str) -> u64 {
    let proj = project_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateProjectDto {
            title: title.into(),
            ..Default::default()
        },
        ws_id,
        -1,
    )
    .unwrap();
    // Auto-create required ProjectSettings
    project_settings_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateProjectSettingsDto::default(),
        proj.id,
        -1,
    )
    .unwrap();
    proj.id
}

/// Create a task under the given project.
pub fn create_task(ctx: &mut TestContext, project_id: u64, title: &str) -> u64 {
    let dto = CreateTaskDto {
        title: title.into(),
        ..Default::default()
    };
    let task =
        task_controller::create(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto, project_id, -1)
            .unwrap();
    task.id
}

/// Create a task under the given project on a specific undo stack.
pub fn create_task_on_stack(
    ctx: &mut TestContext,
    project_id: u64,
    title: &str,
    stack_id: Option<u64>,
) -> u64 {
    let dto = CreateTaskDto {
        title: title.into(),
        ..Default::default()
    };
    let task = task_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        stack_id,
        &dto,
        project_id,
        -1,
    )
    .unwrap();
    task.id
}

/// Create a tag under the given workspace.
pub fn create_tag(ctx: &mut TestContext, ws_id: u64, name: &str, color: &str) -> u64 {
    let dto = CreateTagDto {
        name: name.into(),
        color: color.into(),
        ..Default::default()
    };
    let tag =
        tag_controller::create(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto, ws_id, -1).unwrap();
    tag.id
}

/// Create a comment under the given task.
pub fn create_comment(ctx: &mut TestContext, task_id: u64, text: &str) -> u64 {
    let dto = CreateCommentDto {
        text: text.into(),
        author_name: "Tester".into(),
        ..Default::default()
    };
    let comment =
        comment_controller::create(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto, task_id, -1)
            .unwrap();
    comment.id
}

/// Create a category under the given workspace.
pub fn create_category(ctx: &mut TestContext, ws_id: u64, name: &str) -> u64 {
    let dto = CreateCategoryDto {
        name: name.into(),
        ..Default::default()
    };
    let cat = category_controller::create(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto, ws_id, -1)
        .unwrap();
    cat.id
}

/// Create a project settings under the given project.
pub fn create_project_settings(ctx: &mut TestContext, project_id: u64) -> u64 {
    let dto = CreateProjectSettingsDto::default();
    let settings = project_settings_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &dto,
        project_id,
        -1,
    )
    .unwrap();
    settings.id
}

/// Create a team member under the given workspace.
pub fn create_team_member(ctx: &mut TestContext, ws_id: u64, name: &str, email: &str) -> u64 {
    let dto = CreateTeamMemberDto {
        name: name.into(),
        email: email.into(),
        ..Default::default()
    };
    let member =
        team_member_controller::create(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto, ws_id, -1)
            .unwrap();
    member.id
}
