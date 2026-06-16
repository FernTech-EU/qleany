// Functional tests for Category controller (leaf entity, target of many_to_one relationships)

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
    let cat = category_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateCategoryDto {
            name: "Backend".into(),
            description: "Backend team".into(),
            icon: "server".into(),
            ..Default::default()
        },
        s.workspace_id,
        -1,
    )
    .unwrap();
    assert!(cat.id > 0);
    assert_eq!(cat.name, "Backend");
    assert_eq!(cat.description, "Backend team");
    assert_eq!(cat.icon, "server");
}

#[test]
fn test_create_multiple() {
    let (mut ctx, s) = setup();
    let cats = category_controller::create_multi(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &[
            CreateCategoryDto {
                name: "A".into(),
                ..Default::default()
            },
            CreateCategoryDto {
                name: "B".into(),
                ..Default::default()
            },
        ],
        s.workspace_id,
        -1,
    )
    .unwrap();
    assert_eq!(cats.len(), 2);
    assert_ne!(cats[0].id, cats[1].id);
}

// ---------------------------------------------------------------------------
// get
// ---------------------------------------------------------------------------

#[test]
fn test_get_by_id() {
    let (mut ctx, s) = setup();
    let id = helpers::create_category(&mut ctx, s.workspace_id, "GetCat");
    let fetched = category_controller::get(&ctx.db, &id).unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "GetCat");
}

#[test]
fn test_get_non_existent() {
    let (ctx, _) = setup();
    assert!(
        category_controller::get(&ctx.db, &999999)
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_get_all() {
    let (mut ctx, s) = setup();
    helpers::create_category(&mut ctx, s.workspace_id, "AllCat");
    let all = category_controller::get_all(&ctx.db).unwrap();
    assert!(all.len() >= 1);
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

#[test]
fn test_update_fields() {
    let (mut ctx, s) = setup();
    let id = helpers::create_category(&mut ctx, s.workspace_id, "OldCat");
    let dto = category_controller::get(&ctx.db, &id).unwrap().unwrap();
    let mut update_dto: UpdateCategoryDto = dto.into();
    update_dto.name = "NewCat".into();
    update_dto.description = "Updated".into();
    update_dto.icon = "folder".into();
    let updated =
        category_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.name, "NewCat");
    assert_eq!(updated.description, "Updated");
    assert_eq!(updated.icon, "folder");
}

// ---------------------------------------------------------------------------
// remove
// ---------------------------------------------------------------------------

#[test]
fn test_remove() {
    let (mut ctx, s) = setup();
    let id = helpers::create_category(&mut ctx, s.workspace_id, "DelCat");
    category_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &id).unwrap();
    assert!(category_controller::get(&ctx.db, &id).unwrap().is_none());
}

// ---------------------------------------------------------------------------
// referential integrity: deleting category doesn't delete team member
// ---------------------------------------------------------------------------

#[test]
fn test_delete_category_leaves_team_member_intact() {
    let (mut ctx, s) = setup();
    let cat_id = helpers::create_category(&mut ctx, s.workspace_id, "ToDelete");
    let member_id = helpers::create_team_member(&mut ctx, s.workspace_id, "Member", "m@test.com");

    // Set department relationship
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

    // Delete the category
    category_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &cat_id).unwrap();

    // Team member still exists
    let member = team_member_controller::get(&ctx.db, &member_id).unwrap();
    assert!(member.is_some());
    assert_eq!(member.unwrap().name, "Member");
}

#[test]
fn test_delete_category_does_not_delete_team_member() {
    // Deleting a weak reference target does not affect the referencing entity
    let (mut ctx, s) = setup();
    let cat_id = helpers::create_category(&mut ctx, s.workspace_id, "WillGo");
    let member_id = helpers::create_team_member(&mut ctx, s.workspace_id, "Ref", "ref@test.com");

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

    // Delete category
    category_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &cat_id).unwrap();

    // Team member still exists
    let member = team_member_controller::get(&ctx.db, &member_id).unwrap();
    assert!(member.is_some());
}
