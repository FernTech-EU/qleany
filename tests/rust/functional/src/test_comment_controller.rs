// Functional tests for Comment controller (standalone CRUD for simple owned entity)

use crate::helpers::{self, TestContext};
use direct_access::*;

fn setup() -> (TestContext, helpers::Scaffold, u64) {
    let mut ctx = TestContext::new();
    let scaffold = helpers::create_scaffold(&mut ctx);
    let task_id = helpers::create_task(&mut ctx, scaffold.project_id, "ParentTask");
    (ctx, scaffold, task_id)
}

// ---------------------------------------------------------------------------
// create
// ---------------------------------------------------------------------------

#[test]
fn test_create_with_owner() {
    let (mut ctx, _, task_id) = setup();
    let comment = comment_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateCommentDto {
            text: "Hello world".into(),
            author_name: "Alice".into(),
            ..Default::default()
        },
        task_id,
        -1,
    )
    .unwrap();
    assert!(comment.id > 0);
    assert_eq!(comment.text, "Hello world");
    assert_eq!(comment.author_name, "Alice");
}

#[test]
fn test_create_multiple() {
    let (mut ctx, _, task_id) = setup();
    let comments = comment_controller::create_multi(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &[
            CreateCommentDto {
                text: "C1".into(),
                author_name: "A".into(),
                ..Default::default()
            },
            CreateCommentDto {
                text: "C2".into(),
                author_name: "B".into(),
                ..Default::default()
            },
        ],
        task_id,
        -1,
    )
    .unwrap();
    assert_eq!(comments.len(), 2);
    assert_ne!(comments[0].id, comments[1].id);
}

#[test]
fn test_create_at_index() {
    let (mut ctx, _, task_id) = setup();
    helpers::create_comment(&mut ctx, task_id, "First");
    let inserted = comment_controller::create(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &CreateCommentDto {
            text: "Inserted".into(),
            author_name: "A".into(),
            ..Default::default()
        },
        task_id,
        0,
    )
    .unwrap();

    let rel =
        task_controller::get_relationship(&ctx.db, &task_id, &TaskRelationshipField::Comments)
            .unwrap();
    assert_eq!(rel[0], inserted.id);
}

// ---------------------------------------------------------------------------
// get
// ---------------------------------------------------------------------------

#[test]
fn test_get_by_id() {
    let (mut ctx, _, task_id) = setup();
    let id = helpers::create_comment(&mut ctx, task_id, "GetMe");
    let fetched = comment_controller::get(&ctx.db, &id).unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().text, "GetMe");
}

#[test]
fn test_get_non_existent() {
    let (ctx, _, _) = setup();
    assert!(comment_controller::get(&ctx.db, &999999).unwrap().is_none());
}

#[test]
fn test_get_all() {
    let (mut ctx, _, task_id) = setup();
    helpers::create_comment(&mut ctx, task_id, "All");
    let all = comment_controller::get_all(&ctx.db).unwrap();
    assert!(all.len() >= 1);
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

#[test]
fn test_update_fields() {
    let (mut ctx, _, task_id) = setup();
    let id = helpers::create_comment(&mut ctx, task_id, "Old");
    let dto = comment_controller::get(&ctx.db, &id).unwrap().unwrap();
    let mut update_dto: UpdateCommentDto = dto.into();
    update_dto.text = "New".into();
    update_dto.author_name = "Bob".into();
    let updated =
        comment_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();
    assert_eq!(updated.text, "New");
    assert_eq!(updated.author_name, "Bob");

    let fetched = comment_controller::get(&ctx.db, &id).unwrap().unwrap();
    assert_eq!(fetched.text, "New");
    assert_eq!(fetched.author_name, "Bob");
}

// ---------------------------------------------------------------------------
// remove
// ---------------------------------------------------------------------------

#[test]
fn test_remove() {
    let (mut ctx, _, task_id) = setup();
    let id = helpers::create_comment(&mut ctx, task_id, "Del");
    comment_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &id).unwrap();
    assert!(comment_controller::get(&ctx.db, &id).unwrap().is_none());
}

#[test]
fn test_remove_multiple() {
    let (mut ctx, _, task_id) = setup();
    let c1 = helpers::create_comment(&mut ctx, task_id, "D1");
    let c2 = helpers::create_comment(&mut ctx, task_id, "D2");
    comment_controller::remove_multi(&ctx.db, &ctx.hub, &mut ctx.undo, None, &[c1, c2]).unwrap();
    assert!(comment_controller::get(&ctx.db, &c1).unwrap().is_none());
    assert!(comment_controller::get(&ctx.db, &c2).unwrap().is_none());
}

// ---------------------------------------------------------------------------
// ordering
// ---------------------------------------------------------------------------

#[test]
fn test_comments_ordered_in_parent() {
    let (mut ctx, _, task_id) = setup();
    let c1 = helpers::create_comment(&mut ctx, task_id, "First");
    let c2 = helpers::create_comment(&mut ctx, task_id, "Second");
    let c3 = helpers::create_comment(&mut ctx, task_id, "Third");

    let rel =
        task_controller::get_relationship(&ctx.db, &task_id, &TaskRelationshipField::Comments)
            .unwrap();
    assert_eq!(rel, vec![c1, c2, c3]);
}
