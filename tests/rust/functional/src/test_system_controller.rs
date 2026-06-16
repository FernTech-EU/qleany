// Functional tests for System controller (non-undoable entity with weak relationships)

use crate::helpers::{self, TestContext};
use direct_access::*;

fn setup() -> (TestContext, helpers::Scaffold) {
    let mut ctx = TestContext::new();
    let scaffold = helpers::create_scaffold(&mut ctx);
    (ctx, scaffold)
}

// ---------------------------------------------------------------------------
// create / get / update / remove (non-undoable — no undo params)
// ---------------------------------------------------------------------------

#[test]
fn test_create() {
    let (ctx, s) = setup();
    let sys = system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "MySys".into(),
            ..Default::default()
        },
        s.root_id,
        -1,
    )
    .unwrap();
    assert!(sys.id > 0);
    assert_eq!(sys.name, "MySys");
}

#[test]
fn test_get_by_id() {
    let (ctx, s) = setup();
    let sys = system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "GetSys".into(),
            ..Default::default()
        },
        s.root_id,
        -1,
    )
    .unwrap();
    let fetched = system_controller::get(&ctx.db, &sys.id).unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "GetSys");
}

#[test]
fn test_get_non_existent() {
    let (ctx, _) = setup();
    assert!(system_controller::get(&ctx.db, &999999).unwrap().is_none());
}

#[test]
fn test_get_all() {
    let (ctx, s) = setup();
    system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "AllSys".into(),
            ..Default::default()
        },
        s.root_id,
        -1,
    )
    .unwrap();
    let all = system_controller::get_all(&ctx.db).unwrap();
    assert!(all.len() >= 1);
}

#[test]
fn test_update() {
    let (ctx, s) = setup();
    let sys = system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "OldName".into(),
            ..Default::default()
        },
        s.root_id,
        -1,
    )
    .unwrap();
    let dto = system_controller::get(&ctx.db, &sys.id).unwrap().unwrap();
    let mut update_dto: UpdateSystemDto = dto.into();
    update_dto.name = "NewName".into();
    let updated = system_controller::update(&ctx.db, &ctx.hub, &update_dto).unwrap();
    assert_eq!(updated.name, "NewName");

    let fetched = system_controller::get(&ctx.db, &sys.id).unwrap().unwrap();
    assert_eq!(fetched.name, "NewName");
}

#[test]
fn test_remove() {
    let (ctx, s) = setup();
    let sys = system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "ToDelete".into(),
            ..Default::default()
        },
        s.root_id,
        -1,
    )
    .unwrap();
    system_controller::remove(&ctx.db, &ctx.hub, &sys.id).unwrap();
    assert!(system_controller::get(&ctx.db, &sys.id).unwrap().is_none());
}

// ---------------------------------------------------------------------------
// relationships: FavoriteProjects (ordered_one_to_many weak)
// ---------------------------------------------------------------------------

#[test]
fn test_set_and_get_favorite_projects() {
    let (mut ctx, s) = setup();
    let sys = system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "WithFavorites".into(),
            ..Default::default()
        },
        s.root_id,
        -1,
    )
    .unwrap();

    let proj2_id = helpers::create_project(&mut ctx, s.workspace_id, "Fav2");

    system_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &SystemRelationshipDto {
            id: sys.id,
            field: SystemRelationshipField::FavoriteProjects,
            right_ids: vec![s.project_id, proj2_id],
        },
    )
    .unwrap();

    let rel = system_controller::get_relationship(
        &ctx.db,
        &sys.id,
        &SystemRelationshipField::FavoriteProjects,
    )
    .unwrap();
    assert_eq!(rel, vec![s.project_id, proj2_id]);
}

