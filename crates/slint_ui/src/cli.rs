use crate::app_context::AppContext;
use crate::cli_handlers;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(author, version)]
#[command(about = "Architecture generator for C++/Qt6 and Rust applications")]
#[command(before_help = concat!("Qleany v", env!("CARGO_PKG_VERSION"), " - made by FernTech"))]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to qleany.yaml manifest (searches current directory if not specified)
    #[arg(short, long, global = true)]
    pub manifest: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new qleany.yaml manifest
    New(NewArgs),

    /// Validate the manifest without generating files
    Check(CheckArgs),

    /// List files that would be generated
    List(ListArgs),

    /// Generate scaffolding code
    #[command(visible_alias = "gen")]
    Generate(GenerateArgs),

    /// Generate a project from a demo manifest
    Demo(DemoArgs),

    /// Display manifest information
    Show(ShowArgs),

    /// Export entity diagram
    Export(ExportArgs),

    /// Embedded documentation
    Docs(DocsArgs),

    /// Upgrade manifest to the latest schema version
    Upgrade,

    /// LLM Prompt
    Prompt(PromptArgs),

    /// Show unified diff between generated and on-disk file
    Diff(DiffArgs),

    /// Run Slint GUI to edit manifests
    Gui,
}

// ─────────────────────────────────────────────────────────────
// NEW
// ─────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct NewArgs {
    /// Directory where qleany.yaml will be created
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Target language for the project
    #[arg(short, long, value_enum)]
    pub language: Option<LanguageOption>,

    /// Application name (PascalCase, e.g. MyApp)
    #[arg(short, long)]
    pub name: Option<String>,

    /// Organisation name (e.g. FernTech)
    #[arg(long)]
    pub org_name: Option<String>,

    /// Manifest template to use
    #[arg(short, long, value_enum)]
    pub template: Option<ManifestTemplateOption>,

    /// UI options: rust_cli, rust_teksilo, rust_slint, cpp_qt_qtquick, cpp_qt_qtwidgets, rust_ios, rust_android
    #[arg(short, long)]
    pub options: Vec<String>,

    /// Overwrite existing manifest without prompting
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum ManifestTemplateOption {
    Blank,
    Minimal,
    DocumentEditor,
    DataManagement,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum LanguageOption {
    Rust,
    #[value(alias = "cpp-qt")]
    CppQt,
}

// ─────────────────────────────────────────────────────────────
// DEMO
// ─────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct DemoArgs {
    /// Directory where qleany.yaml will be created
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Target Rust language for the demo project
    #[arg(long)]
    pub rust: bool,

    /// Target C++ Qt language for the demo project
    #[arg(long)]
    pub cpp_qt: bool,

    /// force overwrite the existing qleany-demo folder
    #[arg(long)]
    pub force: bool,
}

// ─────────────────────────────────────────────────────────────
// CHECK
// ─────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct CheckArgs {
    /// List all checked rules instead of running validation
    #[arg(long)]
    pub rules: bool,
}

// ─────────────────────────────────────────────────────────────
// LIST
// ─────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct ListArgs {
    /// What to list: files, entities, features, groups [default: files]
    #[arg(value_enum, default_value = "files")]
    pub target: ListTarget,

    /// Show all files (all statuses + all natures)
    #[arg(long)]
    pub all: bool,

    // Status filters (default: Modified + New)
    /// Include modified files
    #[arg(long, short = 'M')]
    pub modified: bool,

    /// Include new files
    #[arg(long, short = 'N')]
    pub new: bool,

    /// Include unchanged files
    #[arg(long, short = 'U')]
    pub unchanged: bool,

    /// Include all statuses (modified + new + unchanged)
    #[arg(long)]
    pub all_status: bool,

    // Nature filters (default: all natures)
    /// Include infrastructure files
    #[arg(long, short = 'i')]
    pub infra: bool,

    /// Include aggregate files
    #[arg(long, short = 'g')]
    pub aggregates: bool,

    /// Include scaffold files
    #[arg(long, short = 's')]
    pub scaffolds: bool,

    /// Include all natures (infrastructure + aggregate + scaffold)
    #[arg(long)]
    pub all_natures: bool,

    /// Plain text output without colors
    #[arg(long)]
    pub text: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "plain")]
    pub format: OutputFormat,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum ListTarget {
    /// List all generated files (default)
    Files,

    /// List entities defined in manifest
    Entities,

    /// List features and their use cases
    Features,

    /// List file groups (for selective generation)
    Groups,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Plain,
    Json,
    Tree,
}

