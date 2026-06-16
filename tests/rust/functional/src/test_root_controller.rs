// Functional tests for Root controller (orphan entity — no owner, has relationships)

use crate::helpers::TestContext;
use direct_access::*;

#[test]
fn test_create_orphan() {
    let ctx = TestContext::new();
    let root =
        root_controller::create_orphan(&ctx.db, &ctx.hub, &CreateRootDto::default()).unwrap();
    assert!(root.id > 0);
}

#[test]
fn test_create_multiple_orphans() {
    let ctx = TestContext::new();
    let roots = root_controller::create_orphan_multi(
        &ctx.db,
        &ctx.hub,
        &[CreateRootDto::default(), CreateRootDto::default()],
    )
    .unwrap();
    assert_eq!(roots.len(), 2);
    assert_ne!(roots[0].id, roots[1].id);
}

#[test]
fn test_get_by_id() {
    let ctx = TestContext::new();
    let created =
        root_controller::create_orphan(&ctx.db, &ctx.hub, &CreateRootDto::default()).unwrap();
    let fetched = root_controller::get(&ctx.db, &created.id).unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, created.id);
}

#[test]
fn test_get_non_existent() {
    let ctx = TestContext::new();
    let fetched = root_controller::get(&ctx.db, &999999).unwrap();
    assert!(fetched.is_none());
}

#[test]
fn test_get_all() {
    let ctx = TestContext::new();
    root_controller::create_orphan(&ctx.db, &ctx.hub, &CreateRootDto::default()).unwrap();
    let all = root_controller::get_all(&ctx.db).unwrap();
    assert!(all.len() >= 1);
}

#[test]
fn test_update() {
    let ctx = TestContext::new();
    let created =
        root_controller::create_orphan(&ctx.db, &ctx.hub, &CreateRootDto::default()).unwrap();
    let created_id = created.id;
    let update_dto: UpdateRootDto = created.into();
    let updated = root_controller::update(&ctx.db, &ctx.hub, &update_dto).unwrap();
    assert_eq!(updated.id, created_id);
    let fetched = root_controller::get(&ctx.db, &created_id).unwrap().unwrap();
    assert_eq!(fetched.id, created_id);
}

#[test]
fn test_remove() {
    let ctx = TestContext::new();
    let created =
        root_controller::create_orphan(&ctx.db, &ctx.hub, &CreateRootDto::default()).unwrap();
    root_controller::remove(&ctx.db, &ctx.hub, &created.id).unwrap();
    let fetched = root_controller::get(&ctx.db, &created.id).unwrap();
    assert!(fetched.is_none());
}

#[test]
fn test_remove_non_existent() {
    let ctx = TestContext::new();
    // Should not panic
    let _ = root_controller::remove(&ctx.db, &ctx.hub, &999999);
}

#[test]
fn test_set_and_get_relationship_workspace() {
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

    let rel_ids =
        root_controller::get_relationship(&ctx.db, &root.id, &RootRelationshipField::Workspace)
            .unwrap();
    assert_eq!(rel_ids, vec![ws.id]);
}

#[test]
fn test_set_and_get_relationship_system() {
    let ctx = TestContext::new();
    let root =
        root_controller::create_orphan(&ctx.db, &ctx.hub, &CreateRootDto::default()).unwrap();

    let sys = system_controller::create(
        &ctx.db,
        &ctx.hub,
        &CreateSystemDto {
            name: "TestSys".into(),
            ..Default::default()
        },
        root.id,
        -1,
    )
    .unwrap();

    let rel_ids =
        root_controller::get_relationship(&ctx.db, &root.id, &RootRelationshipField::System)
            .unwrap();
    assert_eq!(rel_ids, vec![sys.id]);
}

#[test]
fn test_get_relationship_count() {
    let mut ctx = TestContext::new();
    let root =
        root_controller::create_orphan(&ctx.db, &ctx.hub, &CreateRootDto::default()).unwrap();
    workspace_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateWorkspaceDto::default(),
        root.id,
        -1,
    )
    .unwrap();

    let count = root_controller::get_relationship_count(
        &ctx.db,
        &root.id,
        &RootRelationshipField::Workspace,
    )
    .unwrap();
    assert_eq!(count, 1);
}
