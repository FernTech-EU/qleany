//! Demo Wizard module
//!
//! Handles the GUI demo wizard: language selection, path browsing,
//! and background generation of a complete sample project.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use common::direct_access::system::SystemRelationshipField;
use common::long_operation::OperationStatus;
use cpp_qt_file_generation::cpp_qt_file_generation_controller;
use direct_access::{FileDto, file_controller, system_controller};
use file_generation_shared_steps::file_generation_shared_steps_controller;
use handling_manifest::{
    CreateDto, CreateLanguage, ManifestTemplate, handling_manifest_controller,
};
use rust_file_generation::rust_file_generation_controller;
use slint::ComponentHandle;

use crate::app_context::AppContext;
use crate::cli::LanguageOption;
use crate::cli_handlers::common::run_checks;
use crate::{App, DemoWizardState};

const ROOT_SYSTEM_ID: u64 = 1;

pub fn init(app: &App, app_context: &Arc<AppContext>) {
    setup_browse_path_callback(app);
    setup_start_demo_callback(app, app_context);
    setup_open_result_folder_callback(app);
    setup_copy_next_step_callback(app);
}

/// Result returned by the generation thread to populate the summary UI.
struct DemoResult {
    written: usize,
    manifest_lines: usize,
}

fn setup_browse_path_callback(app: &App) {
    app.global::<DemoWizardState>().on_browse_path({
        let app_weak = app.as_weak();
        move || {
            let default_path = dirs::home_dir().unwrap_or_default();
            let folder = rfd::FileDialog::new()
                .set_directory(&default_path)
                .pick_folder();
            if let Some(path) = folder {
                // The user picks the parent directory; we append qleany-demo
                let target = path.join("qleany-demo");
                if let Some(app) = app_weak.upgrade() {
                    app.global::<DemoWizardState>()
                        .set_output_path(slint::SharedString::from(
                            target.to_string_lossy().to_string(),
                        ));
                }
            }
        }
    });
}

fn setup_start_demo_callback(app: &App, app_context: &Arc<AppContext>) {
    app.global::<DemoWizardState>().on_start_demo({
        let ctx = Arc::clone(app_context);
        let app_weak = app.as_weak();
        move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };

            let wiz = app.global::<DemoWizardState>();
            let language_index = wiz.get_language_index();
            let output_path_str = wiz.get_output_path().to_string().trim().to_string();

            // Resolve output path:
            // - Empty → ~/qleany-demo
            // - Tilde prefix → expand to home dir
            // - The path is used as-is (the Browse callback already appends /qleany-demo)
            let output_path = if output_path_str.is_empty() {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("qleany-demo")
            } else {
                expand_tilde(&output_path_str)
            };

            let target_language = if language_index == 0 {
                LanguageOption::CppQt
            } else {
                LanguageOption::Rust
            };

            // Switch to step 1 (running)
            wiz.set_step(1);
            wiz.set_is_running(true);
            wiz.set_progress(0.0);
            wiz.set_progress_message(slint::SharedString::from("Preparing..."));
            wiz.set_error_message(slint::SharedString::from(""));

            let ctx = Arc::clone(&ctx);
            let app_weak = app.as_weak();

            std::thread::spawn(move || {
                let result =
                    run_demo_generation(&ctx, &app_weak, target_language, &output_path);

                let app_weak2 = app_weak.clone();
                let path_display = output_path.to_string_lossy().to_string();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak2.upgrade() {
                        let wiz = app.global::<DemoWizardState>();
                        wiz.set_is_running(false);
                        match result {
                            Ok(demo_result) => {
                                wiz.set_completed(true);
                                wiz.set_progress(100.0);
                                wiz.set_result_path(slint::SharedString::from(&path_display));
                                wiz.set_copied_to_clipboard(false);

                                // Summary stats
                                wiz.set_summary_stats(slint::SharedString::from(format!(
                                    "Generated {} files from a manifest of {} lines.",
                                    demo_result.written, demo_result.manifest_lines
                                )));

                                // UI line
                                let ui_line = match target_language {
                                    LanguageOption::Rust => "Scaffolds for multiple UIs: CLI and Slint",
                                    LanguageOption::CppQt => "Scaffolds for multiple UIs: QtQuick and QtWidgets",
                                };
                                wiz.set_summary_ui_line(slint::SharedString::from(ui_line));

                                // Next step command
                                let next_cmd = match target_language {
                                    LanguageOption::Rust => format!(
                                        "cd {} && cargo run --bin demo",
                                        path_display
                                    ),
                                    LanguageOption::CppQt => format!(
                                        "cd {} && mkdir build && cd build && cmake .. && cmake --build . --target all -j$(nproc)",
                                        path_display
                                    ),
                                };
                                wiz.set_next_step_command(slint::SharedString::from(next_cmd));
                            }
                            Err(e) => {
                                wiz.set_error_message(slint::SharedString::from(format!(
                                    "{}",
                                    e
                                )));
                            }
                        }
                    }
                });
            });
        }
    });
}

