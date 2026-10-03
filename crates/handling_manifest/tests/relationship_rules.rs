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
