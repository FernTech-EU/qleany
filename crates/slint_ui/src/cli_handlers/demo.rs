use crate::app_context::AppContext;
use crate::cli::{DemoArgs, LanguageOption, OutputContext};
use crate::cli_handlers::common::{TargetLanguage, prompt_language, run_checks};
use anyhow::{Result, bail};
use common::direct_access::system::SystemRelationshipField;
use common::long_operation::OperationStatus;
use cpp_qt_file_generation::cpp_qt_file_generation_controller;
use direct_access::{FileDto, file_controller, system_controller};
use file_generation_shared_steps::file_generation_shared_steps_controller;
use handling_manifest::{
    CreateDto, CreateLanguage, ManifestTemplate, handling_manifest_controller,
};
use rust_file_generation::rust_file_generation_controller;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// The root system entity ID (singleton in the database)
const ROOT_SYSTEM_ID: u64 = 1;

pub fn execute(
    app_context: &Arc<AppContext>,
    args: &DemoArgs,
    output: &OutputContext,
) -> Result<()> {
    let final_path = if args.path.canonicalize()? == PathBuf::from(".").canonicalize()? {
        args.path.join("qleany-demo")
    } else {
        args.path.clone()
    };
    let manifest_path = final_path.join("qleany.yaml");

    // Check for existing manifest
    if manifest_path.exists() && !args.force {
        bail!(
            "Manifest already exists at {}. Use --force to overwrite.",
            manifest_path.display()
        );
    }

    if args.rust && args.cpp_qt {
        bail!("Cannot specify both --rust and --cpp-qt. Please choose one.");
    }
    let target_language = match args.cpp_qt {
        true => LanguageOption::CppQt,
        false => match args.rust {
            true => LanguageOption::Rust,
            false => prompt_language()?,
        },
    };

    let manifest_template = ManifestTemplate::DataManagement;
    let organization_name = "FernTech".to_string();
    let application_name = "Demo".to_string();

    let options = match target_language {
        LanguageOption::Rust => vec!["rust_cli".to_string(), "rust_teksilo".to_string()],
        LanguageOption::CppQt => vec!["cpp_qt_qtquick".to_string(), "cpp_qt_qtwidgets".to_string()],
    };

    std::fs::create_dir_all(&final_path)?;

    let create_dto = CreateDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
        language: match target_language {
            LanguageOption::Rust => CreateLanguage::Rust,
            LanguageOption::CppQt => CreateLanguage::CppQt,
        },
        application_name,
        organization_name,
        manifest_template,
        options,
    };

    let return_dto = handling_manifest_controller::create(&app_context.db_context, &create_dto)?;

    output.success(&format!("Created {}", return_dto.manifest_path));

    let line_count = std::fs::read_to_string(&manifest_path)?.lines().count();

    // Load manifest
    let load_dto = handling_manifest::LoadDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
    };
    handling_manifest_controller::load(&app_context.db_context, &app_context.event_hub, &load_dto)?;
    run_checks(app_context, output)?;

    // generate code
    // Step 1: Fill file list in DB
    output.verbose("Populating file list...");
    match target_language {
        LanguageOption::Rust => {
            let dto = rust_file_generation::FillRustFilesDto {
                only_list_already_existing: false,
            };
            rust_file_generation_controller::fill_rust_files(
                &app_context.db_context,
                &app_context.event_hub,
                &dto,
            )?;
        }
        LanguageOption::CppQt => {
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
            LanguageOption::Rust => rust_file_generation_controller::fill_code_in_rust_files(
                &app_context.db_context,
                &app_context.event_hub,
                &mut long_op_manager,
            )?,
            LanguageOption::CppQt => cpp_qt_file_generation_controller::fill_code_in_cpp_qt_files(
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
        &ROOT_SYSTEM_ID,
        &SystemRelationshipField::Files,
    )?;

    let all_files: Vec<FileDto> = file_controller::get_multi(&app_context.db_context, &file_ids)?
        .into_iter()
        .flatten()
        .collect();

    let files: Vec<&FileDto> = all_files.iter().collect();

    output.verbose(&format!(
        "Writing {} files to {}...",
        files.len(),
        final_path.display()
    ));

    // Step 5: Write generated_code to disk
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

        let mut file_path = final_path.clone();
        if !file.relative_path.is_empty() {
            file_path = file_path.join(&file.relative_path);
        }
        std::fs::create_dir_all(&file_path)?;
        file_path = file_path.join(&file.name);

        std::fs::write(&file_path, code)?;
        output.verbose(&format!("  {}{}", file.relative_path, file.name));
        written += 1;
    }
    output.info("");
    output.success(&format!(
        "Generated {} files{} , for a manifest of {} lines.",
        written,
        if skipped > 0 {
            format!(" ({} skipped - no generated code)", skipped)
        } else {
            String::new()
        },
        line_count
    ));

    crate::cli_handlers::common::detect_and_warn_of_missing_formatters(
        &TargetLanguage::from(target_language),
        output,
        true,
    )?;

    if target_language == LanguageOption::CppQt {
        crate::cli_handlers::common::detect_and_warn_of_missing_qcoro(output)?;
        output.info("Ensuring git repository and tag...");
        detect_and_init_git_and_tag(&final_path)?;
    }

    if !output.quiet {
        output.info("");
        output.info("You have now a fully generated application with:");
        output.info(" - complete CRUD infrastructure (controllers, DTOs, use cases, repositories)");
        output.info(" - multi-stack undo/redo system with cascade snapshot/restore");
        output.info(" - thread-safe event system with buffering (events sent only on success)");
        output.info(" - relationship management with ordering and cascade deletion");
        output.info(" - generated test suite");
        if target_language == LanguageOption::CppQt {
            output.info(" - scaffolds for multiple UIs: QtQuick and QtWidgets");
        }
        if target_language == LanguageOption::Rust {
            output.info(" - scaffolds for multiple UIs: CLI and Slint");
        }
        output.info("");
        output.info("Documentation: https://qleany-docs.pages.dev/");
        output.info("Or run: qleany docs");
        output.info("");
        output.info("Next steps:");
        output.info(&format!("  1. cd {}", final_path.display()));
        if target_language == LanguageOption::Rust {
            output.info("  2. cargo run --bin demo");
        } else if target_language == LanguageOption::CppQt {
            output.info("  2. mkdir build && cd build && cmake .. && cmake --build .  --target all -j$(nproc)");
            output.info("  3. ./src/qtquick_app/DemoApp");
            output.info("  4. ./src/qtwidgets_app/DemoDesktop");
        }
    }

    Ok(())
}

