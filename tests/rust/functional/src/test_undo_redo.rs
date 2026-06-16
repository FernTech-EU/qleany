// Functional tests for Undo/Redo snapshot/restore system.
// Builds entity trees and verifies undo/redo correctly restores
// entities and relationships.

use crate::helpers::{self, TestContext};
use direct_access::*;

fn setup() -> (TestContext, helpers::Scaffold) {
    let mut ctx = TestContext::new();
    let scaffold = helpers::create_scaffold(&mut ctx);
    (ctx, scaffold)
}

// ---------------------------------------------------------------------------
// Basic undo/redo
// ---------------------------------------------------------------------------

#[test]
fn test_undo_create_task() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "UndoMe");
    assert!(task_controller::get(&ctx.db, &task_id).unwrap().is_some());

    ctx.undo.undo(None).unwrap();

    assert!(task_controller::get(&ctx.db, &task_id).unwrap().is_none());
    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert!(!rel.contains(&task_id));
}

#[test]
fn test_redo_create_task() {
    let (mut ctx, s) = setup();
    let _task_id = helpers::create_task(&mut ctx, s.project_id, "RedoMe");

    ctx.undo.undo(None).unwrap();
    ctx.undo.redo(None).unwrap();

    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert!(!rel.is_empty());
    let restored = task_controller::get(&ctx.db, &rel[0]).unwrap().unwrap();
    assert_eq!(restored.title, "RedoMe");
}

#[test]
fn test_undo_remove_task() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "RemoveAndRestore");

    task_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &task_id).unwrap();
    assert!(task_controller::get(&ctx.db, &task_id).unwrap().is_none());

    ctx.undo.undo(None).unwrap();

    let fetched = task_controller::get(&ctx.db, &task_id).unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().title, "RemoveAndRestore");

    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert!(rel.contains(&task_id));
}

#[test]
fn test_undo_update_task() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "OriginalTitle");

    let dto = task_controller::get(&ctx.db, &task_id).unwrap().unwrap();
    let mut update_dto: UpdateTaskDto = dto.into();
    update_dto.title = "UpdatedTitle".into();
    update_dto.content = "UpdatedContent".into();
    task_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();

    ctx.undo.undo(None).unwrap();

    let fetched = task_controller::get(&ctx.db, &task_id).unwrap().unwrap();
    assert_eq!(fetched.title, "OriginalTitle");
}

// ---------------------------------------------------------------------------
// update_with_relationships undo/redo
// ---------------------------------------------------------------------------

#[test]
fn test_undo_update_with_relationships() {
    let (mut ctx, s) = setup();
    let task_id = helpers::create_task(&mut ctx, s.project_id, "Original");
    let t1 = helpers::create_tag(&mut ctx, s.workspace_id, "TagA", "#000");
    let t2 = helpers::create_tag(&mut ctx, s.workspace_id, "TagB", "#FFF");

    task_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TaskRelationshipDto {
            id: task_id,
            field: TaskRelationshipField::Tags,
            right_ids: vec![t1],
        },
    )
    .unwrap();

    // Update both scalar and relationship via update_with_relationships
    let mut dto = task_controller::get(&ctx.db, &task_id).unwrap().unwrap();
    dto.title = "Changed".into();
    dto.tags = vec![t1, t2];
    task_controller::update_with_relationships(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto)
        .unwrap();

    // Verify changes applied
    let fetched = task_controller::get(&ctx.db, &task_id).unwrap().unwrap();
    assert_eq!(fetched.title, "Changed");
    assert_eq!(fetched.tags, vec![t1, t2]);

    // Undo
    ctx.undo.undo(None).unwrap();

    // Verify both scalar and relationship restored
    let restored = task_controller::get(&ctx.db, &task_id).unwrap().unwrap();
    assert_eq!(restored.title, "Original");
    assert_eq!(restored.tags, vec![t1]);

    // Redo
    ctx.undo.redo(None).unwrap();

    let redone = task_controller::get(&ctx.db, &task_id).unwrap().unwrap();
    assert_eq!(redone.title, "Changed");
    assert_eq!(redone.tags, vec![t1, t2]);
}

// ---------------------------------------------------------------------------
// Relationship undo/redo
// ---------------------------------------------------------------------------

