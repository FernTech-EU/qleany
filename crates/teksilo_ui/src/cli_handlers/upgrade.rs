use crate::cli::OutputContext;
use crate::cli_handlers::common::run_checks;
use anyhow::Result;
use frontend::AppContext;
use handling_manifest::handling_manifest_controller;
use std::path::Path;
use std::rc::Rc;

pub fn execute(
    app_context: &Rc<AppContext>,
    manifest_path: &Path,
    output: &OutputContext,
) -> Result<()> {
    output.verbose(&format!("Upgrading {}", manifest_path.display()));

    // Load (triggers automatic migration to current schema version)
    let load_dto = handling_manifest::LoadDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
    };

    handling_manifest_controller::load(&app_context.db_context, &app_context.event_hub, &load_dto)?;
    run_checks(app_context, output)?;

    // Save back with current schema version
    let save_dto = handling_manifest::SaveDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
    };

    handling_manifest_controller::save(&app_context.db_context, &app_context.event_hub, &save_dto)?;

    output.success(&format!("Upgraded {}", manifest_path.display()));

    Ok(())
}
