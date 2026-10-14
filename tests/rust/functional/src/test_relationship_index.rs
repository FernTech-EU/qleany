//! The reverse index behind the exclusive-relationship checks.
//!
//! Exclusive relationships (one-to-one, one-to-many, ordered one-to-many)
//! answer "does another source already hold this target?" from a reverse index
//! kept beside each junction. Every path that writes a junction must keep that
//! index exact, so each test below rebuilds it from the rows after every kind
//! of mutation (`check_junction_indexes`) as well as checking the behaviour
//! the index exists to answer.
use crate::helpers::{self, TestContext};
use common::database::transactions::Transaction;
use common::direct_access::repository_factory;
use common::event::EventBuffer;
use direct_access::*;

fn assert_index_exact(ctx: &TestContext, step: &str) {
    if let Err(problem) = ctx.db.get_store().check_junction_indexes() {
        panic!("after {step}: {problem}");
    }
}

/// `featured_tags` is a weak one-to-many: exclusive, but deleting its source
/// leaves the tags alone, so the index is what has to free them.
fn set_featured(ctx: &mut TestContext, workspace: u64, tags: Vec<u64>) -> anyhow::Result<()> {
    workspace_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &WorkspaceRelationshipDto {
            id: workspace,
            field: WorkspaceRelationshipField::FeaturedTags,
            right_ids: tags,
        },
    )
}

fn featured(ctx: &TestContext, workspace: u64) -> Vec<u64> {
    workspace_controller::get_relationship(
        &ctx.db,
        &workspace,
        &WorkspaceRelationshipField::FeaturedTags,
    )
    .unwrap()
}

#[test]
fn a_batch_can_transfer_a_target_but_a_lone_write_cannot_take_one() {
    let mut ctx = TestContext::new();
    let a = helpers::create_scaffold(&mut ctx);
    let b = helpers::create_scaffold(&mut ctx);
    let tag_a = helpers::create_tag(&mut ctx, a.workspace_id, "A", "red");
    let tag_b = helpers::create_tag(&mut ctx, b.workspace_id, "B", "blue");
    let c = helpers::create_scaffold(&mut ctx);
    assert_index_exact(&ctx, "creating three owners and two tags");

    let mut tx = Transaction::begin_write_transaction(&ctx.db).unwrap();
    let mut repo = repository_factory::write::create_workspace_repository(&tx).unwrap();
    let mut events = EventBuffer::new();
    let tags = WorkspaceRelationshipField::Tags;

    // Both sources are replaced in the batch, so A's tag may move to B.
    repo.set_relationship_multi(
        &mut events,
        &tags,
        vec![
            (a.workspace_id, vec![]),
            (b.workspace_id, vec![tag_b, tag_a]),
        ],
    )
    .unwrap();
    assert_index_exact(&ctx, "a transfer inside one batch");

    // B holds it now and is not in these batches: neither A alone, nor A
    // beside another source being replaced, may take it.
    for batch in [
        vec![(a.workspace_id, vec![tag_a])],
        vec![(a.workspace_id, vec![tag_a]), (c.workspace_id, vec![])],
    ] {
        assert!(
            repo.set_relationship_multi(&mut events, &tags, batch)
                .is_err()
        );
    }
    // A target claimed twice inside one batch is refused too, even when both
    // claimants are being replaced.
    assert!(
        repo.set_relationship_multi(
            &mut events,
            &tags,
            vec![(a.workspace_id, vec![tag_a]), (b.workspace_id, vec![tag_a])],
        )
        .is_err()
    );
    assert_eq!(
        repo.get_relationship(&a.workspace_id, &tags).unwrap(),
        Vec::<u64>::new()
    );
    assert_eq!(
        repo.get_relationship(&b.workspace_id, &tags).unwrap(),
        vec![tag_b, tag_a]
    );
    assert_index_exact(&ctx, "three refused batches");

    // The same transfer back, written in the other order.
    repo.set_relationship_multi(
        &mut events,
        &tags,
        vec![(b.workspace_id, vec![tag_b]), (a.workspace_id, vec![tag_a])],
    )
    .unwrap();
    drop(repo);
    tx.commit().unwrap();
    assert_index_exact(&ctx, "a transfer back");
    assert_eq!(
        workspace_controller::get_relationship(&ctx.db, &a.workspace_id, &tags).unwrap(),
        vec![tag_a]
    );
}