#[test]
fn test_undo_set_relationship_ids() {
    let (mut ctx, s) = setup();
    let tag_a = helpers::create_tag(&mut ctx, s.workspace_id, "TagA", "#AA0000");
    let tag_b = helpers::create_tag(&mut ctx, s.workspace_id, "TagB", "#00BB00");

    project_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &ProjectRelationshipDto {
            id: s.project_id,
            field: ProjectRelationshipField::Tags,
            right_ids: vec![tag_a, tag_b],
        },
    )
    .unwrap();

    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tags,
    )
    .unwrap();
    assert_eq!(rel.len(), 2);

    ctx.undo.undo(None).unwrap();

    let after = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tags,
    )
    .unwrap();
    assert!(after.is_empty());
}

#[test]
fn test_undo_move_relationship_ids() {
    let (mut ctx, s) = setup();
    let a = helpers::create_task(&mut ctx, s.project_id, "A");
    let b = helpers::create_task(&mut ctx, s.project_id, "B");
    let c = helpers::create_task(&mut ctx, s.project_id, "C");

    let orig = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert_eq!(orig, vec![a, b, c]);

    project_controller::move_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
        &[c],
        0,
    )
    .unwrap();

    let moved = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert_eq!(moved, vec![c, a, b]);

    ctx.undo.undo(None).unwrap();

    let restored = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert_eq!(restored, vec![a, b, c]);
}

// ---------------------------------------------------------------------------
// Cascade delete undo
// ---------------------------------------------------------------------------

#[test]
fn test_undo_cascade_remove_project() {
    let (mut ctx, s) = setup();
    let task_a = helpers::create_task(&mut ctx, s.project_id, "CascadeA");
    let task_b = helpers::create_task(&mut ctx, s.project_id, "CascadeB");
    let comment_id = helpers::create_comment(&mut ctx, task_a, "Important");

    project_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &s.project_id).unwrap();

    assert!(
        project_controller::get(&ctx.db, &s.project_id)
            .unwrap()
            .is_none()
    );
    assert!(task_controller::get(&ctx.db, &task_a).unwrap().is_none());
    assert!(task_controller::get(&ctx.db, &task_b).unwrap().is_none());
    assert!(
        comment_controller::get(&ctx.db, &comment_id)
            .unwrap()
            .is_none()
    );

    ctx.undo.undo(None).unwrap();

    let proj = project_controller::get(&ctx.db, &s.project_id)
        .unwrap()
        .unwrap();
    assert_eq!(proj.title, "TestProject");

    assert_eq!(
        task_controller::get(&ctx.db, &task_a)
            .unwrap()
            .unwrap()
            .title,
        "CascadeA"
    );
    assert_eq!(
        task_controller::get(&ctx.db, &task_b)
            .unwrap()
            .unwrap()
            .title,
        "CascadeB"
    );
    assert_eq!(
        comment_controller::get(&ctx.db, &comment_id)
            .unwrap()
            .unwrap()
            .text,
        "Important"
    );

    let task_rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert!(task_rel.contains(&task_a));
    assert!(task_rel.contains(&task_b));

    let comment_rel =
        task_controller::get_relationship(&ctx.db, &task_a, &TaskRelationshipField::Comments)
            .unwrap();
    assert!(comment_rel.contains(&comment_id));
}

