pub mod file_tree;

use crate::cli::{ListArgs, ListTarget, OutputContext, OutputFormat};
use crate::cli_handlers::common::{TargetLanguage, get_target_language, run_checks, system_id};
use anyhow::Result;
use common::direct_access::system::SystemRelationshipField;
use common::entities::{FileNature, FileStatus};
use common::long_operation::OperationStatus;
use cpp_qt_file_generation::cpp_qt_file_generation_controller;
use direct_access::{EntityDto, FileDto, UseCaseDto, file_controller, system_controller};
use file_generation_shared_steps::file_generation_shared_steps_controller;
use frontend::AppContext;
use handling_manifest::handling_manifest_controller;
use rust_file_generation::rust_file_generation_controller;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

/// Resolve status flags into the set of FileStatus values to include.
/// Default (no flags): Modified + New.
fn resolve_status_filter(
    all: bool,
    all_status: bool,
    modified: bool,
    new: bool,
    unchanged: bool,
) -> Vec<FileStatus> {
    if all || all_status {
        return vec![
            FileStatus::Modified,
            FileStatus::New,
            FileStatus::Unchanged,
            FileStatus::Unknown,
        ];
    }
    if !modified && !new && !unchanged {
        // Default: Modified + New
        return vec![FileStatus::Modified, FileStatus::New];
    }
    let mut statuses = Vec::new();
    if modified {
        statuses.push(FileStatus::Modified);
    }
    if new {
        statuses.push(FileStatus::New);
    }
    if unchanged {
        statuses.push(FileStatus::Unchanged);
    }
    statuses
}

/// Resolve nature flags into the set of FileNature values to include.
/// Default (no flags): all natures.
fn resolve_nature_filter(
    all: bool,
    all_natures: bool,
    infra: bool,
    aggregates: bool,
    scaffolds: bool,
) -> Vec<FileNature> {
    if all || all_natures || (!infra && !aggregates && !scaffolds) {
        return vec![
            FileNature::Infrastructure,
            FileNature::Aggregate,
            FileNature::Scaffold,
        ];
    }
    let mut natures = Vec::new();
    if infra {
        natures.push(FileNature::Infrastructure);
    }
    if aggregates {
        natures.push(FileNature::Aggregate);
    }
    if scaffolds {
        natures.push(FileNature::Scaffold);
    }
    natures
}

pub fn execute(
    app_context: &Rc<AppContext>,
    manifest_path: &Path,
    args: &ListArgs,
    output: &OutputContext,
) -> Result<()> {
    // Load manifest first
    let load_dto = handling_manifest::LoadDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
    };
    handling_manifest_controller::load(&app_context.db_context, &app_context.event_hub, &load_dto)?;
    run_checks(app_context, output)?;

    match args.target {
        ListTarget::Files => list_files(app_context, args, output),
        ListTarget::Entities => list_entities(app_context, args, output),
        ListTarget::Features => list_features(app_context, args, output),
        ListTarget::Groups => list_groups(app_context, args, output),
    }
}

fn list_files(app_context: &Rc<AppContext>, args: &ListArgs, output: &OutputContext) -> Result<()> {
    let target_language = get_target_language(app_context)?;

    // Step 1: Fill file list in DB
    output.verbose("Populating file list...");
    match target_language {
        TargetLanguage::Rust => {
            let dto = rust_file_generation::FillRustFilesDto {
                only_list_already_existing: false,
            };
            rust_file_generation_controller::fill_rust_files(
                &app_context.db_context,
                &app_context.event_hub,
                &dto,
            )?;
        }
        TargetLanguage::CppQt => {
            let dto = cpp_qt_file_generation::FillCppQtFilesDto {
                only_list_already_existing: false,
            };
            cpp_qt_file_generation_controller::fill_cpp_qt_files(
                &app_context.db_context,
                &app_context.event_hub,
                &dto,
            )?;
        }
    }

    crate::cli_handlers::common::detect_and_warn_of_missing_formatters(
        &target_language,
        output,
        true,
    )?;

    // Step 2: Fill code in files (long operation — poll until complete)
    output.verbose("Generating code for status comparison...");
    let operation_id = {
        let mut long_op_manager = app_context.long_operation_manager.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire lock on long operation manager: {e}")
        })?;
        match target_language {
            TargetLanguage::Rust => rust_file_generation_controller::fill_code_in_rust_files(
                &app_context.db_context,
                &app_context.event_hub,
                &mut long_op_manager,
            )?,
            TargetLanguage::CppQt => cpp_qt_file_generation_controller::fill_code_in_cpp_qt_files(
                &app_context.db_context,
                &app_context.event_hub,
                &mut long_op_manager,
            )?,
        }
    };

    poll_long_operation(app_context, &operation_id, output)?;

    // Step 3: Fill status by comparing generated code vs disk
    output.verbose("Comparing with files on disk...");
    file_generation_shared_steps_controller::fill_status_in_files(
        &app_context.db_context,
        &app_context.event_hub,
    )?;

    // Step 4: Retrieve all files and filter by status
    let file_ids = system_controller::get_relationship(
        &app_context.db_context,
        &system_id(app_context)?,
        &SystemRelationshipField::Files,
    )?;

    let all_files: Vec<FileDto> = file_controller::get_multi(&app_context.db_context, &file_ids)?
        .into_iter()
        .flatten()
        .collect();

    let status_filter = resolve_status_filter(
        args.all,
        args.all_status,
        args.modified,
        args.new,
        args.unchanged,
    );
    let nature_filter = resolve_nature_filter(
        args.all,
        args.all_natures,
        args.infra,
        args.aggregates,
        args.scaffolds,
    );

    let files: Vec<&FileDto> = all_files
        .iter()
        .filter(|f| status_filter.contains(&f.status))
        .filter(|f| nature_filter.contains(&f.nature))
        .collect();

    // Step 5: Display results
    display_file_list(&files, args, output)
}

