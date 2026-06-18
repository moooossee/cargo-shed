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
pub use fix::{FixReport, apply_fixes};
pub use issue::{Evidence, Issue, Severity, Suggestion};
pub use project::Project;
pub use report::Report;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub manifest_path: Option<Utf8PathBuf>,
    pub fix: bool,
    pub check: bool,
    pub format: OutputFormat,
    pub selected_rule: Option<String>,
    pub no_color: bool,
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