#[test]
fn removing_a_source_frees_its_targets_and_undo_takes_them_back() {
    let mut ctx = TestContext::new();
    let a = helpers::create_scaffold(&mut ctx);
    let b = helpers::create_scaffold(&mut ctx);
    let tag = helpers::create_tag(&mut ctx, a.workspace_id, "Shared", "red");

    set_featured(&mut ctx, b.workspace_id, vec![tag]).unwrap();
    assert!(set_featured(&mut ctx, a.workspace_id, vec![tag]).is_err());
    assert_index_exact(&ctx, "featuring a tag on B");

    // Deleting B removes its rows; the tag (A's) survives and is free again.
    workspace_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &b.workspace_id).unwrap();
    assert_index_exact(&ctx, "removing the source");
    assert!(tag_controller::get(&ctx.db, &tag).unwrap().is_some());
    set_featured(&mut ctx, a.workspace_id, vec![tag]).unwrap();
    assert_index_exact(&ctx, "featuring the freed tag on A");

    // Undo both: B comes back from its snapshot holding the tag again.
    ctx.undo.undo(None).unwrap();
    assert_index_exact(&ctx, "undoing A's write");
    ctx.undo.undo(None).unwrap();
    assert_index_exact(&ctx, "undoing the removal (scoped snapshot restore)");
    assert_eq!(featured(&ctx, b.workspace_id), vec![tag]);
    assert!(featured(&ctx, a.workspace_id).is_empty());
    assert!(set_featured(&mut ctx, a.workspace_id, vec![tag]).is_err());

    // Redo both: B is gone again and A holds the tag.
    ctx.undo.redo(None).unwrap();
    assert_index_exact(&ctx, "redoing the removal");
    ctx.undo.redo(None).unwrap();
    assert_index_exact(&ctx, "redoing A's write");
    assert_eq!(featured(&ctx, a.workspace_id), vec![tag]);
}

#[test]
fn removing_a_target_clears_it_from_every_row_and_from_the_index() {
    let mut ctx = TestContext::new();
    let a = helpers::create_scaffold(&mut ctx);
    let b = helpers::create_scaffold(&mut ctx);
    let tag = helpers::create_tag(&mut ctx, a.workspace_id, "Doomed", "red");
    let keep = helpers::create_tag(&mut ctx, a.workspace_id, "Kept", "blue");
    set_featured(&mut ctx, a.workspace_id, vec![keep, tag]).unwrap();
    // Shared (many-to-many) rows hold it too, from both workspaces.
    for workspace in [a.workspace_id, b.workspace_id] {
        workspace_controller::set_relationship(
            &ctx.db,
            &ctx.hub,
            &mut ctx.undo,
            None,
            &WorkspaceRelationshipDto {
                id: workspace,
                field: WorkspaceRelationshipField::SelectedTags,
                right_ids: vec![tag],
            },
        )
        .unwrap();
    }
    assert_index_exact(&ctx, "linking the tag from three rows");

    tag_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &tag).unwrap();
    assert_index_exact(&ctx, "removing the target");
    assert_eq!(featured(&ctx, a.workspace_id), vec![keep]);

    ctx.undo.undo(None).unwrap();
    assert_index_exact(&ctx, "undoing the target's removal");
    assert_eq!(featured(&ctx, a.workspace_id), vec![keep, tag]);
    assert!(set_featured(&mut ctx, b.workspace_id, vec![tag]).is_err());
}

#[test]
fn a_rolled_back_transaction_and_a_restored_savepoint_restore_the_index() {
    let mut ctx = TestContext::new();
    let a = helpers::create_scaffold(&mut ctx);
    let b = helpers::create_scaffold(&mut ctx);
    let tag = helpers::create_tag(&mut ctx, a.workspace_id, "Shared", "red");
    set_featured(&mut ctx, a.workspace_id, vec![tag]).unwrap();
    let featured_tags = WorkspaceRelationshipField::FeaturedTags;
    let give_to_b = vec![(a.workspace_id, vec![]), (b.workspace_id, vec![tag])];

    // Rollback: the transfer is undone wholesale, index included.
    let mut tx = Transaction::begin_write_transaction(&ctx.db).unwrap();
    let mut repo = repository_factory::write::create_workspace_repository(&tx).unwrap();
    let mut events = EventBuffer::new();
    repo.set_relationship_multi(&mut events, &featured_tags, give_to_b.clone())
        .unwrap();
    assert_index_exact(&ctx, "a transfer inside an open transaction");
    drop(repo);
    tx.rollback().unwrap();
    assert_index_exact(&ctx, "rolling the transaction back");
    assert_eq!(featured(&ctx, a.workspace_id), vec![tag]);
    assert!(set_featured(&mut ctx, b.workspace_id, vec![tag]).is_err());

    // A savepoint inside a transaction that then commits.
    let mut tx = Transaction::begin_write_transaction(&ctx.db).unwrap();
    let savepoint = tx.create_savepoint().unwrap();
    let mut repo = repository_factory::write::create_workspace_repository(&tx).unwrap();
    repo.set_relationship_multi(&mut events, &featured_tags, give_to_b.clone())
        .unwrap();
    drop(repo);
    tx.restore_to_savepoint(savepoint).unwrap();
    assert_index_exact(&ctx, "restoring a savepoint");
    let mut repo = repository_factory::write::create_workspace_repository(&tx).unwrap();
    assert!(
        repo.set_relationship(&mut events, &b.workspace_id, &featured_tags, &[tag])
            .is_err()
    );
    drop(repo);
    tx.commit().unwrap();

    // A whole-store snapshot, restored as an undo snapshot is.
    let mut tx = Transaction::begin_write_transaction(&ctx.db).unwrap();
    let snapshot = tx.snapshot_store();
    let mut repo = repository_factory::write::create_workspace_repository(&tx).unwrap();
    repo.set_relationship_multi(&mut events, &featured_tags, give_to_b)
        .unwrap();
    drop(repo);
    tx.restore_store(&snapshot);
    tx.commit().unwrap();
    assert_index_exact(&ctx, "restoring a whole-store snapshot");
    assert_eq!(featured(&ctx, a.workspace_id), vec![tag]);
    assert!(featured(&ctx, b.workspace_id).is_empty());
}

