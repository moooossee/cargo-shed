//! Static dependency health checks for Rust projects.
//!
//! `cargo-shed` reads a project's manifest, lockfile, and Rust sources without
//! compiling the project or running build scripts. The main entry point is
//! [`analyze`]. Safe edits are available through [`apply_fixes`].
//!
//! ```no_run
//! use cargo_shed::{Config, OutputFormat, analyze};
//!
//! let report = analyze(Config {
//!     manifest_path: Some("Cargo.toml".into()),
//!     format: OutputFormat::Json,
//!     ..Config::default()
//! })?;
//!
//! println!("{}", report.to_json()?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod cli;
pub mod error;
pub mod explain;
pub mod fix;
pub mod issue;
pub mod lockfile;
pub mod manifest;
pub mod project;
pub mod report;
pub mod rules;
pub mod scan;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

pub use error::ShedError;
pub use fix::{Edit, Fix, FixReport, FixSafety, apply_fixes};
pub use issue::{Evidence, Issue, Severity, Suggestion};
pub use lockfile::{LockDependency, LockPackage, Lockfile};
pub use manifest::{Dependency, DependencySection, Manifest};
pub use project::{Project, ProjectKind};
pub use report::Report;
pub use rules::Rule;
pub use scan::{SourceFile, SourceIndex};

/// Output encoding for reports produced by the CLI and library helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    /// Human-readable terminal output.
    Human,
    /// Stable JSON output for machines and CI integrations.
    Json,
}

/// Runtime configuration for project analysis and safe fix application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Path to `Cargo.toml`, or a directory containing it.
    pub manifest_path: Option<Utf8PathBuf>,
    /// Whether the CLI should apply safe fixes instead of only reporting.
    pub fix: bool,
    /// Whether medium and high severity findings should fail CI.
    pub check: bool,
    /// Output format requested by the caller.
    pub format: OutputFormat,
    /// Optional rule or issue id to fix.
    pub selected_rule: Option<String>,
    /// Disable terminal color when supported by renderers.
    pub no_color: bool,
    /// Enable additional diagnostic output when supported.
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            manifest_path: None,
            fix: false,
            check: false,
            format: OutputFormat::Human,
            selected_rule: None,
            no_color: false,
            verbose: false,
        }
    }
}

pub fn analyze(config: Config) -> Result<Report, ShedError> {
    let project = Project::load(config.manifest_path.as_ref())?;
    let issues = rules::run_all(&project);
    let skipped_checks = project.skipped_checks();
    Ok(Report::new(project.root, issues, skipped_checks))
}
