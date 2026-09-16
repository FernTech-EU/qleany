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
    output.verbose(&format!("Validating {}", manifest_path.display()));

    let load_dto = handling_manifest::LoadDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
    };

    // Load validates the manifest structure
    handling_manifest_controller::load(&app_context.db_context, &app_context.event_hub, &load_dto)?;

    // Run semantic checks
    run_checks(app_context, output)?;

    output.success("Manifest is valid");

    Ok(())
}

pub fn list_rules(output: &OutputContext) {
    let rules = handling_manifest_controller::get_check_rules();

    let critical: Vec<_> = rules.iter().filter(|r| r.severity == "critical").collect();
    let warnings: Vec<_> = rules.iter().filter(|r| r.severity == "warning").collect();

    output.info(&format!(
        "Critical rules ({}) — any violation prevents code generation:\n",
        critical.len()
    ));
    for rule in &critical {
        println!("  [{}] {}", rule.id, rule.description);
    }

    println!();

    output.info(&format!(
        "Warning rules ({}) — non-blocking issues worth reviewing:\n",
        warnings.len()
    ));
    for rule in &warnings {
        println!("  [{}] {}", rule.id, rule.description);
    }
}
