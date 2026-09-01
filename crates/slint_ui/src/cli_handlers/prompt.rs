use crate::app_context::AppContext;
use crate::cli::{OutputContext, PromptArgs};
use crate::cli_handlers::common::run_checks;
use crate::cli_handlers::common::{TargetLanguage, get_target_language};
use anyhow::{Result, anyhow};
use cpp_qt_file_generation::GenerateCppQtPromptDto;
use cpp_qt_file_generation::cpp_qt_file_generation_controller;
use direct_access::{feature_controller, global_controller, use_case_controller};
use handling_manifest::handling_manifest_controller;
use heck::AsSnakeCase;
use rust_file_generation::GenerateRustPromptDto;
use rust_file_generation::rust_file_generation_controller;
use std::path::Path;
use std::sync::Arc;

pub fn execute(
    app_context: &Arc<AppContext>,
    manifest_path: &Path,
    args: &PromptArgs,
    output: &OutputContext,
) -> Result<()> {
    // No flags provided → show help
    if !args.list && !args.context && args.use_case.is_none() {
        use clap::CommandFactory;
        let mut cmd = crate::cli::Cli::command();
        // Print help for the "prompt" subcommand
        for sub in cmd.get_subcommands_mut() {
            if sub.get_name() == "prompt" {
                sub.print_help()?;
                println!();
                return Ok(());
            }
        }
    }

    // Load manifest
    let load_dto = handling_manifest::LoadDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
    };
    handling_manifest_controller::load(&app_context.db_context, &app_context.event_hub, &load_dto)?;
    run_checks(app_context, output)?;

    let target_language = get_target_language(app_context)?;

    if args.list {
        return list_use_cases(app_context, manifest_path, &target_language);
    }
    match target_language {
        TargetLanguage::CppQt => {
            let dto = if args.context {
                GenerateCppQtPromptDto {
                    use_case_id: None,
                    context: true,
                    feature_id: None,
                }
            } else {
                // split feature/use_case string to feature and use_case
                let use_case_arg = args.use_case.as_ref().ok_or_else(|| {
                    anyhow!("use_case must be in format feature_name:use_case_name")
                })?;
                let (feature_name, use_case_name) =
                    use_case_arg.split_once(':').ok_or_else(|| {
                        anyhow!("use_case must be in format feature_name:use_case_name")
                    })?;

                // find feature_id from string
                let all_features = feature_controller::get_all(&app_context.db_context)?;
                let feature = all_features
                    .into_iter()
                    .find(|f| f.name == feature_name)
                    .ok_or_else(|| anyhow!("Feature with name {} not found", feature_name))?;

                // find use_case_id from string
                let use_cases = use_case_controller::get_all(&app_context.db_context)?;
                let use_case = use_cases
                    .into_iter()
                    .find(|uc| uc.name == use_case_name && feature.use_cases.contains(&uc.id))
                    .ok_or_else(|| {
                        anyhow!(
                            "Use case with name {} not found in feature {}",
                            use_case_name,
                            feature_name
                        )
                    })?;

                GenerateCppQtPromptDto {
                    use_case_id: Some(use_case.id),
                    context: false,
                    feature_id: Some(feature.id),
                }
            };

            let return_dto = cpp_qt_file_generation_controller::generate_cpp_qt_prompt(
                &app_context.db_context,
                &app_context.event_hub,
                &dto,
            )?;
            println!("{}", return_dto.prompt_text);

            Ok(())
        }
        TargetLanguage::Rust => {
            let dto = if args.context {
                GenerateRustPromptDto {
                    use_case_id: None,
                    context: true,
                    feature_id: None,
                }
            } else {
                let use_case_arg = args.use_case.as_ref().ok_or_else(|| {
                    anyhow!("use_case must be in format feature_name:use_case_name")
                })?;
                let (feature_name, use_case_name) =
                    use_case_arg.split_once(':').ok_or_else(|| {
                        anyhow!("use_case must be in format feature_name:use_case_name")
                    })?;

                let all_features = feature_controller::get_all(&app_context.db_context)?;
                let feature = all_features
                    .into_iter()
                    .find(|f| f.name == feature_name)
                    .ok_or_else(|| anyhow!("Feature with name {} not found", feature_name))?;

                let use_cases = use_case_controller::get_all(&app_context.db_context)?;
                let use_case = use_cases
                    .into_iter()
                    .find(|uc| uc.name == use_case_name && feature.use_cases.contains(&uc.id))
                    .ok_or_else(|| {
                        anyhow!(
                            "Use case with name {} not found in feature {}",
                            use_case_name,
                            feature_name
                        )
                    })?;

                GenerateRustPromptDto {
                    use_case_id: Some(use_case.id),
                    context: false,
                    feature_id: Some(feature.id),
                }
            };

            let return_dto = rust_file_generation_controller::generate_rust_prompt(
                &app_context.db_context,
                &app_context.event_hub,
                &dto,
            )?;
            println!("{}", return_dto.prompt_text);

            Ok(())
        }
    }
}

