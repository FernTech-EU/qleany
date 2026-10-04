//! Isolated manifests for backend regression tests.
use crate::{app_ids::AppIds, manifest::ManifestViewModel};
use frontend::handling_manifest::dtos::{CreateDto, CreateLanguage, ManifestTemplate};
use frontend::{
    AppContext,
    commands::{handling_app_lifecycle_commands, handling_manifest_commands},
};
use std::{path::PathBuf, rc::Rc};

pub struct Fixture {
    pub dir: PathBuf,
    pub path: PathBuf,
    pub ctx: Rc<AppContext>,
    pub ids: AppIds,
    pub manifest: ManifestViewModel,
}

impl Fixture {
    pub fn new(language: CreateLanguage) -> Self {
        let dir = std::env::temp_dir().join(format!("qleany-regression-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("qleany.yaml");
        let ctx = Rc::new(AppContext::new());
        handling_app_lifecycle_commands::initialize_app(&ctx).unwrap();
        handling_manifest_commands::create(
            &ctx,
            &CreateDto {
                manifest_path: path.to_str().unwrap().to_string(),
                language,
                application_name: "Regression".to_string(),
                organization_name: "Example".to_string(),
                manifest_template: ManifestTemplate::Minimal,
                options: vec![],
            },
        )
        .unwrap();
        let ids = AppIds::new();
        ids.system_id.set(crate::bootstrap::system_id(&ctx));
        let manifest = ManifestViewModel::new(ctx.clone(), ids.clone());
        manifest.try_open_path(path.to_str().unwrap()).unwrap();
        Self {
            dir,
            path,
            ctx,
            ids,
            manifest,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.ctx.shutdown();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
