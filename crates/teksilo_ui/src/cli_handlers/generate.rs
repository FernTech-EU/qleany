use crate::cli::{GenerateArgs, GenerateTarget, OutputContext};
use crate::cli_handlers::common::{TargetLanguage, get_target_language, run_checks, system_id};
use anyhow::{Result, bail};
use common::direct_access::system::SystemRelationshipField;
use common::entities::{FileNature, FileStatus};
use common::long_operation::OperationStatus;
use cpp_qt_file_generation::cpp_qt_file_generation_controller;
use direct_access::{FileDto, file_controller, system_controller};
use file_generation_shared_steps::file_generation_shared_steps_controller;
use frontend::AppContext;
use handling_manifest::handling_manifest_controller;
use rust_file_generation::rust_file_generation_controller;
use std::path::{Path, PathBuf};
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
    args: &GenerateArgs,
    output: &OutputContext,
) -> Result<()> {
    // Load manifest
    let load_dto = handling_manifest::LoadDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
    };
    handling_manifest_controller::load(&app_context.db_context, &app_context.event_hub, &load_dto)?;
    run_checks(app_context, output)?;

    let target_language = get_target_language(app_context)?;

    crate::cli_handlers::common::detect_and_warn_of_missing_formatters(
        &target_language,
        output,
        true,
    )?;

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

    // Step 2: Fill code in files (long operation — poll until complete)
    output.verbose("Generating code...");
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

    // Step 4: Retrieve all files
    let file_ids = system_controller::get_relationship(
        &app_context.db_context,
        &system_id(app_context)?,
        &SystemRelationshipField::Files,
    )?;

    let all_files: Vec<FileDto> = file_controller::get_multi(&app_context.db_context, &file_ids)?
        .into_iter()
        .flatten()
        .collect();

    // Step 5: Filter by target
    let filtered_by_target: Vec<&FileDto> = match args.target {
        GenerateTarget::All => all_files.iter().collect(),
        GenerateTarget::Feature => {
            let name = args.target_names.first().ok_or_else(|| {
                anyhow::anyhow!("Feature name required. Usage: generate feature <name>")
            })?;
            let name_lower = name.to_lowercase();
            let matched: Vec<&FileDto> = all_files
                .iter()
                .filter(|f| f.relative_path.to_lowercase().contains(&name_lower))
                .collect();
            if matched.is_empty() {
                bail!(
                    "No files found for feature '{}'. Check the name with: list features",
                    name
                );
            }
            matched
        }
        GenerateTarget::Entity => {
            let name = args.target_names.first().ok_or_else(|| {
                anyhow::anyhow!("Entity name required. Usage: generate entity <name>")
            })?;
            let name_lower = name.to_lowercase();
            let matched: Vec<&FileDto> = all_files
                .iter()
                .filter(|f| f.relative_path.to_lowercase().contains(&name_lower))
                .collect();
            if matched.is_empty() {
                bail!(
                    "No files found for entity '{}'. Check the name with: list entities",
                    name
                );
            }
            matched
        }
        GenerateTarget::Group => {
            let name = args.target_names.first().ok_or_else(|| {
                anyhow::anyhow!("Group name required. Usage: generate group <name>")
            })?;
            let matched: Vec<&FileDto> = all_files
                .iter()
                .filter(|f| f.group.eq_ignore_ascii_case(name))
                .collect();
            if matched.is_empty() {
                bail!(
                    "No files found for group '{}'. Check available groups with: list groups",
                    name
                );
            }
            matched
        }
        GenerateTarget::File => {
            if args.target_names.is_empty() {
                bail!("File path or ID required. Usage: generate file <path-or-id>...");
            }
            let targets = &args.target_names;
            all_files
                .iter()
                .filter(|f| {
                    targets.iter().any(|target| {
                        if let Ok(id) = target.parse::<u64>() {
                            f.id == id
                        } else {
                            let path = format!("{}{}", f.relative_path, f.name);
                            path.ends_with(target) || path == *target
                        }
                    })
                })
                .collect()
        }
    };

    // Step 6: Filter by status and nature
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

    let files: Vec<&FileDto> = filtered_by_target
        .into_iter()
        .filter(|f| status_filter.contains(&f.status))
        .filter(|f| nature_filter.contains(&f.nature))
        .collect();

    if files.is_empty() {
        output.info("No files to generate");
        return Ok(());
    }

    // Determine output path
    let output_path = determine_output_path(args)?;

    if args.dry_run {
        output.info("Dry run - no files will be written:");
        for file in &files {
            let prefix = match file.status {
                FileStatus::New => "[N]",
                FileStatus::Modified => "[M]",
                FileStatus::Unchanged => "[U]",
                FileStatus::Unknown => "[?]",
            };
            println!("  {} {}{}", prefix, file.relative_path, file.name);
        }
        output.info(&format!("{} files would be generated", files.len()));
        return Ok(());
    }

    output.info(&format!(
        "Writing {} files to {}...",
        files.len(),
        output_path.display()
    ));

    // Step 7: Write generated_code to disk
    let mut written = 0;
    let mut skipped = 0;

    for file in &files {
        let Some(ref code) = file.generated_code else {
            skipped += 1;
            output.verbose(&format!(
                "  [skip] {}{} (no generated code)",
                file.relative_path, file.name
            ));
            continue;
        };

        let mut file_path = output_path.clone();
        if !file.relative_path.is_empty() {
            file_path = file_path.join(&file.relative_path);
        }
        std::fs::create_dir_all(&file_path)?;
        file_path = file_path.join(&file.name);

        std::fs::write(&file_path, code)?;
        output.verbose(&format!("  {}{}", file.relative_path, file.name));
        written += 1;
    }

    output.success(&format!(
        "Generated {} files{}",
        written,
        if skipped > 0 {
            format!(" ({} skipped - no generated code)", skipped)
        } else {
            String::new()
        }
    ));

    Ok(())
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

fn determine_output_path(args: &GenerateArgs) -> Result<PathBuf> {
    if let Some(output) = &args.output {
        std::fs::create_dir_all(output)?;
        return Ok(output.clone());
    }

    let mut path = std::env::current_dir()?;
    if args.temp {
        path = path.join("temp");
    }
    std::fs::create_dir_all(&path)?;
    Ok(path)
}
