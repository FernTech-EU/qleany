use crate::cli::{OutputContext, OutputFormat, ShowArgs, ShowTarget};
use crate::cli_handlers::common::run_checks;
use anyhow::Result;
use frontend::AppContext;
use handling_manifest::handling_manifest_controller;
use std::path::Path;
use std::rc::Rc;

pub fn execute(
    app_context: &Rc<AppContext>,
    manifest_path: &Path,
    args: &ShowArgs,
    output: &OutputContext,
) -> Result<()> {
    // Load manifest
    let load_dto = handling_manifest::LoadDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
    };
    handling_manifest_controller::load(&app_context.db_context, &app_context.event_hub, &load_dto)?;
    run_checks(app_context, output)?;

    output.verbose(&format!("Loaded manifest from {}", manifest_path.display()));

    let target = args.target.as_ref().unwrap_or(&ShowTarget::Manifest);

    match target {
        ShowTarget::Manifest => show_manifest(manifest_path, args),
        ShowTarget::Config => show_config(app_context, args),
        ShowTarget::Entity { name } => show_entity(app_context, name, args),
        ShowTarget::Feature { name } => show_feature(app_context, name, args),
    }
}

fn show_manifest(manifest_path: &Path, args: &ShowArgs) -> Result<()> {
    let content = std::fs::read_to_string(manifest_path)?;

    match args.format {
        OutputFormat::Plain | OutputFormat::Tree => {
            println!("{}", content);
        }
        OutputFormat::Json => {
            // Parse YAML and convert to JSON
            let yaml: serde_yml::Value = serde_yml::from_str(&content)?;
            println!("{}", serde_json::to_string_pretty(&yaml)?);
        }
    }

    Ok(())
}

fn show_config(app_context: &Rc<AppContext>, args: &ShowArgs) -> Result<()> {
    use common::direct_access::workspace::WorkspaceRelationshipField;
    use direct_access::{global_controller, workspace_controller};

    let workspaces = workspace_controller::get_all(&app_context.db_context)?;
    let workspace = workspaces
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No workspace loaded"))?;

    let global_ids = workspace_controller::get_relationship(
        &app_context.db_context,
        &workspace.id,
        &WorkspaceRelationshipField::Global,
    )?;
    let global_id = global_ids
        .first()
        .ok_or_else(|| anyhow::anyhow!("No global configuration found"))?;

    let global = global_controller::get(&app_context.db_context, global_id)?
        .ok_or_else(|| anyhow::anyhow!("Global configuration not found"))?;

    match args.format {
        OutputFormat::Plain | OutputFormat::Tree => {
            println!("Language:     {}", global.language);
            println!("Application:  {}", global.application_name);
            println!(
                "Organisation: {} ({})",
                global.organisation_name, global.organisation_domain
            );
            println!("Prefix path:  {}", global.prefix_path);
        }
        OutputFormat::Json => {
            let json = serde_json::json!({
                "language": global.language,
                "application_name": global.application_name,
                "organisation_name": global.organisation_name,
                "organisation_domain": global.organisation_domain,
                "prefix_path": global.prefix_path,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }

    Ok(())
}

fn show_entity(app_context: &Rc<AppContext>, name: &str, args: &ShowArgs) -> Result<()> {
    use common::direct_access::entity::EntityRelationshipField;
    use common::direct_access::workspace::WorkspaceRelationshipField;
    use direct_access::{entity_controller, field_controller, workspace_controller};

    let workspaces = workspace_controller::get_all(&app_context.db_context)?;
    let workspace = workspaces
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No workspace loaded"))?;

    let entity_ids = workspace_controller::get_relationship(
        &app_context.db_context,
        &workspace.id,
        &WorkspaceRelationshipField::Entities,
    )?;

    // Find the entity by name
    let mut found_entity = None;
    for id in &entity_ids {
        if let Some(entity) = entity_controller::get(&app_context.db_context, id)?
            && entity.name.eq_ignore_ascii_case(name)
        {
            found_entity = Some((*id, entity));
            break;
        }
    }

    let (entity_id, entity) =
        found_entity.ok_or_else(|| anyhow::anyhow!("Entity not found: {}", name))?;

    // Get fields
    let field_ids = entity_controller::get_relationship(
        &app_context.db_context,
        &entity_id,
        &EntityRelationshipField::Fields,
    )?;

    let mut fields = Vec::new();
    for id in field_ids {
        if let Some(field) = field_controller::get(&app_context.db_context, &id)? {
            fields.push(field);
        }
    }

    match args.format {
        OutputFormat::Plain | OutputFormat::Tree => {
            println!("Entity: {}", entity.name);
            if entity.only_for_heritage {
                println!("  (heritage only)");
            }
            println!("  undoable: {}", entity.undoable);
            println!("\nFields ({}):", fields.len());
            for field in &fields {
                println!("  - {}: {:?}", field.name, field.field_type);
            }
        }
        OutputFormat::Json => {
            let json = serde_json::json!({
                "name": entity.name,
                "only_for_heritage": entity.only_for_heritage,
                "undoable": entity.undoable,
                "fields": fields.iter().map(|f| serde_json::json!({
                    "name": f.name,
                    "field_type": format!("{:?}", f.field_type),
                    "optional": f.optional,
                })).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }

    Ok(())
}

fn show_feature(app_context: &Rc<AppContext>, name: &str, args: &ShowArgs) -> Result<()> {
    use common::direct_access::feature::FeatureRelationshipField;
    use common::direct_access::workspace::WorkspaceRelationshipField;
    use direct_access::{feature_controller, use_case_controller, workspace_controller};

    let workspaces = workspace_controller::get_all(&app_context.db_context)?;
    let workspace = workspaces
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No workspace loaded"))?;

    let feature_ids = workspace_controller::get_relationship(
        &app_context.db_context,
        &workspace.id,
        &WorkspaceRelationshipField::Features,
    )?;

    // Find the feature by name
    let mut found_feature = None;
    for id in &feature_ids {
        if let Some(feature) = feature_controller::get(&app_context.db_context, id)?
            && feature.name.eq_ignore_ascii_case(name)
        {
            found_feature = Some((*id, feature));
            break;
        }
    }

    let (feature_id, feature) =
        found_feature.ok_or_else(|| anyhow::anyhow!("Feature not found: {}", name))?;

    // Get use cases
    let use_case_ids = feature_controller::get_relationship(
        &app_context.db_context,
        &feature_id,
        &FeatureRelationshipField::UseCases,
    )?;

    let mut use_cases = Vec::new();
    for id in use_case_ids {
        if let Some(uc) = use_case_controller::get(&app_context.db_context, &id)? {
            use_cases.push(uc);
        }
    }

    match args.format {
        OutputFormat::Plain | OutputFormat::Tree => {
            println!("Feature: {}", feature.name);
            println!("\nUse Cases ({}):", use_cases.len());
            for uc in &use_cases {
                let mut flags = Vec::new();
                if uc.undoable {
                    flags.push("undoable");
                }
                if uc.read_only {
                    flags.push("read-only");
                }
                if uc.long_operation {
                    flags.push("async");
                }

                let flags_str = if flags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", flags.join(", "))
                };
                println!("  - {}{}", uc.name, flags_str);
            }
        }
        OutputFormat::Json => {
            let json = serde_json::json!({
                "name": feature.name,
                "use_cases": use_cases.iter().map(|uc| serde_json::json!({
                    "name": uc.name,
                    "undoable": uc.undoable,
                    "read_only": uc.read_only,
                    "long_operation": uc.long_operation,
                })).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }

    Ok(())
}