// ---------------------------------------------------------------------------
// Multiple operations
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_undo_redo() {
    let (mut ctx, s) = setup();

    // Clear undo stack from scaffold operations
    ctx.undo.clear_all_stacks();

    let a = helpers::create_task(&mut ctx, s.project_id, "MultiA");
    let b = helpers::create_task(&mut ctx, s.project_id, "MultiB");

    let dto = task_controller::get(&ctx.db, &a).unwrap().unwrap();
    let mut update_dto: UpdateTaskDto = dto.into();
    update_dto.title = "MultiA_Updated".into();
    task_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto).unwrap();

    // Undo update
    ctx.undo.undo(None).unwrap();
    assert_eq!(
        task_controller::get(&ctx.db, &a).unwrap().unwrap().title,
        "MultiA"
    );

    // Redo update
    ctx.undo.redo(None).unwrap();
    assert_eq!(
        task_controller::get(&ctx.db, &a).unwrap().unwrap().title,
        "MultiA_Updated"
    );

    // Undo update again
    ctx.undo.undo(None).unwrap();
    assert_eq!(
        task_controller::get(&ctx.db, &a).unwrap().unwrap().title,
        "MultiA"
    );

    // Undo create B
    ctx.undo.undo(None).unwrap();
    assert!(task_controller::get(&ctx.db, &b).unwrap().is_none());

    // Undo create A
    ctx.undo.undo(None).unwrap();
    assert!(task_controller::get(&ctx.db, &a).unwrap().is_none());

    // Redo create A
    ctx.undo.redo(None).unwrap();
    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert!(!rel.is_empty());
    let restored_a = task_controller::get(&ctx.db, &rel[0]).unwrap().unwrap();
    assert_eq!(restored_a.title, "MultiA");

    // Redo create B
    ctx.undo.redo(None).unwrap();
    let rel2 = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert_eq!(rel2.len(), 2);

    // Redo update — should restore the updated title
    ctx.undo.redo(None).unwrap();
    let restored = task_controller::get(&ctx.db, &rel[0]).unwrap().unwrap();
    assert_eq!(restored.title, "MultiA_Updated");
}

// ---------------------------------------------------------------------------
// State queries
// ---------------------------------------------------------------------------

#[test]
fn test_can_undo_can_redo() {
    let (mut ctx, s) = setup();
    ctx.undo.clear_all_stacks();

    assert!(!ctx.undo.can_undo(None));
    assert!(!ctx.undo.can_redo(None));

    helpers::create_task(&mut ctx, s.project_id, "StateTest");

    assert!(ctx.undo.can_undo(None));
    assert!(!ctx.undo.can_redo(None));

    ctx.undo.undo(None).unwrap();

    assert!(!ctx.undo.can_undo(None));
    assert!(ctx.undo.can_redo(None));

    ctx.undo.redo(None).unwrap();

    assert!(ctx.undo.can_undo(None));
    assert!(!ctx.undo.can_redo(None));
}

#[test]
fn test_undo_redo_stack_count() {
    let (mut ctx, s) = setup();
    ctx.undo.clear_all_stacks();

    assert_eq!(ctx.undo.get_stack_size(0), 0);

    helpers::create_task(&mut ctx, s.project_id, "Count1");
    assert_eq!(ctx.undo.get_stack_size(0), 1);

    helpers::create_task(&mut ctx, s.project_id, "Count2");
    assert_eq!(ctx.undo.get_stack_size(0), 2);

    ctx.undo.undo(None).unwrap();
    assert_eq!(ctx.undo.get_stack_size(0), 1);
    assert!(ctx.undo.can_redo(None));
}

// ---------------------------------------------------------------------------
// Full tree snapshot/restore
// ---------------------------------------------------------------------------