// ─────────────────────────────────────────────────────────────
// GENERATE
// ─────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct GenerateArgs {
    /// What to generate: all, feature, entity, file, group [default: all]
    #[arg(value_enum, default_value = "all")]
    pub target: GenerateTarget,

    /// Target name(s) — feature name, entity name, file path/ID, or group name
    #[arg(value_name = "NAME")]
    pub target_names: Vec<String>,

    /// Output directory (defaults to manifest's prefix_path)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Generate to ./temp/ subdirectory (safe for comparison)
    #[arg(long)]
    pub temp: bool,

    /// Show what would be generated without writing files
    #[arg(long)]
    pub dry_run: bool,

    /// Write all files (all statuses + all natures)
    #[arg(long)]
    pub all: bool,

    // Status filters (default: Modified + New)
    /// Include modified files
    #[arg(long, short = 'M')]
    pub modified: bool,

    /// Include new files
    #[arg(long, short = 'N')]
    pub new: bool,

    /// Include unchanged files
    #[arg(long, short = 'U')]
    pub unchanged: bool,

    /// Include all statuses (modified + new + unchanged)
    #[arg(long)]
    pub all_status: bool,

    // Nature filters (default: all natures)
    /// Include infrastructure files
    #[arg(long, short = 'i')]
    pub infra: bool,

    /// Include aggregate files
    #[arg(long, short = 'g')]
    pub aggregates: bool,

    /// Include scaffold files
    #[arg(long, short = 's')]
    pub scaffolds: bool,

    /// Include all natures (infrastructure + aggregate + scaffold)
    #[arg(long)]
    pub all_natures: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum GenerateTarget {
    /// Generate all files (default)
    All,

    /// Generate files for a specific feature
    Feature,

    /// Generate entity-related files
    Entity,

    /// Generate specific file(s) by path or ID
    File,

    /// Generate files matching a group
    Group,
}

// ─────────────────────────────────────────────────────────────
// SHOW
// ─────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct ShowArgs {
    /// What to display
    #[command(subcommand)]
    pub target: Option<ShowTarget>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "plain")]
    pub format: OutputFormat,
}

#[derive(Subcommand)]
pub enum ShowTarget {
    /// Show full manifest (default)
    Manifest,

    /// Show project configuration (global section)
    Config,

    /// Show details for a specific entity
    Entity { name: String },

    /// Show details for a specific feature
    Feature { name: String },
}

// ─────────────────────────────────────────────────────────────
// EXPORT
// ─────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct ExportArgs {
    /// Export format
    #[command(subcommand)]
    pub format: ExportFormat,

    /// Output file (stdout if not specified)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum ExportFormat {
    /// Export entity relationships as Mermaid diagram
    Mermaid,

    /// Export manifest as JSON
    Json,
}

// ─────────────────────────────────────────────────────────────
// DOC
// ─────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct DocsArgs {
    /// Which documentation to show
    #[command(subcommand)]
    pub target: Option<DocsTarget>,

    /// Output raw Markdown instead of terminal-formatted text
    #[arg(long, global = true)]
    pub md: bool,
}

#[derive(Subcommand, Clone)]
pub enum DocsTarget {
    /// Show all documentations
    All,

    /// Show introduction documentation
    #[command(visible_alias = "intro")]
    Introduction,

    /// Show manifest reference documentation
    #[command(visible_alias = "manifest")]
    ManifestReference,

    /// Show architecture design documentation
    #[command(visible_alias = "design")]
    DesignPhilosophy,

    /// Show how operations flow documentation
    #[command(visible_alias = "flow")]
    HowOperationsFlow,

    /// Show undo/redo architecture documentation
    #[command(visible_alias = "undo")]
    UndoRedoArchitecture,

    /// Show generated code documentation for C++/Qt
    #[command(visible_alias = "cpp")]
    GeneratedCodeCppQt,

    /// Show generated code documentation for Rust
    #[command(visible_alias = "rust")]
    GeneratedCodeRust,

    /// Show API reference for C++/Qt
    #[command(visible_alias = "api-cpp")]
    ApiReferenceCppQt,

    /// Show API reference for Rust
    #[command(visible_alias = "api-rust")]
    ApiReferenceRust,

    /// Show quick start guide for C++/Qt
    #[command(visible_alias = "start-cpp")]
    QuickStartCppQt,

    /// Show quick start guide for Rust
    #[command(visible_alias = "start-rust")]
    QuickStartRust,

    /// Show QML integration documentation
    #[command(visible_alias = "qml")]
    QmlIntegration,

    /// Show migration guide documentation
    #[command(visible_alias = "mig")]
    MigrationGuide,

    /// Show troubleshooting documentation
    #[command(visible_alias = "trouble")]
    Troubleshooting,

    /// Show regeneration workflow documentation
    #[command(visible_alias = "regen")]
    RegenerationWorkflow,

    /// Show mobile bridge development documentation
    #[command(visible_alias = "mobile")]
    MobileBridgeDevelopment,
}