fn list_use_cases(
    app_context: &Arc<AppContext>,
    manifest_path: &Path,
    target_language: &TargetLanguage,
) -> Result<()> {
    let globals = global_controller::get_all(&app_context.db_context)?;
    let global = globals
        .first()
        .ok_or_else(|| anyhow!("No global configuration found"))?;
    let prefix_path = &global.prefix_path;

    let root_path = manifest_path
        .parent()
        .ok_or_else(|| anyhow!("Cannot determine manifest directory"))?;

    let all_features = feature_controller::get_all(&app_context.db_context)?;

    if all_features.is_empty() {
        println!("No features defined in manifest.");
        return Ok(());
    }

    let all_use_cases = use_case_controller::get_all(&app_context.db_context)?;

    for feature in &all_features {
        let feature_snake = AsSnakeCase(&feature.name).to_string();
        println!("{}:", feature.name);

        for uc in &all_use_cases {
            if !feature.use_cases.contains(&uc.id) {
                continue;
            }

            let uc_snake = AsSnakeCase(&uc.name).to_string();
            let status = match target_language {
                TargetLanguage::CppQt => {
                    check_cpp_qt_implementation(root_path, prefix_path, &feature_snake, &uc_snake)
                }
                TargetLanguage::Rust => {
                    check_rust_implementation(root_path, prefix_path, &feature_snake, &uc_snake)
                }
            };

            println!("  {}:{}{}", feature.name, uc.name, status);
        }
    }

    Ok(())
}

fn check_cpp_qt_implementation(
    root_path: &Path,
    prefix_path: &str,
    feature_snake: &str,
    uc_snake: &str,
) -> &'static str {
    let path = root_path
        .join(prefix_path)
        .join(feature_snake)
        .join("use_cases")
        .join(format!("{}_uc.cpp", uc_snake));

    check_file_implementation(
        &path,
        &["qCritical(\"Unimplemented code:", "Q_UNIMPLEMENTED"],
    )
}

fn check_rust_implementation(
    root_path: &Path,
    prefix_path: &str,
    feature_snake: &str,
    uc_snake: &str,
) -> &'static str {
    let path = root_path
        .join(prefix_path)
        .join(feature_snake)
        .join("src")
        .join("use_cases")
        .join(format!("{}_uc.rs", uc_snake));

    check_file_implementation(&path, &["unimplemented!", "todo!"])
}

fn check_file_implementation(path: &Path, markers: &[&str]) -> &'static str {
    if !path.exists() {
        return " [NOT GENERATED]";
    }
    match std::fs::read_to_string(path) {
        Ok(content) => {
            if markers.iter().any(|m| content.contains(m)) {
                " [NOT IMPLEMENTED]"
            } else {
                ""
            }
        }
        Err(_) => " [UNREADABLE]",
    }
}