#[test]
fn test_full_tree_snapshot_restore() {
    let (mut ctx, s) = setup();

    let tag_a = helpers::create_tag(&mut ctx, s.workspace_id, "Priority", "#FF0000");
    let tag_b = helpers::create_tag(&mut ctx, s.workspace_id, "Feature", "#00FF00");

    let task1 = helpers::create_task(&mut ctx, s.project_id, "Implement login");
    let task2 = helpers::create_task(&mut ctx, s.project_id, "Write tests");
    let task3 = helpers::create_task(&mut ctx, s.project_id, "Deploy");

    // Set tags on project (many-to-many)
    project_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &ProjectRelationshipDto {
            id: s.project_id,
            field: ProjectRelationshipField::Tags,
            right_ids: vec![tag_a, tag_b],
        },
    )
    .unwrap();

    // Set tags on task1 (many-to-many)
    task_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &TaskRelationshipDto {
            id: task1,
            field: TaskRelationshipField::Tags,
            right_ids: vec![tag_a],
        },
    )
    .unwrap();

    // Add comment to task2
    let comment_id = helpers::create_comment(&mut ctx, task2, "Needs more coverage");

    // Clear stack so only next operation is undoable
    ctx.undo.clear_all_stacks();

    // Remove the project — cascades to tasks, comments
    project_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &s.project_id).unwrap();

    assert!(
        project_controller::get(&ctx.db, &s.project_id)
            .unwrap()
            .is_none()
    );
    assert!(task_controller::get(&ctx.db, &task1).unwrap().is_none());
    assert!(task_controller::get(&ctx.db, &task2).unwrap().is_none());
    assert!(task_controller::get(&ctx.db, &task3).unwrap().is_none());
    assert!(
        comment_controller::get(&ctx.db, &comment_id)
            .unwrap()
            .is_none()
    );

    // Tags still exist (weak relationship)
    assert!(tag_controller::get(&ctx.db, &tag_a).unwrap().is_some());
    assert!(tag_controller::get(&ctx.db, &tag_b).unwrap().is_some());

    // Undo
    ctx.undo.undo(None).unwrap();

    // Project restored
    let proj = project_controller::get(&ctx.db, &s.project_id)
        .unwrap()
        .unwrap();
    assert_eq!(proj.title, "TestProject");

    // Tasks restored
    assert_eq!(
        task_controller::get(&ctx.db, &task1)
            .unwrap()
            .unwrap()
            .title,
        "Implement login"
    );
    assert_eq!(
        task_controller::get(&ctx.db, &task2)
            .unwrap()
            .unwrap()
            .title,
        "Write tests"
    );
    assert_eq!(
        task_controller::get(&ctx.db, &task3)
            .unwrap()
            .unwrap()
            .title,
        "Deploy"
    );

    // Comment restored
    assert_eq!(
        comment_controller::get(&ctx.db, &comment_id)
            .unwrap()
            .unwrap()
            .text,
        "Needs more coverage"
    );

    // Project → Tasks relationship restored (ordered)
    let task_rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert_eq!(task_rel.len(), 3);
    assert_eq!(task_rel, vec![task1, task2, task3]);

    // Project → Tags relationship restored
    let proj_tag_rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tags,
    )
    .unwrap();
    assert_eq!(proj_tag_rel.len(), 2);
    assert!(proj_tag_rel.contains(&tag_a));
    assert!(proj_tag_rel.contains(&tag_b));

    // Task1 → Tags relationship restored
    let task1_tag_rel =
        task_controller::get_relationship(&ctx.db, &task1, &TaskRelationshipField::Tags).unwrap();
    assert_eq!(task1_tag_rel, vec![tag_a]);

    // Task2 → Comments relationship restored
    let task2_comment_rel =
        task_controller::get_relationship(&ctx.db, &task2, &TaskRelationshipField::Comments)
            .unwrap();
    assert!(task2_comment_rel.contains(&comment_id));
}

// ---------------------------------------------------------------------------
// Cross-trunk isolation (scoped undo restore)
//
// Regression tests for the whole-store snapshot/restore bug: undoing an operation
// on one undoable trunk used to revert every other trunk (and non-undoable data).
// Two Projects under one Workspace act as two independent trunks, each on its own
// undo stack (Some(s1) / Some(s2)).
// ---------------------------------------------------------------------------

/// The exact scenario from the bug report: remove on trunk A (stack 1), create on
/// trunk B (stack 2), undo stack 1 → B's edit must survive.
#[test]
fn test_two_trunk_remove_isolation() {
    let (mut ctx, s) = setup();
    let s1 = ctx.undo.create_new_stack();
    let s2 = ctx.undo.create_new_stack();
    let p1 = s.project_id;
    let p2 = helpers::create_project(&mut ctx, s.workspace_id, "P2");
    let a_task = helpers::create_task(&mut ctx, p1, "A-seed");
    let _b_seed = helpers::create_task(&mut ctx, p2, "B-seed");
    ctx.undo.clear_all_stacks();

    // Remove A's task on stack 1 (snapshot captured here).
    task_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, Some(s1), &a_task).unwrap();
    assert!(task_controller::get(&ctx.db, &a_task).unwrap().is_none());

    // Create a new task in B on stack 2 (after A's snapshot).
    let b_task = helpers::create_task_on_stack(&mut ctx, p2, "B-edit", Some(s2));

    // Undo stack 1 — restores A's task, must NOT touch trunk B.
    ctx.undo.undo(Some(s1)).unwrap();

    assert!(
        task_controller::get(&ctx.db, &a_task).unwrap().is_some(),
        "A restored"
    );
    assert!(
        project_controller::get_relationship(&ctx.db, &p1, &ProjectRelationshipField::Tasks)
            .unwrap()
            .contains(&a_task)
    );
    assert!(
        task_controller::get(&ctx.db, &b_task).unwrap().is_some(),
        "cross-trunk: B's stack-2 edit must survive undo of trunk A"
    );
    assert!(
        project_controller::get_relationship(&ctx.db, &p2, &ProjectRelationshipField::Tasks)
            .unwrap()
            .contains(&b_task)
    );
}

