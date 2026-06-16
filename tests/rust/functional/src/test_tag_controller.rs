// Functional tests for Tag controller (entity with owner, no children)

use crate::helpers::{self, TestContext};
use direct_access::*;

fn setup() -> (TestContext, helpers::Scaffold) {
    let mut ctx = TestContext::new();
    let scaffold = helpers::create_scaffold(&mut ctx);
    (ctx, scaffold)
}

#[test]
fn test_create_with_owner() {
    let (mut ctx, s) = setup();
    let tag = tag_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTagDto {
            name: "Tag1".into(),
            color: "#FF0000".into(),
            ..Default::default()
        },
        s.workspace_id,
        -1,
    )
    .unwrap();
    assert!(tag.id > 0);
    assert_eq!(tag.name, "Tag1");
    assert_eq!(tag.color, "#FF0000");
}

#[test]
fn test_create_multiple() {
    let (mut ctx, s) = setup();
    let tags = tag_controller::create_multi(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &[
            CreateTagDto {
                name: "A".into(),
                color: "#000".into(),
                ..Default::default()
            },
            CreateTagDto {
                name: "B".into(),
                color: "#111".into(),
                ..Default::default()
            },
        ],
        s.workspace_id,
        -1,
    )
    .unwrap();
    assert_eq!(tags.len(), 2);
    assert_ne!(tags[0].id, tags[1].id);
}

#[test]
fn test_get_by_id() {
    let (mut ctx, s) = setup();
    let tag = tag_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTagDto {
            name: "Get".into(),
            color: "#000".into(),
            ..Default::default()
        },
        s.workspace_id,
        -1,
    )
    .unwrap();
    let fetched = tag_controller::get(&ctx.db, &tag.id).unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "Get");
}

#[test]
fn test_get_non_existent() {
    let (ctx, _) = setup();
    let fetched = tag_controller::get(&ctx.db, &999999).unwrap();
    assert!(fetched.is_none());
}

#[test]
fn test_get_all() {
    let (mut ctx, s) = setup();
    tag_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTagDto {
            name: "All".into(),
            color: "#000".into(),
            ..Default::default()
        },
        s.workspace_id,
        -1,
    )
    .unwrap();
    let all = tag_controller::get_all(&ctx.db).unwrap();
    assert!(all.len() >= 1);
}

#[test]
fn test_update_fields() {
    let (mut ctx, s) = setup();
    let tag = tag_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTagDto {
            name: "Old".into(),
            color: "#000".into(),
            ..Default::default()
        },
        s.workspace_id,
        -1,
    )
    .unwrap();

    let dto = tag_controller::get(&ctx.db, &tag.id).unwrap().unwrap();
    let mut update_dto: UpdateTagDto = dto.into();
    update_dto.name = "New".into();
    update_dto.color = "#FFF".into();
    let updated =
        tag_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.name, "New");
    assert_eq!(updated.color, "#FFF");

    let fetched = tag_controller::get(&ctx.db, &tag.id).unwrap().unwrap();
    assert_eq!(fetched.name, "New");
    assert_eq!(fetched.color, "#FFF");
}

#[test]
fn test_remove() {
    let (mut ctx, s) = setup();
    let tag = tag_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateTagDto {
            name: "Del".into(),
            color: "#000".into(),
            ..Default::default()
        },
        s.workspace_id,
        -1,
    )
    .unwrap();
    tag_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &tag.id).unwrap();
    let fetched = tag_controller::get(&ctx.db, &tag.id).unwrap();
    assert!(fetched.is_none());
}

#[test]
fn test_remove_multiple() {
    let (mut ctx, s) = setup();
    let tags = tag_controller::create_multi(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &[
            CreateTagDto {
                name: "D1".into(),
                color: "#000".into(),
                ..Default::default()
            },
            CreateTagDto {
                name: "D2".into(),
                color: "#000".into(),
                ..Default::default()
            },
        ],
        s.workspace_id,
        -1,
    )
    .unwrap();
    let ids: Vec<u64> = tags.iter().map(|t| t.id).collect();
    tag_controller::remove_multi(&ctx.db, &ctx.hub, &mut ctx.undo, None, &ids).unwrap();
    for id in &ids {
        assert!(tag_controller::get(&ctx.db, id).unwrap().is_none());
    }
}
