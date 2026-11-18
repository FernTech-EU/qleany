//! Shared by the rule test files: load a variant of the full Rust example.
//! A directory module, so cargo does not build it as a test target of its own.
//! Each test root declares `mod init;` and `mod rules_fixture;`.

use crate::init;
use common::database::db_context::DbContext;
use common::event::EventHub;
use handling_manifest::{LoadDto, handling_manifest_controller as controller};
use serde_yml::Value;
use std::{path::PathBuf, sync::Arc};

pub struct Fixture {
    pub path: PathBuf,
    pub db: DbContext,
    pub hub: Arc<EventHub>,
}
impl Fixture {
    pub fn new(edit: impl FnOnce(&mut Value)) -> Self {
        let mut manifest: Value =
            serde_yml::from_str(include_str!("../../../../examples/rust/full/qleany.yaml"))
                .unwrap();
        edit(&mut manifest);
        // The tests of one file run in parallel threads of one process, so the path
        // takes a counter: a timestamp alone can repeat on a coarse clock, and two
        // fixtures sharing a file load each other's manifest.
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "qleany-rules-{}-{}.yaml",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
pub fn fields<'a>(manifest: &'a mut Value, name: &str) -> &'a mut Vec<Value> {
    manifest["entities"]
        .as_sequence_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e["name"].as_str() == Some(name))
        .unwrap()["fields"]
        .as_sequence_mut()
        .unwrap()
}