/// Undo of a create on trunk A (stack 1) must not delete trunk B's concurrent create.
#[test]
fn test_two_trunk_create_isolation() {
    let (mut ctx, s) = setup();
    let s1 = ctx.undo.create_new_stack();
    let s2 = ctx.undo.create_new_stack();
    let p1 = s.project_id;
    let p2 = helpers::create_project(&mut ctx, s.workspace_id, "P2");
    ctx.undo.clear_all_stacks();

    let a_task = helpers::create_task_on_stack(&mut ctx, p1, "A", Some(s1));
    let b_task = helpers::create_task_on_stack(&mut ctx, p2, "B", Some(s2));

    ctx.undo.undo(Some(s1)).unwrap();

    assert!(
        task_controller::get(&ctx.db, &a_task).unwrap().is_none(),
        "A's creation undone"
    );
    assert!(
        task_controller::get(&ctx.db, &b_task).unwrap().is_some(),
        "cross-trunk: B's task must survive undo of trunk A"
    );
    assert!(
        project_controller::get_relationship(&ctx.db, &p2, &ProjectRelationshipField::Tasks)
            .unwrap()
            .contains(&b_task)
    );
}

/// Undo of a create must not clobber a sibling added to the SAME owner on another
/// stack after the create (the create-undo owner-junction fix).
#[test]
fn test_create_undo_preserves_concurrent_sibling() {
    let (mut ctx, s) = setup();
    let s1 = ctx.undo.create_new_stack();
    let s2 = ctx.undo.create_new_stack();
    ctx.undo.clear_all_stacks();

    let a_task = helpers::create_task_on_stack(&mut ctx, s.project_id, "A", Some(s1));
    let sibling = helpers::create_task_on_stack(&mut ctx, s.project_id, "Sibling", Some(s2));

    ctx.undo.undo(Some(s1)).unwrap();

    assert!(task_controller::get(&ctx.db, &a_task).unwrap().is_none());
    let tasks = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert!(
        tasks.contains(&sibling),
        "sibling added on another stack must survive create-undo"
    );
    assert!(!tasks.contains(&a_task));
}

/// Undo of a remove must re-add the entity AND preserve a sibling added to the same
/// owner after the snapshot (surgical owner reconcile — the case v1.6.3 got wrong).
#[test]
fn test_remove_undo_preserves_concurrent_sibling() {
    let (mut ctx, s) = setup();
    let s1 = ctx.undo.create_new_stack();
    let s2 = ctx.undo.create_new_stack();
    let a_task = helpers::create_task(&mut ctx, s.project_id, "A");
    ctx.undo.clear_all_stacks();

    // Remove A on stack 1 (snapshot: project.tasks == [A]).
    task_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, Some(s1), &a_task).unwrap();
    // Add a sibling to the SAME project on stack 2 (after the snapshot).
    let sibling = helpers::create_task_on_stack(&mut ctx, s.project_id, "Sibling", Some(s2));

    ctx.undo.undo(Some(s1)).unwrap();

    assert!(
        task_controller::get(&ctx.db, &a_task).unwrap().is_some(),
        "A restored"
    );
    let tasks = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
    )
    .unwrap();
    assert!(tasks.contains(&a_task), "A back in owner");
    assert!(
        tasks.contains(&sibling),
        "concurrent sibling preserved (surgical owner reconcile)"
    );
}

