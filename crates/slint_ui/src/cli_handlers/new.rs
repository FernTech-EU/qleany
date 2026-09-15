use crate::app_context::AppContext;
use crate::cli::{LanguageOption, ManifestTemplateOption, NewArgs, OutputContext};
use crate::cli_handlers::common::{prompt_language, require_interactive};
use anyhow::{Result, bail};
use handling_manifest::{
    CreateDto, CreateLanguage, ManifestTemplate, handling_manifest_controller,
};
use std::io::{self, Write};
use std::sync::Arc;

pub fn execute(
    app_context: &Arc<AppContext>,
    args: &NewArgs,
    output: &OutputContext,
) -> Result<()> {
    let manifest_path = args.path.join("qleany.yaml");

    // Check for existing manifest
    if manifest_path.exists() && !args.force {
        bail!(
            "Manifest already exists at {}. Use --force to overwrite.",
            manifest_path.display()
        );
    }

    // Resolve language (interactive if missing)
    let language = match &args.language {
        Some(lang) => *lang,
        None => prompt_language()?,
    };

    // Resolve application name (interactive if missing)
    let application_name = match &args.name {
        Some(name) => name.clone(),
        None => prompt_string("Application name (PascalCase, e.g. MyApp)", "MyApp")?,
    };

    // Resolve organisation name (interactive if missing)
    let organization_name = match &args.org_name {
        Some(name) => name.clone(),
        None => prompt_string("Organisation name (e.g. FernTech)", "MyOrganization")?,
    };

    // Resolve manifest template (interactive if missing)
    let template = match &args.template {
        Some(t) => *t,
        None => prompt_template()?,
    };

    // Resolve options (interactive if missing)
    let options = if args.options.is_empty() {
        prompt_options(&language)?
    } else {
        args.options.clone()
    };

    output.verbose(&format!(
        "Creating new manifest at {}",
        manifest_path.display()
    ));

    // Create directory structure if needed
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let create_dto = CreateDto {
        manifest_path: manifest_path.to_string_lossy().to_string(),
        language: match language {
            LanguageOption::Rust => CreateLanguage::Rust,
            LanguageOption::CppQt => CreateLanguage::CppQt,
        },
        application_name,
        organization_name,
        manifest_template: match template {
            ManifestTemplateOption::Blank => ManifestTemplate::Blank,
            ManifestTemplateOption::Minimal => ManifestTemplate::Minimal,
            ManifestTemplateOption::DocumentEditor => ManifestTemplate::DocumentEditor,
            ManifestTemplateOption::DataManagement => ManifestTemplate::DataManagement,
        },
        options,
    };

    let return_dto = handling_manifest_controller::create(
        &app_context.db_context,
        &app_context.event_hub,
        &create_dto,
    )?;

    output.success(&format!("Created {}", return_dto.manifest_path));

    if !output.quiet {
        output.info("");
        output.info("If not already done, create a git repository and commit the initial manifest, and tag:");
        output.info("'git init && git add . && git commit -m\"initial commit\" && git tag v0.0.1'");
        output.info("(A git tag is mandatory with a C++/Qt manifest)");
        output.info("");
        output.info("Next steps:");
        output.info("  1. Edit qleany.yaml to define your entities and features");
        output.info("  2. Run 'qleany check' to validate the manifest");
        output.info("  3. Run 'qleany list' to see the list of files to be generated");
        output.info("  4. Run 'qleany generate --temp' to preview generated files in ./temp/");
        output.info("  5. Run 'qleany generate' to generate files");
        output.info("  6. Run 'qleany diff [file]' to see offered changes to specific files");
        output.info("  7. Run 'qleany prompt' if you are using LLMs");
        output.info("");
        output.info("Or run 'qleany' to start the UI");
    }

    Ok(())
}

fn prompt_string(prompt: &str, example: &str) -> Result<String> {
    require_interactive(&format!("the appropriate CLI flag (default: {})", example))?;
    print!("{} [{}]: ", prompt, example);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_string();
    if input.is_empty() {
        Ok(example.to_string())
    } else {
        Ok(input)
    }
}

fn prompt_template() -> Result<ManifestTemplateOption> {
    require_interactive("--template")?;
    println!("Manifest template:");
    println!("  1. blank          - EntityBase + empty Root");
    println!(
        "  2. minimal        - Root with one entity (Item with title). Hello world equivalent"
    );
    println!("  3. document-editor - Root > Documents > Sections with load/save use cases");
    println!("  4. data-management - Items, Categories, Tags with import/export use cases");
    print!("Choose [1]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    match input.trim() {
        "" | "1" | "blank" => Ok(ManifestTemplateOption::Blank),
        "2" | "minimal" => Ok(ManifestTemplateOption::Minimal),
        "3" | "document-editor" => Ok(ManifestTemplateOption::DocumentEditor),
        "4" | "data-management" => Ok(ManifestTemplateOption::DataManagement),
        other => bail!("Invalid template choice: '{}'", other),
    }
}

fn prompt_options(language: &LanguageOption) -> Result<Vec<String>> {
    require_interactive("--options")?;
    let (available, default) = match language {
        LanguageOption::Rust => (
            vec![
                "rust_teksilo",
                "rust_cli",
                "rust_slint",
                "rust_ios",
                "rust_android",
            ],
            "rust_teksilo",
        ),
        LanguageOption::CppQt => (vec!["cpp_qt_qtquick", "cpp_qt_qtwidgets"], "cpp_qt_qtquick"),
    };
    println!("UI options (comma-separated, or 'none' for no UI):");
    for opt in &available {
        let marker = if *opt == default { " (default)" } else { "" };
        println!("  - {}{}", opt, marker);
    }
    print!("Choose [{}]: ", default);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    if input.is_empty() {
        Ok(vec![default.to_string()])
    } else if input.eq_ignore_ascii_case("none") {
        Ok(vec![])
    } else {
        Ok(input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }
}
