use std::fmt::Write as _;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::issue::Issue;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub root: Utf8PathBuf,
    pub score: u8,
    pub issues: Vec<Issue>,
    pub skipped_checks: Vec<String>,
}

impl Report {
    pub fn new(root: Utf8PathBuf, issues: Vec<Issue>, skipped_checks: Vec<String>) -> Self {
        let penalty = issues
            .iter()
            .map(|issue| issue.severity.penalty())
            .sum::<u8>();
        let score = 100u8.saturating_sub(penalty);

        Self {
            root,
            score,
            issues,
            skipped_checks,
        }
    }

    pub fn has_ci_failures(&self) -> bool {
        self.issues.iter().any(|issue| {
            matches!(
                issue.severity,
                crate::issue::Severity::High | crate::issue::Severity::Medium
            )
        })
    }

    pub fn to_human(&self) -> String {
        let mut out = String::new();
        out.push_str("cargo-shed report\n\n");
        let root = &self.root;
        let score = self.score;
        let _ = writeln!(out, "Project: {root}");
        let _ = writeln!(out, "Score: {score}/100\n");

        if self.issues.is_empty() {
            out.push_str("No issues found yet.\n");
        } else {
            out.push_str("Problems found:\n\n");

            for issue in &self.issues {
                let severity = issue.severity.as_str().to_uppercase();
                let id = &issue.id;
                let message = &issue.message;
                let _ = writeln!(out, "[{severity}] {id}");
                let _ = writeln!(out, "{message}");

                for evidence in &issue.evidence {
                    let label = &evidence.label;
                    let value = &evidence.value;
                    let _ = writeln!(out, "{label}: {value}");
                }

                if let Some(suggestion) = &issue.suggestion {
                    let summary = &suggestion.summary;
                    let _ = writeln!(out, "Suggested: {summary}");

                    if let Some(command) = &suggestion.command {
                        let _ = writeln!(out, "Run: {command}");
                    }
                }

                out.push('\n');
            }
        }

        if !self.skipped_checks.is_empty() {
            out.push('\n');
            out.push_str("Skipped:\n");

            for skipped in &self.skipped_checks {
                let _ = writeln!(out, "- {skipped}");
            }
        }

        out
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
