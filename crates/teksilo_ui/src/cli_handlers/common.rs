use crate::cli::{LanguageOption, OutputContext};
use anyhow::{Result, bail};
use frontend::AppContext;
use frontend::EntityId;
use handling_manifest::handling_manifest_controller;
use std::io::{self, IsTerminal, Write};
use std::rc::Rc;

/// Check that stdin is a terminal. Call this before any interactive prompt
/// so that non-interactive callers (CI, LLMs) get a clear error instead of
/// hanging forever on a blocking `read_line`.
pub fn require_interactive(missing_flag_hint: &str) -> Result<()> {
    if !io::stdin().is_terminal() {
        bail!(
            "Interactive input required but stdin is not a terminal. \
             Use {} to run non-interactively.",
            missing_flag_hint
        );
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum TargetLanguage {
    Rust,
    CppQt,
}

impl From<LanguageOption> for TargetLanguage {
    fn from(lang: LanguageOption) -> Self {
        match lang {
            LanguageOption::Rust => TargetLanguage::Rust,
            LanguageOption::CppQt => TargetLanguage::CppQt,
        }
    }
}

pub fn prompt_language() -> Result<LanguageOption> {
    require_interactive("--language (or --rust / --cpp-qt for demo)")?;
    println!("Target language:");
    println!("  1. C++/Qt - C++ 20 and Qt 6");
    println!("  2. Rust   - 2024 edition");
    print!("Choose [1]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    match input.trim() {
        "" | "1" | "cpp-qt" | "cpp_qt" => Ok(LanguageOption::CppQt),
        "2" | "rust" => Ok(LanguageOption::Rust),
        other => bail!("Invalid language choice: '{}'", other),
    }
}
pub fn get_target_language(app_context: &Rc<AppContext>) -> Result<TargetLanguage> {
    use direct_access::global_controller;

    let global_dtos = global_controller::get_all(&app_context.db_context)?;
    let global_dto = global_dtos
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No global configuration found"))?;

    Ok(match global_dto.language.as_str() {
        "cpp-qt" => TargetLanguage::CppQt,
        "rust" => TargetLanguage::Rust,
        _ => anyhow::bail!("Unsupported language: {}", global_dto.language),
    })
}

/// Run semantic checks on the loaded manifest. Prints warnings/errors and
/// returns an error if any critical errors are found.
pub fn run_checks(app_context: &Rc<AppContext>, output: &OutputContext) -> Result<()> {
    let check_result =
        handling_manifest_controller::check(&app_context.db_context, &app_context.event_hub)?;

    for warning in &check_result.warnings {
        output.warn(&format!("Warning: {}", warning));
    }
    for error in &check_result.critical_errors {
        eprintln!("✗ Error: {}", error);
    }

    if !check_result.critical_errors.is_empty() {
        anyhow::bail!(
            "Manifest has {} critical error(s)",
            check_result.critical_errors.len()
        );
    }

    Ok(())
}

pub fn detect_and_warn_of_missing_formatters(
    target_language: &TargetLanguage,
    output: &OutputContext,
    bailing_out_if_missing: bool,
) -> Result<()> {
    use std::process::Command;

    let missing = if *target_language == TargetLanguage::Rust {
        let ok = Command::new("rustfmt")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            output.warn("rustfmt is not installed or not in PATH.");
            output.warn("The generated code may not be formatted correctly.");
            output.warn("Install rustup from https://rustup.rs/ and follow the instructions. This will install Rust and rustfmt.");
            true
        } else {
            false
        }
    } else if *target_language == TargetLanguage::CppQt {
        let clang_ok = Command::new("clang-format")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !clang_ok {
            output.warn("clang-format is not installed or not in PATH.");
            output.warn("The generated C++/Qt code may not be formatted correctly.");
            match detect_distro_family() {
                DistroFamily::Debian => output.warn("Install: sudo apt install clang-format"),
                DistroFamily::Fedora => output.warn("Install: sudo dnf install clang-tools-extra"),
                DistroFamily::Arch => output.warn("Install: sudo pacman -S clang"),
                DistroFamily::Unknown => output.warn(
                    "Install clang-format from your package manager or https://releases.llvm.org",
                ),
            }
        }

        let qml_ok = Command::new("qmlformat")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            || Command::new("/usr/lib/qt6/bin/qmlformat")
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
        if !qml_ok {
            output.warn("qmlformat is not installed or not in PATH.");
            output.warn("The generated QML code may not be formatted correctly.");
            match detect_distro_family() {
                DistroFamily::Debian => {
                    output.warn("Install: sudo apt install qt6-declarative-dev-tools")
                }
                DistroFamily::Fedora => {
                    output.warn("Install: sudo dnf install qt6-qtdeclarative-devel")
                }
                DistroFamily::Arch => output.warn("Install: sudo pacman -S qt6-declarative"),
                DistroFamily::Unknown => output.warn(
                    "Install qmlformat from your package manager (part of Qt6 declarative tools)",
                ),
            }
        }

        !clang_ok || !qml_ok
    } else {
        false
    };

    if missing && bailing_out_if_missing {
        anyhow::bail!("Required formatter is missing");
    }

    Ok(())
}

pub fn detect_and_warn_of_missing_qcoro(output: &OutputContext) -> Result<()> {
    // Look for QCoro6Config.cmake in standard cmake search paths
    let cmake_search_dirs = [
        "/usr/lib/cmake",
        "/usr/lib/x86_64-linux-gnu/cmake",
        "/usr/lib/aarch64-linux-gnu/cmake",
        "/usr/lib64/cmake",
        "/usr/local/lib/cmake",
        "/usr/local/lib64/cmake",
        "/usr/share/cmake",
    ];

    let found = cmake_search_dirs.iter().any(|dir| {
        std::path::Path::new(dir)
            .join("QCoro6")
            .join("QCoro6Config.cmake")
            .exists()
    });

    if found {
        return Ok(());
    }

    output.warn("QCoro6 does not appear to be installed.");
    output.warn("QCoro is required to build the generated C++/Qt code.");

    // Detect distro family from /etc/os-release
    let distro_family = detect_distro_family();

    match distro_family {
        DistroFamily::Debian => {
            output.warn("Install on Debian/Ubuntu:");
            output.warn("  sudo apt install qcoro-qt6-dev");
        }
        DistroFamily::Fedora => {
            output.warn("Install on Fedora:");
            output.warn("  sudo dnf install qcoro-qt6-devel");
        }
        DistroFamily::Arch => {
            output.warn("Install on Arch:");
            output.warn("  sudo pacman -S qcoro-qt6");
        }
        DistroFamily::Unknown => {
            output.warn("Install QCoro from source: https://github.com/danvratil/qcoro");
        }
    }

    Ok(())
}

enum DistroFamily {
    Debian,
    Fedora,
    Arch,
    Unknown,
}

fn detect_distro_family() -> DistroFamily {
    let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();

    for line in os_release.lines() {
        if let Some(value) = line.strip_prefix("ID_LIKE=") {
            let value = value.trim_matches('"').to_lowercase();
            if value.contains("debian") || value.contains("ubuntu") {
                return DistroFamily::Debian;
            }
            if value.contains("fedora") || value.contains("rhel") || value.contains("centos") {
                return DistroFamily::Fedora;
            }
            if value.contains("arch") {
                return DistroFamily::Arch;
            }
        }
        if let Some(value) = line.strip_prefix("ID=") {
            let value = value.trim_matches('"').to_lowercase();
            if value == "debian" || value == "ubuntu" {
                return DistroFamily::Debian;
            }
            if value == "fedora" || value == "rhel" || value == "centos" {
                return DistroFamily::Fedora;
            }
            if value == "arch" || value == "manjaro" || value == "endeavouros" {
                return DistroFamily::Arch;
            }
        }
    }

    DistroFamily::Unknown
}

/// The `System` row, read from `Root` rather than assumed to be 1.
///
/// Four CLI handlers each carried `const ROOT_SYSTEM_ID: u64 = 1`, and two
/// Slint screens carried a fifth and sixth copy. That is true of a freshly
/// seeded store and stops being true the moment anything is created before it,
/// at which point the handler silently reads another entity's relationships.
///
/// The GUI resolves the same thing through [`crate::bootstrap::system_id`] and
/// keeps it in `AppIds`; a CLI process is one command long, so it reads it where
/// it needs it.
pub fn system_id(app_context: &Rc<AppContext>) -> Result<EntityId> {
    crate::bootstrap::system_id(app_context)
        .ok_or_else(|| anyhow::anyhow!("no System in the store; the application failed to start"))
}
