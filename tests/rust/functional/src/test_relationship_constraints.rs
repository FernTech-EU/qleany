//! Cardinality and membership regressions shared by the generated Rust APIs.
use crate::helpers;
use direct_access::*;

#[test]
fn unordered_one_to_many_rejects_a_second_owner_without_changing_either() {
    let mut ctx = helpers::TestContext::new();
    let a = helpers::create_scaffold(&mut ctx);
    let b = helpers::create_scaffold(&mut ctx);
    let tag = helpers::create_tag(&mut ctx, a.workspace_id, "Shared", "red");
    assert!(
        workspace_controller::set_relationship(
            &ctx.db,
            &ctx.hub,
            &mut ctx.undo,
            None,
            &WorkspaceRelationshipDto {
                id: b.workspace_id,
                field: WorkspaceRelationshipField::Tags,
                right_ids: vec![tag]
            }
        )
        .is_err()
    );
    assert_eq!(
        workspace_controller::get_relationship(
            &ctx.db,
            &a.workspace_id,
            &WorkspaceRelationshipField::Tags
        )
        .unwrap(),
        vec![tag]
    );
    assert!(
        workspace_controller::get_relationship(
            &ctx.db,
            &b.workspace_id,
            &WorkspaceRelationshipField::Tags
        )
        .unwrap()
        .is_empty()
    );
    workspace_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &b.workspace_id).unwrap();
    assert!(tag_controller::get(&ctx.db, &tag).unwrap().is_some());
}