fn setup_open_result_folder_callback(app: &App) {
    app.global::<DemoWizardState>().on_open_result_folder({
        let app_weak = app.as_weak();
        move || {
            if let Some(app) = app_weak.upgrade() {
                let path = app
                    .global::<DemoWizardState>()
                    .get_result_path()
                    .to_string();
                if !path.is_empty() {
                    let _ = open::that(&path);
                }
            }
        }
    });
}

fn setup_copy_next_step_callback(app: &App) {
    app.global::<DemoWizardState>().on_copy_next_step({
        let app_weak = app.as_weak();
        move || {
            if let Some(app) = app_weak.upgrade() {
                let cmd = app
                    .global::<DemoWizardState>()
                    .get_next_step_command()
                    .to_string();
                if !cmd.is_empty() {
                    super::common::set_clipboard_text(cmd);
                    app.global::<DemoWizardState>()
                        .set_copied_to_clipboard(true);
                }
            }
        }
    });
}

/// Expand a leading `~` or `~/` to the user's home directory.
fn expand_tilde(path: &str) -> std::path::PathBuf {
    if path == "~" {
        dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
    } else if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(rest)
    } else {
        std::path::PathBuf::from(path)
    }
}

fn set_progress(app_weak: &slint::Weak<App>, progress: f32, message: &str) {
    let app_weak = app_weak.clone();
    let msg = message.to_string();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app_weak.upgrade() {
            let wiz = app.global::<DemoWizardState>();
            wiz.set_progress(progress);
            wiz.set_progress_message(slint::SharedString::from(msg));
        }
    });
}