#[test]
fn one_to_one_targets_are_freed_by_removal_and_checked_on_every_write_path() {
    let mut ctx = TestContext::new();
    let s = helpers::create_scaffold(&mut ctx);
    let other = helpers::create_project(&mut ctx, s.workspace_id, "Other");
    let lead = helpers::create_team_member(&mut ctx, s.workspace_id, "Lead", "lead@example.org");
    let set_lead = |ctx: &mut TestContext, project: u64| {
        project_controller::set_relationship(
            &ctx.db,
            &ctx.hub,
            &mut ctx.undo,
            None,
            &ProjectRelationshipDto {
                id: project,
                field: ProjectRelationshipField::Lead,
                right_ids: vec![lead],
            },
        )
    };
    set_lead(&mut ctx, s.project_id).unwrap();
    assert_index_exact(&ctx, "setting a one-to-one");

    // Taken: through set_relationship, update_with_relationships and create.
    assert!(set_lead(&mut ctx, other).is_err());
    let mut dto = project_controller::get(&ctx.db, &other).unwrap().unwrap();
    dto.lead = Some(lead);
    let refused =
        project_controller::update_with_relationships(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto)
            .unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("One-to-one constraint violation"),
        "{refused}"
    );
    assert!(
        project_controller::create(
            &ctx.db,
            &ctx.hub,
            &mut ctx.undo,
            None,
            &CreateProjectDto {
                lead: Some(lead),
                ..Default::default()
            },
            s.workspace_id,
            -1,
        )
        .is_err()
    );
    assert_index_exact(&ctx, "three refused writes");

    // Removing the holder frees the member for the other project.
    project_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &s.project_id).unwrap();
    assert_index_exact(&ctx, "removing the one-to-one holder");
    project_controller::update_with_relationships(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto)
        .unwrap();
    assert_index_exact(&ctx, "update_with_relationships taking the freed target");
    assert_eq!(
        project_controller::get(&ctx.db, &other)
            .unwrap()
            .unwrap()
            .lead,
        Some(lead)
    );
}

#[test]
fn ordered_moves_and_list_updates_keep_the_index_exact() {
    let mut ctx = TestContext::new();
    let s = helpers::create_scaffold(&mut ctx);
    let first = helpers::create_task(&mut ctx, s.project_id, "First");
    let second = helpers::create_task(&mut ctx, s.project_id, "Second");
    let third = helpers::create_task(&mut ctx, s.project_id, "Third");
    let comment = helpers::create_comment(&mut ctx, first, "Note");
    assert_index_exact(&ctx, "creating children under their owners");

    project_controller::move_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &s.project_id,
        &ProjectRelationshipField::Tasks,
        &[third],
        0,
    )
    .unwrap();
    assert_index_exact(&ctx, "moving inside an ordered relationship");
    ctx.undo.undo(None).unwrap();
    assert_index_exact(&ctx, "undoing the move");

    // Hand the comment from one task to another in one update of each.
    let mut from = task_controller::get(&ctx.db, &first).unwrap().unwrap();
    let mut to = task_controller::get(&ctx.db, &second).unwrap().unwrap();
    to.comments = vec![comment];
    assert!(
        task_controller::update_with_relationships(&ctx.db, &ctx.hub, &mut ctx.undo, None, &to)
            .is_err()
    );
    from.comments = vec![];
    task_controller::update_with_relationships_multi(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &[from, to],
    )
    .unwrap();
    assert_index_exact(&ctx, "a transfer through update_with_relationships_multi");
    assert_eq!(
        task_controller::get(&ctx.db, &second)
            .unwrap()
            .unwrap()
            .comments,
        vec![comment]
    );
    ctx.undo.undo(None).unwrap();
    assert_index_exact(&ctx, "undoing the transfer");
    assert_eq!(
        task_controller::get(&ctx.db, &first)
            .unwrap()
            .unwrap()
            .comments,
        vec![comment]
    );
}
