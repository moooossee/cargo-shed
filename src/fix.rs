use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::error::ShedError;
use crate::{Config, Project};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edit {
    RemoveDependency {
        section: crate::manifest::DependencySection,
        name: String,
    },
    ReplaceDependencyFeatures {
        section: crate::manifest::DependencySection,
        name: String,
        features: Vec<String>,
        default_features: Option<bool>,
    },
    SetDefaultFeatures {
        section: crate::manifest::DependencySection,
        name: String,
        value: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixSafety {
    Safe,
    Risky,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fix {
    pub issue_id: String,
    pub edits: Vec<Edit>,
    pub safety: FixSafety,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixReport {
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: bool,
}

impl FixReport {
    pub fn to_human(&self) -> String {
        let mut out = String::new();

        if self.applied.is_empty() && self.skipped.is_empty() {
            out.push_str("No safe fixes are available yet.\n");
            return out;
        }

        if !self.applied.is_empty() {
            out.push_str("Applied:\n");

            for item in &self.applied {
                let _ = writeln!(out, "- {item}");
            }
        }

        if !self.skipped.is_empty() {
            out.push_str("Skipped:\n");

            for item in &self.skipped {
                let _ = writeln!(out, "- {item}");
            }
        }

        out
    }
}

pub fn apply_fixes(config: Config) -> Result<FixReport, ShedError> {
    let _project = Project::load(config.manifest_path.as_ref())?;

    Ok(FixReport {
        applied: Vec::new(),
        skipped: config
            .selected_rule
            .map(|rule| format!("{rule} is not implemented in the fix engine yet"))
            .into_iter()
            .collect(),
        failed: false,
    })
}