/// `set_relationship` undo is scoped to one junction row — undoing trunk A's set must
/// not revert trunk B's set on another stack.
#[test]
fn test_two_trunk_set_relationship_isolation() {
    let (mut ctx, s) = setup();
    let s1 = ctx.undo.create_new_stack();
    let s2 = ctx.undo.create_new_stack();
    let p1 = s.project_id;
    let p2 = helpers::create_project(&mut ctx, s.workspace_id, "P2");
    let tag_a = helpers::create_tag(&mut ctx, s.workspace_id, "A", "#AA0000");
    let tag_b = helpers::create_tag(&mut ctx, s.workspace_id, "B", "#00BB00");
    ctx.undo.clear_all_stacks();

    project_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        Some(s1),
        &ProjectRelationshipDto {
            id: p1,
            field: ProjectRelationshipField::Tags,
            right_ids: vec![tag_a],
        },
    )
    .unwrap();
    project_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        Some(s2),
        &ProjectRelationshipDto {
            id: p2,
            field: ProjectRelationshipField::Tags,
            right_ids: vec![tag_b],
        },
    )
    .unwrap();

    ctx.undo.undo(Some(s1)).unwrap();

    assert!(
        project_controller::get_relationship(&ctx.db, &p1, &ProjectRelationshipField::Tags)
            .unwrap()
            .is_empty(),
        "P1's tags reverted to prior (empty) state"
    );
    assert_eq!(
        project_controller::get_relationship(&ctx.db, &p2, &ProjectRelationshipField::Tags)
            .unwrap(),
        vec![tag_b],
        "cross-trunk: P2's set_relationship survives undo of P1's"
    );
}

/// `move_relationship` undo is scoped to one junction row — undoing trunk A's reorder
/// must not revert trunk B's reorder on another stack.
#[test]
fn test_two_trunk_move_relationship_isolation() {
    let (mut ctx, s) = setup();
    let s1 = ctx.undo.create_new_stack();
    let s2 = ctx.undo.create_new_stack();
    let p1 = s.project_id;
    let p2 = helpers::create_project(&mut ctx, s.workspace_id, "P2");
    let a1 = helpers::create_task(&mut ctx, p1, "a1");
    let a2 = helpers::create_task(&mut ctx, p1, "a2");
    let b1 = helpers::create_task(&mut ctx, p2, "b1");
    let b2 = helpers::create_task(&mut ctx, p2, "b2");
    ctx.undo.clear_all_stacks();

    project_controller::move_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        Some(s1),
        &p1,
        &ProjectRelationshipField::Tasks,
        &[a2],
        0,
    )
    .unwrap();
    project_controller::move_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        Some(s2),
        &p2,
        &ProjectRelationshipField::Tasks,
        &[b2],
        0,
    )
    .unwrap();
    assert_eq!(
        project_controller::get_relationship(&ctx.db, &p1, &ProjectRelationshipField::Tasks)
            .unwrap(),
        vec![a2, a1]
    );

    ctx.undo.undo(Some(s1)).unwrap();

    assert_eq!(
        project_controller::get_relationship(&ctx.db, &p1, &ProjectRelationshipField::Tasks)
            .unwrap(),
        vec![a1, a2],
        "P1's order restored"
    );
    assert_eq!(
        project_controller::get_relationship(&ctx.db, &p2, &ProjectRelationshipField::Tasks)
            .unwrap(),
        vec![b2, b1],
        "cross-trunk: P2's move survives undo of P1's"
    );
}

/// Undo of a cascade remove restores the whole subtree (recursion through strong
/// children), with the child correctly re-linked to its parent.
#[test]
fn test_undo_remove_deep_subtree_restored() {
    let (mut ctx, s) = setup();
    let s1 = ctx.undo.create_new_stack();
    let task = helpers::create_task(&mut ctx, s.project_id, "Deep");
    let comment = helpers::create_comment(&mut ctx, task, "c1");
    ctx.undo.clear_all_stacks();

    task_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, Some(s1), &task).unwrap();
    assert!(task_controller::get(&ctx.db, &task).unwrap().is_none());
    assert!(
        comment_controller::get(&ctx.db, &comment)
            .unwrap()
            .is_none()
    );

    ctx.undo.undo(Some(s1)).unwrap();

    assert!(
        task_controller::get(&ctx.db, &task).unwrap().is_some(),
        "task restored"
    );
    assert!(
        comment_controller::get(&ctx.db, &comment)
            .unwrap()
            .is_some(),
        "child comment restored"
    );
    assert!(
        task_controller::get_relationship(&ctx.db, &task, &TaskRelationshipField::Comments)
            .unwrap()
            .contains(&comment),
        "child re-linked to parent"
    );
}
