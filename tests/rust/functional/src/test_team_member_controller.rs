// Functional tests for TeamMember controller (many_to_one relationship, enum field)

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
    let member = team_member_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTeamMemberDto {
            name: "Alice".into(),
            email: "alice@test.com".into(),
            role: common::entities::MemberRole::Developer,
            ..Default::default()
        },
        s.workspace_id,
        -1,
    )
    .unwrap();
    assert!(member.id > 0);
    assert_eq!(member.name, "Alice");
    assert_eq!(member.email, "alice@test.com");
    assert_eq!(member.role, common::entities::MemberRole::Developer);
}

#[test]
fn test_create_multiple() {
    let (mut ctx, s) = setup();
    let members = team_member_controller::create_multi(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &[
            CreateTeamMemberDto {
                name: "Bob".into(),
                email: "bob@test.com".into(),
                role: common::entities::MemberRole::Designer,
                ..Default::default()
            },
            CreateTeamMemberDto {
                name: "Carol".into(),
                email: "carol@test.com".into(),
                role: common::entities::MemberRole::Tester,
                ..Default::default()
            },
        ],
        s.workspace_id,
        -1,
    )
    .unwrap();
    assert_eq!(members.len(), 2);
    assert_ne!(members[0].id, members[1].id);
}

// ---------------------------------------------------------------------------
// get
// ---------------------------------------------------------------------------

#[test]
fn test_get_by_id() {
    let (mut ctx, s) = setup();
    let id = helpers::create_team_member(&mut ctx, s.workspace_id, "Get", "get@test.com");
    let fetched = team_member_controller::get(&ctx.db, &id).unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "Get");
}

#[test]
fn test_get_non_existent() {
    let (ctx, _) = setup();
    assert!(
        team_member_controller::get(&ctx.db, &999999)
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_get_all() {
    let (mut ctx, s) = setup();
    helpers::create_team_member(&mut ctx, s.workspace_id, "All", "all@test.com");
    let all = team_member_controller::get_all(&ctx.db).unwrap();
    assert!(all.len() >= 1);
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

#[test]
fn test_update_fields() {
    let (mut ctx, s) = setup();
    let id = helpers::create_team_member(&mut ctx, s.workspace_id, "Old", "old@test.com");
    let dto = team_member_controller::get(&ctx.db, &id).unwrap().unwrap();
    let mut update_dto: UpdateTeamMemberDto = dto.into();
    update_dto.name = "New".into();
    update_dto.email = "new@test.com".into();
    let updated =
        team_member_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto)
            .unwrap();
    assert_eq!(updated.name, "New");
    assert_eq!(updated.email, "new@test.com");
}

#[test]
fn test_update_enum_role() {
    let (mut ctx, s) = setup();
    let id = helpers::create_team_member(&mut ctx, s.workspace_id, "Enum", "enum@test.com");
    let dto = team_member_controller::get(&ctx.db, &id).unwrap().unwrap();
    let mut update_dto: UpdateTeamMemberDto = dto.into();
    update_dto.role = common::entities::MemberRole::Manager;
    let updated =
        team_member_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto)
            .unwrap();
    assert_eq!(updated.role, common::entities::MemberRole::Manager);
}

// ---------------------------------------------------------------------------
// remove
// ---------------------------------------------------------------------------

#[test]
fn test_remove() {
    let (mut ctx, s) = setup();
    let id = helpers::create_team_member(&mut ctx, s.workspace_id, "Del", "del@test.com");
    team_member_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &id).unwrap();
    assert!(team_member_controller::get(&ctx.db, &id).unwrap().is_none());
}

// ---------------------------------------------------------------------------
// relationships: Department (many_to_one weak)
// ---------------------------------------------------------------------------

#[test]
fn test_set_and_get_department() {
    let (mut ctx, s) = setup();
    let member_id = helpers::create_team_member(&mut ctx, s.workspace_id, "Dept", "dept@test.com");
    let cat_id = helpers::create_category(&mut ctx, s.workspace_id, "Engineering");

    team_member_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TeamMemberRelationshipDto {
            id: member_id,
            field: TeamMemberRelationshipField::Department,
            right_ids: vec![cat_id],
        },
    )
    .unwrap();

    let rel = team_member_controller::get_relationship(
        &ctx.db,
        &member_id,
        &TeamMemberRelationshipField::Department,
    )
    .unwrap();
    assert_eq!(rel, vec![cat_id]);
}

#[test]
fn test_change_department() {
    let (mut ctx, s) = setup();
    let member_id =
        helpers::create_team_member(&mut ctx, s.workspace_id, "Switch", "switch@test.com");
    let cat1 = helpers::create_category(&mut ctx, s.workspace_id, "Dept1");
    let cat2 = helpers::create_category(&mut ctx, s.workspace_id, "Dept2");

    team_member_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TeamMemberRelationshipDto {
            id: member_id,
            field: TeamMemberRelationshipField::Department,
            right_ids: vec![cat1],
        },
    )
    .unwrap();

    // Change to different department
    team_member_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TeamMemberRelationshipDto {
            id: member_id,
            field: TeamMemberRelationshipField::Department,
            right_ids: vec![cat2],
        },
    )
    .unwrap();

    let rel = team_member_controller::get_relationship(
        &ctx.db,
        &member_id,
        &TeamMemberRelationshipField::Department,
    )
    .unwrap();
    assert_eq!(rel, vec![cat2]);
}

#[test]
fn test_multiple_members_same_department() {
    let (mut ctx, s) = setup();
    let m1 = helpers::create_team_member(&mut ctx, s.workspace_id, "M1", "m1@test.com");
    let m2 = helpers::create_team_member(&mut ctx, s.workspace_id, "M2", "m2@test.com");
    let cat = helpers::create_category(&mut ctx, s.workspace_id, "Shared");

    team_member_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TeamMemberRelationshipDto {
            id: m1,
            field: TeamMemberRelationshipField::Department,
            right_ids: vec![cat],
        },
    )
    .unwrap();
    team_member_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TeamMemberRelationshipDto {
            id: m2,
            field: TeamMemberRelationshipField::Department,
            right_ids: vec![cat],
        },
    )
    .unwrap();

    let rel1 = team_member_controller::get_relationship(
        &ctx.db,
        &m1,
        &TeamMemberRelationshipField::Department,
    )
    .unwrap();
    let rel2 = team_member_controller::get_relationship(
        &ctx.db,
        &m2,
        &TeamMemberRelationshipField::Department,
    )
    .unwrap();
    assert_eq!(rel1, vec![cat]);
    assert_eq!(rel2, vec![cat]);
}
