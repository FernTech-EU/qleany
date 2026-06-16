// Functional tests for Workspace controller (hub entity with multiple relationship types)

use crate::helpers::{self, TestContext};
use direct_access::*;

fn setup() -> (TestContext, u64) {
    let mut ctx = TestContext::new();
    let root =
        root_controller::create_orphan(&ctx.db, &ctx.hub, &CreateRootDto::default()).unwrap();
    (ctx, root.id)
}

// ---------------------------------------------------------------------------
// create
// ---------------------------------------------------------------------------

#[test]
fn test_create_with_owner() {
    let (mut ctx, root_id) = setup();
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root_id,
        -1,
    )
    .unwrap();
    assert!(ws.id > 0);
}

// ---------------------------------------------------------------------------
// get
// ---------------------------------------------------------------------------

#[test]
fn test_get_by_id() {
    let (mut ctx, root_id) = setup();
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root_id,
        -1,
    )
    .unwrap();
    let fetched = workspace_controller::get(&ctx.db, &ws.id).unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, ws.id);
}

#[test]
fn test_get_non_existent() {
    let (ctx, _) = setup();
    assert!(
        workspace_controller::get(&ctx.db, &999999)
            .unwrap()
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

#[test]
fn test_update() {
    let (mut ctx, root_id) = setup();
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root_id,
        -1,
    )
    .unwrap();
    let dto = workspace_controller::get(&ctx.db, &ws.id).unwrap().unwrap();
    let update_dto: UpdateWorkspaceDto = dto.into();
    let updated =
        workspace_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.id, ws.id);
}

// ---------------------------------------------------------------------------
// remove
// ---------------------------------------------------------------------------

#[test]
fn test_remove() {
    let (mut ctx, root_id) = setup();
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root_id,
        -1,
    )
    .unwrap();
    workspace_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &ws.id).unwrap();
    assert!(
        workspace_controller::get(&ctx.db, &ws.id)
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_remove_cascades_children() {
    let (mut ctx, root_id) = setup();
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root_id,
        -1,
    )
    .unwrap();

    let proj_id = helpers::create_project(&mut ctx, ws.id, "Child");
    let tag = tag_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTagDto {
            name: "Tag".into(),
            color: "#000".into(),
            ..Default::default()
        },
        ws.id,
        -1,
    )
    .unwrap();
    let cat = category_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateCategoryDto {
            name: "Cat".into(),
            ..Default::default()
        },
        ws.id,
        -1,
    )
    .unwrap();
    let member = team_member_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTeamMemberDto {
            name: "M".into(),
            email: "m@t.com".into(),
            ..Default::default()
        },
        ws.id,
        -1,
    )
    .unwrap();

    workspace_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &ws.id).unwrap();

    assert!(
        project_controller::get(&ctx.db, &proj_id)
            .unwrap()
            .is_none()
    );
    assert!(tag_controller::get(&ctx.db, &tag.id).unwrap().is_none());
    assert!(
        category_controller::get(&ctx.db, &cat.id)
            .unwrap()
            .is_none()
    );
    assert!(
        team_member_controller::get(&ctx.db, &member.id)
            .unwrap()
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// relationships: Projects (ordered_one_to_many strong)
// ---------------------------------------------------------------------------

#[test]
fn test_get_relationship_projects() {
    let (mut ctx, root_id) = setup();
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root_id,
        -1,
    )
    .unwrap();

    let p1 = helpers::create_project(&mut ctx, ws.id, "P1");
    let p2 = helpers::create_project(&mut ctx, ws.id, "P2");

    let rel = workspace_controller::get_relationship(
        &ctx.db,
        &ws.id,
        &WorkspaceRelationshipField::Projects,
    )
    .unwrap();
    assert_eq!(rel, vec![p1, p2]);
}

#[test]
fn test_move_relationship_projects() {
    let (mut ctx, root_id) = setup();
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root_id,
        -1,
    )
    .unwrap();

    let p1 = helpers::create_project(&mut ctx, ws.id, "A");
    let p2 = helpers::create_project(&mut ctx, ws.id, "B");

    workspace_controller::move_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &ws.id,
        &WorkspaceRelationshipField::Projects,
        &[p2],
        0,
    )
    .unwrap();

    let rel = workspace_controller::get_relationship(
        &ctx.db,
        &ws.id,
        &WorkspaceRelationshipField::Projects,
    )
    .unwrap();
    assert_eq!(rel, vec![p2, p1]);
}

// ---------------------------------------------------------------------------
// relationships: Categories (one_to_many strong, unordered)
// ---------------------------------------------------------------------------

#[test]
fn test_get_relationship_categories() {
    let (mut ctx, root_id) = setup();
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root_id,
        -1,
    )
    .unwrap();

    let c1 = category_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateCategoryDto {
            name: "Cat1".into(),
            ..Default::default()
        },
        ws.id,
        -1,
    )
    .unwrap();
    let c2 = category_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateCategoryDto {
            name: "Cat2".into(),
            ..Default::default()
        },
        ws.id,
        -1,
    )
    .unwrap();

    let rel = workspace_controller::get_relationship(
        &ctx.db,
        &ws.id,
        &WorkspaceRelationshipField::Categories,
    )
    .unwrap();
    assert_eq!(rel.len(), 2);
    assert!(rel.contains(&c1.id));
    assert!(rel.contains(&c2.id));
}

// ---------------------------------------------------------------------------
// relationships: Tags (one_to_many strong)
// ---------------------------------------------------------------------------

#[test]
fn test_get_relationship_tags() {
    let (mut ctx, root_id) = setup();
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root_id,
        -1,
    )
    .unwrap();

    let t1 = tag_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTagDto {
            name: "T1".into(),
            color: "#000".into(),
            ..Default::default()
        },
        ws.id,
        -1,
    )
    .unwrap();

    let rel =
        workspace_controller::get_relationship(&ctx.db, &ws.id, &WorkspaceRelationshipField::Tags)
            .unwrap();
    assert!(rel.contains(&t1.id));
}

// ---------------------------------------------------------------------------
// relationships: TeamMembers (one_to_many strong)
// ---------------------------------------------------------------------------

#[test]
fn test_get_relationship_team_members() {
    let (mut ctx, root_id) = setup();
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root_id,
        -1,
    )
    .unwrap();

    let m1 = team_member_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTeamMemberDto {
            name: "M1".into(),
            email: "m1@t.com".into(),
            ..Default::default()
        },
        ws.id,
        -1,
    )
    .unwrap();

    let rel = workspace_controller::get_relationship(
        &ctx.db,
        &ws.id,
        &WorkspaceRelationshipField::TeamMembers,
    )
    .unwrap();
    assert!(rel.contains(&m1.id));
}

// ---------------------------------------------------------------------------
// relationship count
// ---------------------------------------------------------------------------

#[test]
fn test_get_relationship_count() {
    let (mut ctx, root_id) = setup();
    let ws = workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root_id,
        -1,
    )
    .unwrap();

    helpers::create_project(&mut ctx, ws.id, "P1");
    helpers::create_project(&mut ctx, ws.id, "P2");

    let count = workspace_controller::get_relationship_count(
        &ctx.db,
        &ws.id,
        &WorkspaceRelationshipField::Projects,
    )
    .unwrap();
    assert_eq!(count, 2);
}