#[test]
fn test_favorite_projects_order_preserved() {
    let (mut ctx, s) = setup();
    let sys = system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "Ordered".into(),
            ..Default::default()
        },
        s.root_id,
        -1,
    )
    .unwrap();

    let p2_id = helpers::create_project(&mut ctx, s.workspace_id, "P2");
    let p3_id = helpers::create_project(&mut ctx, s.workspace_id, "P3");

    system_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &SystemRelationshipDto {
            id: sys.id,
            field: SystemRelationshipField::FavoriteProjects,
            right_ids: vec![p3_id, p2_id, s.project_id],
        },
    )
    .unwrap();

    let rel = system_controller::get_relationship(
        &ctx.db,
        &sys.id,
        &SystemRelationshipField::FavoriteProjects,
    )
    .unwrap();
    assert_eq!(rel, vec![p3_id, p2_id, s.project_id]);
}

#[test]
fn test_move_favorite_projects() {
    let (mut ctx, s) = setup();
    let sys = system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "Move".into(),
            ..Default::default()
        },
        s.root_id,
        -1,
    )
    .unwrap();

    let p2_id = helpers::create_project(&mut ctx, s.workspace_id, "P2");

    system_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &SystemRelationshipDto {
            id: sys.id,
            field: SystemRelationshipField::FavoriteProjects,
            right_ids: vec![s.project_id, p2_id],
        },
    )
    .unwrap();

    system_controller::move_relationship(
        &ctx.db,
        &ctx.hub,
        &sys.id,
        &SystemRelationshipField::FavoriteProjects,
        &[p2_id],
        0,
    )
    .unwrap();

    let rel = system_controller::get_relationship(
        &ctx.db,
        &sys.id,
        &SystemRelationshipField::FavoriteProjects,
    )
    .unwrap();
    assert_eq!(rel, vec![p2_id, s.project_id]);
}

// ---------------------------------------------------------------------------
// relationships: PinnedTags (one_to_many weak)
// ---------------------------------------------------------------------------

#[test]
fn test_set_and_get_pinned_tags() {
    let (mut ctx, s) = setup();
    let sys = system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "WithPins".into(),
            ..Default::default()
        },
        s.root_id,
        -1,
    )
    .unwrap();

    let tag1 = helpers::create_tag(&mut ctx, s.workspace_id, "Pin1", "#000");
    let tag2 = helpers::create_tag(&mut ctx, s.workspace_id, "Pin2", "#FFF");

    system_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &SystemRelationshipDto {
            id: sys.id,
            field: SystemRelationshipField::PinnedTags,
            right_ids: vec![tag1, tag2],
        },
    )
    .unwrap();

    let rel =
        system_controller::get_relationship(&ctx.db, &sys.id, &SystemRelationshipField::PinnedTags)
            .unwrap();
    assert_eq!(rel.len(), 2);
    assert!(rel.contains(&tag1));
    assert!(rel.contains(&tag2));
}

#[test]
fn test_get_relationship_count() {
    let (mut ctx, s) = setup();
    let sys = system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "Count".into(),
            ..Default::default()
        },
        s.root_id,
        -1,
    )
    .unwrap();

    let tag1 = helpers::create_tag(&mut ctx, s.workspace_id, "C1", "#000");
    let tag2 = helpers::create_tag(&mut ctx, s.workspace_id, "C2", "#FFF");

    system_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &SystemRelationshipDto {
            id: sys.id,
            field: SystemRelationshipField::PinnedTags,
            right_ids: vec![tag1, tag2],
        },
    )
    .unwrap();

    let count = system_controller::get_relationship_count(
        &ctx.db,
        &sys.id,
        &SystemRelationshipField::PinnedTags,
    )
    .unwrap();
    assert_eq!(count, 2);
}

// ---------------------------------------------------------------------------
// weak reference: deleting target does not cascade to system
// ---------------------------------------------------------------------------

#[test]
fn test_deleting_project_does_not_delete_system() {
    let (mut ctx, s) = setup();
    let sys = system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "Survives".into(),
            ..Default::default()
        },
        s.root_id,
        -1,
    )
    .unwrap();

    system_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &SystemRelationshipDto {
            id: sys.id,
            field: SystemRelationshipField::FavoriteProjects,
            right_ids: vec![s.project_id],
        },
    )
    .unwrap();

    project_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &s.project_id).unwrap();

    let fetched = system_controller::get(&ctx.db, &sys.id).unwrap();
    assert!(fetched.is_some());
}