// ─────────────────────────────────────────────────────────────
// Prompt
// ─────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct PromptArgs {
    /// List all use cases grouped by feature
    #[arg(short, long)]
    pub list: bool,

    /// Generate a project context
    #[arg(short, long)]
    pub context: bool,

    /// Generate a prompt for a specific use case. Write "feature:use_case".
    #[arg(short, long)]
    pub use_case: Option<String>,
}

// ─────────────────────────────────────────────────────────────
// DIFF
// ─────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct DiffArgs {
    /// File path (relative to output) or numeric file ID from `list files`
    pub target: String,
}

/// Run the CLI with the given application context.
/// Returns `Some(())` if the application should continue running as GUI, `None` otherwise.
pub fn run_cli(app_context: &Arc<AppContext>) -> Option<()> {
    let cli = Cli::parse();

    // No command provided → launch GUI
    let command = cli.command;

    let command = match command {
        Some(command) => command,
        None => return Some(()),
    };

    // Resolve manifest path for commands that need it
    let manifest_path = resolve_manifest_path(&cli.manifest, &command);

    // Create output context for consistent messaging
    let output = OutputContext {
        verbose: cli.verbose,
        quiet: cli.quiet,
    };

    let result = match command {
        Commands::New(args) => cli_handlers::new::execute(app_context, &args, &output),
        Commands::Check(args) => {
            if args.rules {
                cli_handlers::check::list_rules(&output);
                return None;
            }
            let path = manifest_path.expect("Check requires a manifest");
            cli_handlers::check::execute(app_context, &path, &output)
        }
        Commands::List(args) => {
            let path = manifest_path.expect("List requires a manifest");
            cli_handlers::list::execute(app_context, &path, &args, &output)
        }
        Commands::Generate(args) => {
            let path = manifest_path.expect("Generate requires a manifest");
            cli_handlers::generate::execute(app_context, &path, &args, &output)
        }
        Commands::Show(args) => {
            let path = manifest_path.expect("Show requires a manifest");
            cli_handlers::show::execute(app_context, &path, &args, &output)
        }
        Commands::Export(args) => {
            let path = manifest_path.expect("Export requires a manifest");
            cli_handlers::export::execute(app_context, &path, &args, &output)
        }
        Commands::Docs(args) => cli_handlers::docs::execute(app_context, &args, &output),
        Commands::Upgrade => {
            let path = manifest_path.expect("Upgrade requires a manifest");
            cli_handlers::upgrade::execute(app_context, &path, &output)
        }
        Commands::Prompt(args) => {
            let path = manifest_path.expect("Prompt requires a manifest");
            cli_handlers::prompt::execute(app_context, &path, &args, &output)
        }
        Commands::Diff(args) => {
            let path = manifest_path.expect("Diff requires a manifest");
            cli_handlers::diff::execute(app_context, &path, &args, &output)
        }
        Commands::Demo(args) => cli_handlers::demo::execute(app_context, &args, &output),
        Commands::Gui => return Some(()),
    };

    if let Err(e) = result {
        if !output.quiet {
            eprintln!("Error: {}", e);
        }
        std::process::exit(1);
    }

    None
}

/// Resolves the manifest path from CLI arguments or discovers it in the current directory.
fn resolve_manifest_path(explicit: &Option<PathBuf>, command: &Commands) -> Option<PathBuf> {
    // New, Demo, and Doc commands don't need an existing manifest
    if matches!(
        command,
        Commands::New(_) | Commands::Demo(_) | Commands::Docs(_)
    ) {
        return None;
    }

    // Use explicit path if provided
    if let Some(path) = explicit {
        if path.is_file() {
            return Some(path.clone());
        }
        if path.is_dir() {
            let manifest = path.join("qleany.yaml");
            if manifest.exists() {
                return Some(manifest);
            }
        }
        eprintln!("Manifest not found: {}", path.display());
        std::process::exit(1);
    }

    // Search current directory
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let candidates = ["qleany.yaml", "qleany.yml"];

    for candidate in candidates {
        let path = current_dir.join(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    eprintln!("No qleany.yaml found in current directory. Use --manifest to specify location.");
    std::process::exit(1);
}

/// Context for controlling CLI output behavior.
#[derive(Clone, Copy)]
pub struct OutputContext {
    pub verbose: bool,
    pub quiet: bool,
}

impl OutputContext {
    pub fn info(&self, msg: &str) {
        if !self.quiet {
            println!("{}", msg);
        }
    }

    pub fn verbose(&self, msg: &str) {
        if self.verbose && !self.quiet {
            println!("{}", msg);
        }
    }

    pub fn success(&self, msg: &str) {
        if !self.quiet {
            println!("✓ {}", msg);
        }
    }

    pub fn warn(&self, msg: &str) {
        if !self.quiet {
            eprintln!("⚠ {}", msg);
        }
    }
}
