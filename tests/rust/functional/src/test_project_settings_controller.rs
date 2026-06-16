// Functional tests for ProjectSettings controller (one_to_one strong required child of Project)

use crate::helpers::{self, TestContext};
use direct_access::*;

fn setup() -> (TestContext, helpers::Scaffold) {
    let mut ctx = TestContext::new();
    let scaffold = helpers::create_scaffold(&mut ctx);
    (ctx, scaffold)
}

// ---------------------------------------------------------------------------
// get
// ---------------------------------------------------------------------------

#[test]
fn test_get_by_id() {
    let (ctx, s) = setup();
    let fetched = project_settings_controller::get(&ctx.db, &s.project_settings_id).unwrap();
    assert!(fetched.is_some());
}

#[test]
fn test_get_non_existent() {
    let (ctx, _) = setup();
    assert!(
        project_settings_controller::get(&ctx.db, &999999)
            .unwrap()
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// settings linked via relationship
// ---------------------------------------------------------------------------

#[test]
fn test_settings_linked_to_project() {
    let (ctx, s) = setup();
    let rel = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Settings,
    )
    .unwrap();
    assert_eq!(rel.len(), 1);
    assert_eq!(rel[0], s.project_settings_id);
}

#[test]
fn test_settings_have_default_values() {
    let (ctx, s) = setup();
    let settings = project_settings_controller::get(&ctx.db, &s.project_settings_id)
        .unwrap()
        .unwrap();
    assert!(!settings.notifications_enabled);
    assert_eq!(settings.default_priority, 0);
    assert!(settings.color_theme.is_empty());
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

#[test]
fn test_update_fields() {
    let (mut ctx, s) = setup();
    let dto = project_settings_controller::get(&ctx.db, &s.project_settings_id)
        .unwrap()
        .unwrap();

    let mut update_dto: UpdateProjectSettingsDto = dto.into();
    update_dto.notifications_enabled = true;
    update_dto.default_priority = 5;
    update_dto.color_theme = "dark".into();

    let updated =
        project_settings_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto)
            .unwrap();
    assert!(updated.notifications_enabled);
    assert_eq!(updated.default_priority, 5);
    assert_eq!(updated.color_theme, "dark");

    let fetched = project_settings_controller::get(&ctx.db, &s.project_settings_id)
        .unwrap()
        .unwrap();
    assert!(fetched.notifications_enabled);
    assert_eq!(fetched.default_priority, 5);
    assert_eq!(fetched.color_theme, "dark");
}

// ---------------------------------------------------------------------------
// cascade delete: removing project removes settings
// ---------------------------------------------------------------------------

#[test]
fn test_remove_project_cascades_to_settings() {
    let (mut ctx, s) = setup();
    assert!(
        project_settings_controller::get(&ctx.db, &s.project_settings_id)
            .unwrap()
            .is_some()
    );

    project_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &s.project_id).unwrap();

    assert!(
        project_settings_controller::get(&ctx.db, &s.project_settings_id)
            .unwrap()
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// undo cascade delete restores settings
// ---------------------------------------------------------------------------

#[test]
fn test_undo_remove_project_restores_settings() {
    let (mut ctx, s) = setup();

    // Update settings first
    let dto = project_settings_controller::get(&ctx.db, &s.project_settings_id)
        .unwrap()
        .unwrap();
    let mut update_dto: UpdateProjectSettingsDto = dto.into();
    update_dto.color_theme = "solarized".into();
    project_settings_controller::update(&ctx.db, &ctx.hub, &mut ctx.undo, None, &update_dto)
        .unwrap();

    // Remove project
    project_controller::remove(&ctx.db, &ctx.hub, &mut ctx.undo, None, &s.project_id).unwrap();
    assert!(
        project_settings_controller::get(&ctx.db, &s.project_settings_id)
            .unwrap()
            .is_none()
    );

    // Undo
    ctx.undo.undo(None).unwrap();

    // Settings restored with updated value
    let restored = project_settings_controller::get(&ctx.db, &s.project_settings_id)
        .unwrap()
        .unwrap();
    assert_eq!(restored.color_theme, "solarized");
}

// ---------------------------------------------------------------------------
// each project gets its own settings
// ---------------------------------------------------------------------------

#[test]
fn test_each_project_gets_own_settings() {
    let (mut ctx, s) = setup();
    let proj2_id = helpers::create_project(&mut ctx, s.workspace_id, "SecondProject");

    let rel1 = project_controller::get_relationship(
        &ctx.db,
        &s.project_id,
        &ProjectRelationshipField::Settings,
    )
    .unwrap();
    let rel2 = project_controller::get_relationship(
        &ctx.db,
        &proj2_id,
        &ProjectRelationshipField::Settings,
    )
    .unwrap();

    assert_eq!(rel1.len(), 1);
    assert_eq!(rel2.len(), 1);
    assert_ne!(rel1[0], rel2[0]);
}