/// Polls a long operation until it completes, reporting progress if verbose.
fn poll_long_operation(
    app_context: &Rc<AppContext>,
    operation_id: &str,
    output: &OutputContext,
) -> Result<()> {
    let mut last_percentage: f32 = 0.0;

    loop {
        std::thread::sleep(Duration::from_millis(100));

        let long_op_manager = app_context.long_operation_manager.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire lock on long operation manager: {e}")
        })?;

        let status = match long_op_manager.get_operation_status(operation_id) {
            Some(s) => s,
            None => {
                output.warn("Operation not found");
                break;
            }
        };

        if output.verbose
            && let Some(progress) = long_op_manager.get_operation_progress(operation_id)
            && (progress.percentage - last_percentage).abs() >= 10.0
        {
            output.verbose(&format!(
                "[{:.0}%] {}",
                progress.percentage,
                progress.message.as_deref().unwrap_or("")
            ));
            last_percentage = progress.percentage;
        }

        match status {
            OperationStatus::Running => {}
            OperationStatus::Completed => break,
            OperationStatus::Cancelled => anyhow::bail!("Operation was cancelled"),
            OperationStatus::Failed(err) => anyhow::bail!("Operation failed: {}", err),
        }
    }

    Ok(())
}

/// Displays the filtered file list according to the chosen format.
fn display_file_list(files: &[&FileDto], args: &ListArgs, output: &OutputContext) -> Result<()> {
    let count_new = files.iter().filter(|f| f.status == FileStatus::New).count();
    let count_modified = files
        .iter()
        .filter(|f| f.status == FileStatus::Modified)
        .count();
    let count_unchanged = files
        .iter()
        .filter(|f| f.status == FileStatus::Unchanged)
        .count();

    match args.format {
        OutputFormat::Plain => {
            if args.text {
                // Plain text with status prefix
                for file in files {
                    let prefix = match file.status {
                        FileStatus::New => "[N]",
                        FileStatus::Modified => "[M]",
                        FileStatus::Unchanged => "[U]",
                        FileStatus::Unknown => "[?]",
                    };
                    println!("{} {}{}", prefix, file.relative_path, file.name);
                }
            } else {
                // Colored output via termimad
                let mut md = String::new();
                for file in files {
                    let path = format!("{}{}", file.relative_path, file.name);
                    match file.status {
                        FileStatus::New => md.push_str(&format!("**~~[N]~~** **{}**\n", path)),
                        FileStatus::Modified => md.push_str(&format!("*[M]* *{}*\n", path)),
                        FileStatus::Unchanged => md.push_str(&format!("[U] {}\n", path)),
                        FileStatus::Unknown => md.push_str(&format!("[?] {}\n", path)),
                    }
                }
                let mut skin = termimad::MadSkin::default();
                skin.bold.set_fg(termimad::crossterm::style::Color::Green);
                skin.italic
                    .set_fg(termimad::crossterm::style::Color::Yellow);
                skin.strikeout
                    .set_fg(termimad::crossterm::style::Color::Green);
                skin.print_text(&md);
            }
            output.info(&format!(
                "\n{} new, {} modified, {} unchanged — {} total",
                count_new,
                count_modified,
                count_unchanged,
                files.len()
            ));
        }
        OutputFormat::Json => {
            let json: Vec<_> = files
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "path": format!("{}{}", f.relative_path, f.name),
                        "status": format!("{:?}", f.status),
                        "group": f.group,
                    })
                })
                .collect();
            let wrapper = serde_json::json!({
                "files": json,
                "count": files.len(),
                "new": count_new,
                "modified": count_modified,
                "unchanged": count_unchanged,
            });
            println!("{}", serde_json::to_string_pretty(&wrapper)?);
        }
        OutputFormat::Tree => {
            let paths: Vec<String> = files
                .iter()
                .map(|f| format!("{}{}", f.relative_path, f.name))
                .collect();
            file_tree::print_file_tree(&paths);
            output.info(&format!(
                "\n{} new, {} modified, {} unchanged — {} total",
                count_new,
                count_modified,
                count_unchanged,
                files.len()
            ));
        }
    }

    Ok(())
}