/// Polls a long operation until it completes, reporting progress if verbose.
fn poll_long_operation(
    app_context: &Arc<AppContext>,
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

fn detect_and_init_git_and_tag(path: &PathBuf) -> Result<()> {
    use std::process::{Command, Stdio};

    // Check if git is available
    if Command::new("git").arg("--version").output().is_err() {
        bail!("Git is not installed or not available in PATH");
    }

    // Check if .git exists
    if !path.join(".git").exists() {
        // Initialize git repository (suppress hints)
        let output = Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !output.success() {
            bail!("Failed to initialize git repository");
        }
    }

    // detect the presence of one tag "vX.X.X" pattern, if not present create tag v0.0.1
    let tags_output = Command::new("git")
        .args(["tag", "--list", "v*.*.*"])
        .current_dir(path)
        .output()?;

    let tag_list = String::from_utf8_lossy(&tags_output.stdout);
    if tags_output.status.success() && !tag_list.trim().is_empty() {
        return Ok(());
    }

    // Add all files and commit
    let add_status = Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .stdout(Stdio::null())
        .status()?;
    if !add_status.success() {
        bail!("Failed to stage files with git add");
    }

    let commit_status = Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !commit_status.success() {
        bail!("Failed to create initial git commit (is git user.name/user.email configured?)");
    }

    // Create tag v0.0.1
    let tag_status = Command::new("git")
        .args(["tag", "v0.0.1"])
        .current_dir(path)
        .status()?;
    if !tag_status.success() {
        bail!("Failed to create git tag v0.0.1");
    }

    Ok(())
}
