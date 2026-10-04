use common::database::{db_context::DbContext, transactions::Transaction};
use common::direct_access::repository_factory;
use common::entities::{FieldRelationshipType, FieldType};
use common::event::{EventBuffer, EventHub};
use handling_manifest::{LoadDto, SaveDto, handling_manifest_controller as controller};
use serde_yml::Value;
use std::{path::PathBuf, sync::Arc};
mod init;

struct Fixture {
    path: PathBuf,
    db: DbContext,
    hub: Arc<EventHub>,
}
impl Fixture {
    fn new(edit: impl FnOnce(&mut Value)) -> Self {
        let mut manifest: Value =
            serde_yml::from_str(include_str!("../../../examples/rust/full/qleany.yaml")).unwrap();
        edit(&mut manifest);
        let path = std::env::temp_dir().join(format!(
            "qleany-rules-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, serde_yml::to_string(&manifest).unwrap()).unwrap();
        let db = DbContext::new().unwrap();
        let hub = Arc::new(EventHub::new());
        init::initialize_app(&db, &hub).unwrap();
        controller::load(
            &db,
            &hub,
            &LoadDto {
                manifest_path: path.to_string_lossy().into_owned(),
            },
        )
        .unwrap();
        Self { path, db, hub }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
fn fields<'a>(manifest: &'a mut Value, name: &str) -> &'a mut Vec<Value> {
    manifest["entities"]
        .as_sequence_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e["name"].as_str() == Some(name))
        .unwrap()["fields"]
        .as_sequence_mut()
        .unwrap()
}

#[test]
fn many_relationships_cannot_be_strong_and_rules_are_listed() {
    for kind in ["many_to_many", "many_to_one"] {
        let f = Fixture::new(|m| {
            let field = fields(m, "Workspace")
                .iter_mut()
                .find(|f| f["name"].as_str() == Some("tags"))
                .unwrap();
            field["relationship"] = Value::String(kind.into());
        });
        let result = controller::check(&f.db, &f.hub).unwrap();
        assert!(
            result
                .critical_errors
                .iter()
                .any(|e| e.contains("must be weak")),
            "{:?}",
            result.critical_errors
        );
    }
    let rules = controller::get_check_rules();
    for (id, severity) in [("C49", "critical"), ("W06", "warning")] {
        assert!(
            rules
                .iter()
                .any(|rule| rule.id == id && rule.severity == severity)
        );
    }
}

#[test]
fn multiple_strong_fields_warn_about_ambiguous_creation() {
    let f = Fixture::new(|m| {
        let fs = fields(m, "Workspace");
        let mut duplicate = fs
            .iter()
            .find(|f| f["name"].as_str() == Some("tags"))
            .unwrap()
            .clone();
        duplicate["name"] = Value::String("other_tags".into());
        fs.push(duplicate);
    });
    let result = controller::check(&f.db, &f.hub).unwrap();
    assert!(
        result
            .warnings
            .iter()
            .any(|e| e.contains("multiple strong owning fields")),
        "{:?}",
        result.critical_errors
    );
}

#[test]
fn weak_lists_and_a_distinct_strong_owner_are_valid() {
    let f = Fixture::new(|_| {});
    let result = controller::check(&f.db, &f.hub).unwrap();
    assert!(
        result.critical_errors.is_empty(),
        "{:?}",
        result.critical_errors
    );
}

#[test]
fn missing_target_on_save_returns_error_and_preserves_file() {
    let f = Fixture::new(|_| {});
    let before = std::fs::read(&f.path).unwrap();
    let mut tx = Transaction::begin_write_transaction(&f.db).unwrap();
    let mut repo = repository_factory::write::create_field_repository(&tx).unwrap();
    let mut field = repo
        .get_all()
        .unwrap()
        .into_iter()
        .find(|f| {
            f.field_type == FieldType::Entity && f.relationship == FieldRelationshipType::ManyToMany
        })
        .unwrap();
    field.entity = None;
    repo.update_with_relationships(&mut EventBuffer::new(), &field)
        .unwrap();
    drop(repo);
    tx.commit().unwrap();
    let result = controller::save(
        &f.db,
        &f.hub,
        &SaveDto {
            manifest_path: f.path.to_string_lossy().into_owned(),
        },
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("select a referenced entity")
    );
    assert_eq!(std::fs::read(&f.path).unwrap(), before);
}

#[test]
fn ownership_validation_tracks_field_edits_without_reloading() {
    let f = Fixture::new(|m| {
        let fs = fields(m, "Workspace");
        let mut duplicate = fs
            .iter()
            .find(|f| f["name"].as_str() == Some("tags"))
            .unwrap()
            .clone();
        duplicate["name"] = Value::String("other_tags".into());
        fs.push(duplicate);
    });
    let has_ambiguity = || {
        controller::check(&f.db, &f.hub)
            .unwrap()
            .warnings
            .iter()
            .any(|message| message.contains("multiple strong owning fields"))
    };
    assert!(has_ambiguity());
    for (strong, expected_warning) in [(false, false), (true, true)] {
        let mut tx = Transaction::begin_write_transaction(&f.db).unwrap();
        let mut repo = repository_factory::write::create_field_repository(&tx).unwrap();
        let mut field = repo
            .get_all()
            .unwrap()
            .into_iter()
            .find(|f| f.name == "other_tags")
            .unwrap();
        field.strong = strong;
        repo.update_with_relationships(&mut EventBuffer::new(), &field)
            .unwrap();
        drop(repo);
        tx.commit().unwrap();
        assert_eq!(has_ambiguity(), expected_warning);
    }
    // A new graph edge must also participate even though no Relationship row was
    // created for it by load. Point the owner's field at itself to create a cycle.
    let mut tx = Transaction::begin_write_transaction(&f.db).unwrap();
    let entities = repository_factory::write::create_entity_repository(&tx)
        .unwrap()
        .get_all()
        .unwrap();
    let workspace_id = entities.iter().find(|e| e.name == "Workspace").unwrap().id;
    let mut repo = repository_factory::write::create_field_repository(&tx).unwrap();
    let mut field = repo
        .get_all()
        .unwrap()
        .into_iter()
        .find(|f| f.name == "other_tags")
        .unwrap();
    field.entity = Some(workspace_id);
    repo.update_with_relationships(&mut EventBuffer::new(), &field)
        .unwrap();
    drop(repo);
    tx.commit().unwrap();
    assert!(
        controller::check(&f.db, &f.hub)
            .unwrap()
            .critical_errors
            .iter()
            .any(|message| message.contains("Cyclic strong dependency"))
    );
}
