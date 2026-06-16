// Functional tests for is_list entity fields (Vec<T> round-trip through create/get/update)

use crate::helpers::TestContext;
use direct_access::*;

/// Create Root → Workspace (no project — tests create their own).
fn setup() -> (TestContext, u64) {
    let mut ctx = TestContext::new();

    let root =
        root_controller::create_orphan(&ctx.db, &ctx.hub, &CreateRootDto::default()).unwrap();

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

    (ctx, ws.id)
}

fn create_project_with(ctx: &mut TestContext, ws_id: u64, dto: CreateProjectDto) -> ProjectDto {
    project_controller::create(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto, ws_id, -1).unwrap()
}

fn default_create_dto(title: &str) -> CreateProjectDto {
    CreateProjectDto {
        title: title.into(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// default (empty lists)
// ---------------------------------------------------------------------------

#[test]
fn test_list_fields_default_empty() {
    let (mut ctx, ws_id) = setup();
    let proj = create_project_with(&mut ctx, ws_id, default_create_dto("Empty"));

    assert!(proj.labels.is_empty());
    assert!(proj.scores.is_empty());
    assert!(proj.version_ids.is_empty());
    assert!(proj.milestone_dates.is_empty());
    assert!(proj.participant_counts.is_empty());
    assert!(proj.retry_counts.is_empty());
    assert!(proj.feature_flags.is_empty());
}

// ---------------------------------------------------------------------------
// create with values, then get
// ---------------------------------------------------------------------------

#[test]
fn test_create_with_string_list() {
    let (mut ctx, ws_id) = setup();
    let proj = create_project_with(
        &mut ctx,
        ws_id,
        CreateProjectDto {
            title: "Strings".into(),
            labels: vec!["alpha".into(), "beta".into(), "gamma".into()],
            ..Default::default()
        },
    );

    assert_eq!(proj.labels, vec!["alpha", "beta", "gamma"]);

    let fetched = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    assert_eq!(fetched.labels, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn test_create_with_float_list() {
    let (mut ctx, ws_id) = setup();
    let proj = create_project_with(
        &mut ctx,
        ws_id,
        CreateProjectDto {
            title: "Floats".into(),
            scores: vec![1.5, 2.7, 3.14],
            ..Default::default()
        },
    );

    assert_eq!(proj.scores, vec![1.5, 2.7, 3.14]);

    let fetched = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    assert_eq!(fetched.scores, vec![1.5, 2.7, 3.14]);
}

#[test]
fn test_create_with_uuid_list() {
    let (mut ctx, ws_id) = setup();
    let u1 = uuid::Uuid::new_v4();
    let u2 = uuid::Uuid::new_v4();

    let proj = create_project_with(
        &mut ctx,
        ws_id,
        CreateProjectDto {
            title: "Uuids".into(),
            version_ids: vec![u1, u2],
            ..Default::default()
        },
    );

    assert_eq!(proj.version_ids, vec![u1, u2]);

    let fetched = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    assert_eq!(fetched.version_ids, vec![u1, u2]);
}

#[test]
fn test_create_with_datetime_list() {
    let (mut ctx, ws_id) = setup();
    let d1 = chrono::Utc::now();
    let d2 = d1 + chrono::Duration::hours(1);

    let proj = create_project_with(
        &mut ctx,
        ws_id,
        CreateProjectDto {
            title: "Dates".into(),
            milestone_dates: vec![d1, d2],
            ..Default::default()
        },
    );

    assert_eq!(proj.milestone_dates.len(), 2);

    let fetched = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    assert_eq!(fetched.milestone_dates.len(), 2);
}

#[test]
fn test_create_with_integer_list() {
    let (mut ctx, ws_id) = setup();
    let proj = create_project_with(
        &mut ctx,
        ws_id,
        CreateProjectDto {
            title: "Ints".into(),
            participant_counts: vec![10, 20, 30],
            ..Default::default()
        },
    );

    assert_eq!(proj.participant_counts, vec![10, 20, 30]);

    let fetched = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    assert_eq!(fetched.participant_counts, vec![10, 20, 30]);
}

#[test]
fn test_create_with_uinteger_list() {
    let (mut ctx, ws_id) = setup();
    let proj = create_project_with(
        &mut ctx,
        ws_id,
        CreateProjectDto {
            title: "Uints".into(),
            retry_counts: vec![100, 200],
            ..Default::default()
        },
    );

    assert_eq!(proj.retry_counts, vec![100, 200]);

    let fetched = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    assert_eq!(fetched.retry_counts, vec![100, 200]);
}

#[test]
fn test_create_with_bool_list() {
    let (mut ctx, ws_id) = setup();
    let proj = create_project_with(
        &mut ctx,
        ws_id,
        CreateProjectDto {
            title: "Bools".into(),
            feature_flags: vec![true, false, true],
            ..Default::default()
        },
    );

    assert_eq!(proj.feature_flags, vec![true, false, true]);

    let fetched = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    assert_eq!(fetched.feature_flags, vec![true, false, true]);
}

// ---------------------------------------------------------------------------
// update list fields
// ---------------------------------------------------------------------------

#[test]
fn test_update_string_list() {
    let (mut ctx, ws_id) = setup();
    let proj = create_project_with(&mut ctx, ws_id, default_create_dto("Update"));

    let dto = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    let mut update_dto: UpdateProjectDto = dto.into();
    update_dto.labels = vec!["x".into(), "y".into()];
    let updated =
        project_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.labels, vec!["x", "y"]);

    let fetched = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    assert_eq!(fetched.labels, vec!["x", "y"]);
}

#[test]
fn test_update_list_to_empty() {
    let (mut ctx, ws_id) = setup();
    let proj = create_project_with(
        &mut ctx,
        ws_id,
        CreateProjectDto {
            title: "ClearMe".into(),
            labels: vec!["a".into(), "b".into()],
            ..Default::default()
        },
    );

    let dto = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    assert_eq!(dto.labels.len(), 2);

    let mut update_dto: UpdateProjectDto = dto.into();
    update_dto.labels = vec![];
    let updated =
        project_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert!(updated.labels.is_empty());

    let fetched = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    assert!(fetched.labels.is_empty());
}

#[test]
fn test_update_all_list_types() {
    let (mut ctx, ws_id) = setup();
    let proj = create_project_with(&mut ctx, ws_id, default_create_dto("AllTypes"));

    let u1 = uuid::Uuid::new_v4();
    let d1 = chrono::Utc::now();

    let dto = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    let mut update_dto: UpdateProjectDto = dto.into();
    update_dto.labels = vec!["updated".into()];
    update_dto.scores = vec![9.9];
    update_dto.version_ids = vec![u1];
    update_dto.milestone_dates = vec![d1];
    update_dto.participant_counts = vec![42];
    update_dto.retry_counts = vec![7];
    update_dto.feature_flags = vec![false, true];

    let updated =
        project_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.labels, vec!["updated"]);
    assert_eq!(updated.scores, vec![9.9]);
    assert_eq!(updated.version_ids, vec![u1]);
    assert_eq!(updated.milestone_dates.len(), 1);
    assert_eq!(updated.participant_counts, vec![42]);
    assert_eq!(updated.retry_counts, vec![7]);
    assert_eq!(updated.feature_flags, vec![false, true]);

    // Verify persistence
    let fetched = project_controller::get(&ctx.db, &proj.id).unwrap().unwrap();
    assert_eq!(fetched.labels, vec!["updated"]);
    assert_eq!(fetched.scores, vec![9.9]);
    assert_eq!(fetched.version_ids, vec![u1]);
    assert_eq!(fetched.participant_counts, vec![42]);
    assert_eq!(fetched.retry_counts, vec![7]);
    assert_eq!(fetched.feature_flags, vec![false, true]);
}