fn run_demo_generation(
    app_context: &Arc<AppContext>,
    app_weak: &slint::Weak<App>,
    target_language: LanguageOption,
    final_path: &std::path::Path,
) -> Result<DemoResult> {
    let manifest_path = final_path.join("qleany.yaml");

    // Check for existing manifest
    if manifest_path.exists() {
        anyhow::bail!(
            "A project already exists at {}. Remove it first or choose another folder.",
            final_path.display()
        );
    }

    set_progress(app_weak, 5.0, "Creating output folder...");
    std::fs::create_dir_all(final_path)?;

    let application_name = "Demo".to_string();
    let organization_name = "FernTech".to_string();
    let manifest_template = ManifestTemplate::DataManagement;

    let options = match target_language {
        LanguageOption::Rust => vec!["rust_cli".to_string(), "rust_teksilo".to_string()],
        LanguageOption::CppQt => {
            vec!["cpp_qt_qtquick".to_string(), "cpp_qt_qtwidgets".to_string()]
        }
    };

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

    set_progress(app_weak, 10.0, "Creating manifest...");
    handling_manifest_controller::create(
        &app_context.db_context,
        &app_context.event_hub,
        &create_dto,
    )?;

    let manifest_lines = std::fs::read_to_string(&manifest_path)?.lines().count();

    set_progress(app_weak, 15.0, "Loading manifest...");
    let load_dto = handling_manifest::LoadDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
    };
    handling_manifest_controller::load(&app_context.db_context, &app_context.event_hub, &load_dto)?;

    set_progress(app_weak, 20.0, "Running checks...");
    let output = crate::cli::OutputContext {
        verbose: false,
        quiet: true,
    };
    run_checks(app_context, &output)?;

    // Fill file list
    set_progress(app_weak, 25.0, "Populating file list...");
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

    // Fill code (long operation)
    set_progress(app_weak, 30.0, "Generating code...");
    let operation_id = {
        let mut long_op_manager = app_context
            .long_operation_manager
            .lock()
            .unwrap_or_else(|_| panic!("Failed to acquire lock on long operation manager."));
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

    // Poll long operation (30% -> 80%)
    loop {
        std::thread::sleep(Duration::from_millis(100));
        let long_op_manager = app_context.long_operation_manager.lock().unwrap();

        let status = match long_op_manager.get_operation_status(&operation_id) {
            Some(s) => s,
            None => break,
        };

        if let Some(progress) = long_op_manager.get_operation_progress(&operation_id) {
            let mapped = 30.0 + progress.percentage * 0.5; // 30 -> 80
            let msg = progress.message.as_deref().unwrap_or("Generating code...");
            set_progress(app_weak, mapped, msg);
        }

        match status {
            OperationStatus::Running => {}
            OperationStatus::Completed => break,
            OperationStatus::Cancelled => anyhow::bail!("Operation was cancelled"),
            OperationStatus::Failed(err) => anyhow::bail!("Code generation failed: {}", err),
        }
    }

    // Fill status
    set_progress(app_weak, 85.0, "Comparing with files on disk...");
    file_generation_shared_steps_controller::fill_status_in_files(
        &app_context.db_context,
        &app_context.event_hub,
    )?;

    // Retrieve all files
    set_progress(app_weak, 90.0, "Writing files to disk...");
    let file_ids = system_controller::get_relationship(
        &app_context.db_context,
        &ROOT_SYSTEM_ID,
        &SystemRelationshipField::Files,
    )?;

    let all_files: Vec<FileDto> = file_controller::get_multi(&app_context.db_context, &file_ids)?
        .into_iter()
        .flatten()
        .collect();

    // Write files
    let mut written = 0;
    for file in &all_files {
        let Some(ref code) = file.generated_code else {
            continue;
        };

        let mut file_path = final_path.to_path_buf();
        if !file.relative_path.is_empty() {
            file_path = file_path.join(&file.relative_path);
        }
        std::fs::create_dir_all(&file_path)?;
        file_path = file_path.join(&file.name);
        std::fs::write(&file_path, code)?;
        written += 1;
    }

    // C++/Qt: init git and tag
    if target_language == LanguageOption::CppQt {
        set_progress(app_weak, 95.0, "Initializing git repository...");
        detect_and_init_git_and_tag(&final_path.to_path_buf())?;
    }

    set_progress(app_weak, 100.0, "Done!");
    Ok(DemoResult {
        written,
        manifest_lines,
    })
}

fn detect_and_init_git_and_tag(path: &std::path::PathBuf) -> Result<()> {
    use std::process::{Command, Stdio};

    if Command::new("git").arg("--version").output().is_err() {
        anyhow::bail!("Git is not installed or not available in PATH");
    }

    if !path.join(".git").exists() {
        let output = Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !output.success() {
            anyhow::bail!("Failed to initialize git repository");
        }
    }

    let tags_output = Command::new("git")
        .args(["tag", "--list", "v*.*.*"])
        .current_dir(path)
        .output()?;

    let tag_list = String::from_utf8_lossy(&tags_output.stdout);
    if tags_output.status.success() && !tag_list.trim().is_empty() {
        return Ok(());
    }

    let add_status = Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .stdout(Stdio::null())
        .status()?;
    if !add_status.success() {
        anyhow::bail!("Failed to stage files with git add");
    }

    let commit_status = Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !commit_status.success() {
        anyhow::bail!(
            "Failed to create initial git commit (is git user.name/user.email configured?)"
        );
    }

    let tag_status = Command::new("git")
        .args(["tag", "v0.0.1"])
        .current_dir(path)
        .status()?;
    if !tag_status.success() {
        anyhow::bail!("Failed to create git tag v0.0.1");
    }

    Ok(())
}