#[test]
fn many_to_many_shares_without_cascade_and_rejects_duplicate_links() {
    let mut ctx = helpers::TestContext::new();
    let a = helpers::create_scaffold(&mut ctx);
    let other = helpers::create_project(&mut ctx, a.workspace_id, "Other");
    let tag = helpers::create_tag(&mut ctx, a.workspace_id, "Shared", "red");
    for project in [a.project_id, other] {
        let mut dto = ProjectRelationshipDto {
            id: project,
            field: ProjectRelationshipField::Tags,
            right_ids: vec![tag],
        };
        project_controller::set_relationship(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto).unwrap();
        dto.right_ids.push(tag);
        assert!(
            project_controller::set_relationship(&ctx.db, &ctx.hub, &mut ctx.undo, None, &dto)
                .is_err()
        );
        assert_eq!(
            project_controller::get_relationship(
                &ctx.db,
                &project,
                &ProjectRelationshipField::Tags
            )
            .unwrap(),
            vec![tag]
        );
    }
    project_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &a.project_id).unwrap();
    assert!(tag_controller::get(&ctx.db, &tag).unwrap().is_some());
    assert_eq!(
        project_controller::get_relationship(&ctx.db, &other, &ProjectRelationshipField::Tags)
            .unwrap(),
        vec![tag]
    );
    tag_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &tag).unwrap();
    assert!(
        project_controller::get_relationship(&ctx.db, &other, &ProjectRelationshipField::Tags)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn moves_require_ordered_relationships_and_distinct_existing_members() {
    let mut ctx = helpers::TestContext::new();
    let a = helpers::create_scaffold(&mut ctx);
    let first = helpers::create_task(&mut ctx, a.project_id, "First");
    let second = helpers::create_task(&mut ctx, a.project_id, "Second");
    let other = helpers::create_project(&mut ctx, a.workspace_id, "Other");
    let foreign = helpers::create_task(&mut ctx, other, "Foreign");
    for ids in [vec![999_999], vec![foreign], vec![first, first]] {
        assert!(
            project_controller::move_relationship(
                &ctx.db,
                &ctx.hub,
                &mut ctx.undo,
                None,
                &a.project_id,
                &ProjectRelationshipField::Tasks,
                &ids,
                0
            )
            .is_err()
        );
    }
    assert_eq!(
        project_controller::get_relationship(
            &ctx.db,
            &a.project_id,
            &ProjectRelationshipField::Tasks
        )
        .unwrap(),
        vec![first, second]
    );
    assert!(
        project_controller::move_relationship(
            &ctx.db,
            &ctx.hub,
            &mut ctx.undo,
            None,
            &a.project_id,
            &ProjectRelationshipField::Tags,
            &[],
            0
        )
        .is_err()
    );
    assert!(
        workspace_controller::move_relationship(
            &ctx.db,
            &ctx.hub,
            &mut ctx.undo,
            None,
            &a.workspace_id,
            &WorkspaceRelationshipField::Tags,
            &[],
            0
        )
        .is_err()
    );
    assert_eq!(
        project_controller::move_relationship(
            &ctx.db,
            &ctx.hub,
            &mut ctx.undo,
            None,
            &a.project_id,
            &ProjectRelationshipField::Tasks,
            &[second],
            0
        )
        .unwrap(),
        vec![second, first]
    );
}

#[test]
fn batch_transfers_are_atomic_and_entity_writes_cannot_bypass_constraints() {
    use common::database::transactions::Transaction;
    use common::direct_access::repository_factory;
    use common::event::EventBuffer;
    let mut ctx = helpers::TestContext::new();
    let a = helpers::create_scaffold(&mut ctx);
    let b = helpers::create_scaffold(&mut ctx);
    let tag_a = helpers::create_tag(&mut ctx, a.workspace_id, "A", "red");
    let tag_b = helpers::create_tag(&mut ctx, b.workspace_id, "B", "blue");
    let mut tx = Transaction::begin_write_transaction(&ctx.db).unwrap();
    let mut repo = repository_factory::write::create_workspace_repository(&tx).unwrap();
    let mut events = EventBuffer::new();
    // Both duplicate sources and two new owners of one target fail before writes.
    for batch in [
        vec![(a.workspace_id, vec![]), (a.workspace_id, vec![tag_a])],
        vec![(a.workspace_id, vec![tag_b]), (b.workspace_id, vec![tag_b])],
    ] {
        assert!(
            repo.set_relationship_multi(&mut events, &WorkspaceRelationshipField::Tags, batch)
                .is_err()
        );
        assert_eq!(
            repo.get_relationship(&a.workspace_id, &WorkspaceRelationshipField::Tags)
                .unwrap(),
            vec![tag_a]
        );
        assert_eq!(
            repo.get_relationship(&b.workspace_id, &WorkspaceRelationshipField::Tags)
                .unwrap(),
            vec![tag_b]
        );
    }
    let mut entity = repo.get(&a.workspace_id).unwrap().unwrap();
    entity.tags = vec![tag_b];
    assert!(
        repo.update_with_relationships(&mut events, &entity)
            .is_err()
    );
    entity.id = 0;
    assert!(repo.create(&mut events, &entity, a.root_id, -1).is_err());
    // Each side is being replaced, so exchanging targets is valid as one batch.
    repo.set_relationship_multi(
        &mut events,
        &WorkspaceRelationshipField::Tags,
        vec![(a.workspace_id, vec![tag_b]), (b.workspace_id, vec![tag_a])],
    )
    .unwrap();
    drop(repo);
    tx.commit().unwrap();
    assert_eq!(
        workspace_controller::get_relationship(
            &ctx.db,
            &a.workspace_id,
            &WorkspaceRelationshipField::Tags
        )
        .unwrap(),
        vec![tag_b]
    );
    assert_eq!(
        workspace_controller::get_relationship(
            &ctx.db,
            &b.workspace_id,
            &WorkspaceRelationshipField::Tags
        )
        .unwrap(),
        vec![tag_a]
    );
}

#[test]
fn weak_one_to_many_is_exclusive_but_weak_many_to_many_is_shared() {
    let mut ctx = helpers::TestContext::new();
    let a = helpers::create_scaffold(&mut ctx);
    let b = helpers::create_scaffold(&mut ctx);
    let tag = helpers::create_tag(&mut ctx, a.workspace_id, "Shared", "red");
    for source in [a.workspace_id, b.workspace_id] {
        workspace_controller::set_relationship(
            &ctx.db,
            &ctx.hub,
            &mut ctx.undo,
            None,
            &WorkspaceRelationshipDto {
                id: source,
                field: WorkspaceRelationshipField::SelectedTags,
                right_ids: vec![tag],
            },
        )
        .unwrap();
    }
    workspace_controller::set_relationship(
        &ctx.db,
        &ctx.hub,
        &mut ctx.undo,
        None,
        &WorkspaceRelationshipDto {
            id: a.workspace_id,
            field: WorkspaceRelationshipField::FeaturedTags,
            right_ids: vec![tag],
        },
    )
    .unwrap();
    assert!(
        workspace_controller::set_relationship(
            &ctx.db,
            &ctx.hub,
            &mut ctx.undo,
            None,
            &WorkspaceRelationshipDto {
                id: b.workspace_id,
                field: WorkspaceRelationshipField::FeaturedTags,
                right_ids: vec![tag]
            }
        )
        .is_err()
    );
    assert!(
        workspace_controller::set_relationship(
            &ctx.db,
            &ctx.hub,
            &mut ctx.undo,
            None,
            &WorkspaceRelationshipDto {
                id: 999_999,
                field: WorkspaceRelationshipField::SelectedTags,
                right_ids: vec![tag]
            }
        )
        .is_err()
    );
    workspace_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &b.workspace_id).unwrap();
    assert!(tag_controller::get(&ctx.db, &tag).unwrap().is_some());
}