fn list_entities(
    app_context: &Rc<AppContext>,
    args: &ListArgs,
    output: &OutputContext,
) -> Result<()> {
    use common::direct_access::workspace::WorkspaceRelationshipField;
    use direct_access::{entity_controller, workspace_controller};

    // Get workspace and entities
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

    let entities = entity_controller::get_multi(&app_context.db_context, entity_ids.as_slice())?
        .into_iter()
        .flatten()
        .collect::<Vec<EntityDto>>();

    match args.format {
        OutputFormat::Plain => {
            for entity in &entities {
                let heritage = if entity.only_for_heritage {
                    " (heritage only)"
                } else {
                    ""
                };
                println!("{}{}", entity.name, heritage);
            }
            output.info(&format!("\n{} entities", entities.len()));
        }
        OutputFormat::Json => {
            let json: Vec<_> = entities
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "name": e.name,
                        "only_for_heritage": e.only_for_heritage,
                        "undoable": e.undoable,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Tree => {
            file_tree::print_file_tree(
                &entities
                    .iter()
                    .map(|e| e.name.as_str())
                    .collect::<Vec<&str>>(),
            );
        }
    }

    Ok(())
}

fn list_features(
    app_context: &Rc<AppContext>,
    args: &ListArgs,
    output: &OutputContext,
) -> Result<()> {
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

    let mut features_data = Vec::new();
    for feature_id in feature_ids {
        if let Some(feature) = feature_controller::get(&app_context.db_context, &feature_id)? {
            let use_case_ids = feature_controller::get_relationship(
                &app_context.db_context,
                &feature_id,
                &FeatureRelationshipField::UseCases,
            )?;

            let mut use_cases = Vec::new();
            for uc_id in use_case_ids {
                if let Some(uc) = use_case_controller::get(&app_context.db_context, &uc_id)? {
                    use_cases.push(uc);
                }
            }
            features_data.push((feature, use_cases));
        }
    }

    match args.format {
        OutputFormat::Plain => {
            for (feature, use_cases) in &features_data {
                println!("{}:", feature.name);
                for uc in use_cases {
                    let flags = format_use_case_flags(uc);
                    println!("  - {}{}", uc.name, flags);
                }
            }
            output.info(&format!(
                "\n{} features, {} use cases",
                features_data.len(),
                features_data
                    .iter()
                    .map(|(_, ucs)| ucs.len())
                    .sum::<usize>()
            ));
        }
        OutputFormat::Json => {
            let json: Vec<_> = features_data
                .iter()
                .map(|(f, ucs)| {
                    serde_json::json!({
                        "name": f.name,
                        "use_cases": ucs.iter().map(|uc| serde_json::json!({
                            "name": uc.name,
                            "undoable": uc.undoable,
                            "read_only": uc.read_only,
                        })).collect::<Vec<_>>()
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Tree => {}
    }

    Ok(())
}

fn list_groups(
    app_context: &Rc<AppContext>,
    args: &ListArgs,
    output: &OutputContext,
) -> Result<()> {
    use direct_access::file_controller;

    let target_language = get_target_language(app_context)?;

    // Populate file list in DB so groups are available
    output.verbose("Populating file list...");
    match target_language {
        TargetLanguage::Rust => {
            let dto = rust_file_generation::FillRustFilesDto {
                only_list_already_existing: false,
            };
            rust_file_generation_controller::fill_rust_files(
                &app_context.db_context,
                &app_context.event_hub,
                &dto,
            )?;
        }
        TargetLanguage::CppQt => {
            let dto = cpp_qt_file_generation::FillCppQtFilesDto {
                only_list_already_existing: false,
            };
            cpp_qt_file_generation_controller::fill_cpp_qt_files(
                &app_context.db_context,
                &app_context.event_hub,
                &dto,
            )?;
        }
    }

    let file_ids = system_controller::get_relationship(
        &app_context.db_context,
        &system_id(app_context)?,
        &SystemRelationshipField::Files,
    )?;
    let mut groups: BTreeMap<String, usize> = BTreeMap::new();

    for id in file_ids {
        if let Some(file) = file_controller::get(&app_context.db_context, &id)? {
            *groups.entry(file.group).or_insert(0) += 1;
        }
    }

    if groups.is_empty() {
        output.info("No file groups found");
        return Ok(());
    }

    match args.format {
        OutputFormat::Plain | OutputFormat::Tree => {
            for (group, count) in &groups {
                println!("{} ({} files)", group, count);
            }
            output.info(&format!("\n{} groups", groups.len()));
        }
        OutputFormat::Json => {
            let json: Vec<_> = groups
                .iter()
                .map(|(g, c)| serde_json::json!({"name": g, "file_count": c}))
                .collect();
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }

    Ok(())
}

fn format_use_case_flags(uc: &UseCaseDto) -> String {
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
    if flags.is_empty() {
        String::new()
    } else {
        format!(" [{}]", flags.join(", "))
    }
}
