use crate::cli::{DiffArgs, OutputContext};
use crate::cli_handlers::common::{TargetLanguage, get_target_language, run_checks, system_id};
use anyhow::Result;
use common::direct_access::system::SystemRelationshipField;
use common::long_operation::OperationStatus;
use cpp_qt_file_generation::cpp_qt_file_generation_controller;
use direct_access::{FileDto, file_controller, system_controller};
use file_generation_shared_steps::file_generation_shared_steps_controller;
use frontend::AppContext;
use handling_manifest::handling_manifest_controller;
use rust_file_generation::rust_file_generation_controller;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

pub fn execute(
    app_context: &Rc<AppContext>,
    manifest_path: &Path,
    args: &DiffArgs,
    output: &OutputContext,
) -> Result<()> {
    // Load manifest
    let load_dto = handling_manifest::LoadDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
    };
    handling_manifest_controller::load(&app_context.db_context, &app_context.event_hub, &load_dto)?;
    run_checks(app_context, output)?;

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

    // Step 5: Find the target file (by numeric ID or path suffix)
    let matches: Vec<&FileDto> = if let Ok(id) = args.target.parse::<u64>() {
        all_files.iter().filter(|f| f.id == id).collect()
    } else {
        all_files
            .iter()
            .filter(|f| {
                let path = format!("{}{}", f.relative_path, f.name);
                path.ends_with(&args.target) || path == args.target
            })
            .collect()
    };

    if matches.is_empty() {
        anyhow::bail!("No file matching '{}' found", args.target);
    }

    if matches.len() > 1 {
        eprintln!("Multiple files match '{}':", args.target);
        for f in &matches {
            eprintln!("  [{}] {}{}", f.id, f.relative_path, f.name);
        }
        anyhow::bail!(
            "Ambiguous target '{}' — use a more specific path or a numeric file ID",
            args.target
        );
    }

    let file = matches[0];

    // Step 6: Compute and display the diff
    let diff_dto = file_generation_shared_steps::GetDiffDto { file_id: file.id };
    let diff_out = file_generation_shared_steps_controller::get_file_diff(
        &app_context.db_context,
        &app_context.event_hub,
        &diff_dto,
    )?;

    if diff_out.diff_text.is_empty() {
        output.info(&format!(
            "No differences for {}{}",
            file.relative_path, file.name
        ));
    } else {
        print!("{}", diff_out.diff_text);
    }

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
