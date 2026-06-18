use std::fmt::Write as _;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::issue::{Issue, Severity};

const REPORT_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueSummary {
    pub total: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

impl IssueSummary {
    fn from_issues(issues: &[Issue]) -> Self {
        let mut summary = Self {
            total: issues.len(),
            ..Self::default()
        };

        for issue in issues {
            match issue.severity {
                Severity::High => summary.high += 1,
                Severity::Medium => summary.medium += 1,
                Severity::Low => summary.low += 1,
            }
        }

        summary
    }

    fn to_human(self) -> String {
        if self.total == 0 {
            return "0".to_owned();
        }

        let mut parts = Vec::new();
        push_count(&mut parts, self.high, "high");
        push_count(&mut parts, self.medium, "medium");
        push_count(&mut parts, self.low, "low");
        parts.join(", ")
    }
}

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
            .map(|issue| u16::from(issue.severity.penalty()))
            .sum::<u16>();
        let score = 100u8.saturating_sub(penalty.min(100) as u8);

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

    pub fn summary(&self) -> IssueSummary {
        IssueSummary::from_issues(&self.issues)
    }

    pub fn to_human(&self) -> String {
        let mut out = String::new();
        out.push_str("cargo-shed report\n\n");
        let root = &self.root;
        let score = self.score;
        let summary = self.summary();
        let _ = writeln!(out, "Project: {root}");
        let _ = writeln!(out, "Score: {score}/100");
        let _ = writeln!(out, "Issues: {}\n", summary.to_human());

        if self.issues.is_empty() {
            out.push_str("No issues found.\n");
        } else {
            out.push_str("Problems found:\n\n");

            for issue in &self.issues {
                let severity = issue.severity.as_str().to_uppercase();
                let id = &issue.id;
                let message = &issue.message;
                let _ = writeln!(out, "[{severity}] {id}");
                let _ = writeln!(out, "Reason: {message}");

                if !issue.evidence.is_empty() {
                    out.push_str("Evidence:\n");

                    for evidence in &issue.evidence {
                        let label = &evidence.label;
                        let value = &evidence.value;
                        let _ = writeln!(out, "- {label}: {value}");
                    }
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
            out.push_str("Skipped checks:\n");

            for skipped in &self.skipped_checks {
                let _ = writeln!(out, "- {skipped}");
            }
        }

        out
    }

    pub fn to_check_human(&self) -> String {
        let mut out = String::new();

        if !self.has_ci_failures() {
            out.push_str("cargo-shed check passed\n");
            let score = self.score;
            let _ = writeln!(out, "Score: {score}/100");
            return out;
        }

        out.push_str("cargo-shed check failed\n\n");
        let summary = self.summary();
        let blocking = IssueSummary {
            total: summary.high + summary.medium,
            high: summary.high,
            medium: summary.medium,
            low: 0,
        };
        let _ = writeln!(out, "Blocking issues: {}", blocking.to_human());
        out.push_str("\nAction required:\n");

        for issue in self.issues.iter().filter(|issue| {
            matches!(
                issue.severity,
                crate::issue::Severity::High | crate::issue::Severity::Medium
            )
        }) {
            let severity = issue.severity.as_str().to_uppercase();
            let id = &issue.id;
            let message = &issue.message;
            let _ = write!(out, "- [{severity}] {id}: {message}");

            if let Some(command) = issue
                .suggestion
                .as_ref()
                .and_then(|suggestion| suggestion.command.as_ref())
            {
                let _ = write!(out, " ({command})");
            }

            out.push('\n');
        }

        out.push_str("\nRun cargo shed for the full report.\n");
        out
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&JsonReport::from(self))
    }
}

#[derive(Serialize)]
struct JsonReport<'a> {
    schema_version: u8,
    root: &'a str,
    score: u8,
    summary: IssueSummary,
    issues: &'a [Issue],
    skipped_checks: &'a [String],
}

impl<'a> From<&'a Report> for JsonReport<'a> {
    fn from(report: &'a Report) -> Self {
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            root: report.root.as_str(),
            score: report.score,
            summary: report.summary(),
            issues: &report.issues,
            skipped_checks: &report.skipped_checks,
        }
    }
}

fn push_count(parts: &mut Vec<String>, count: usize, label: &str) {
    if count > 0 {
        parts.push(format!("{count} {label}"));
    }
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;

    use crate::issue::{Issue, Severity};

    use super::Report;

    fn issue(severity: Severity) -> Issue {
        Issue::new("test-issue", "test-rule", severity, "test message")
    }

    #[test]
    fn score_saturates_at_zero() {
        let issues = (0..20).map(|_| issue(Severity::High)).collect();
        let report = Report::new(Utf8PathBuf::from("."), issues, Vec::new());

        assert_eq!(report.score, 0);
    }

    #[test]
    fn summary_counts_severities() {
        let report = Report::new(
            Utf8PathBuf::from("."),
            vec![
                issue(Severity::High),
                issue(Severity::Medium),
                issue(Severity::Medium),
                issue(Severity::Low),
            ],
            Vec::new(),
        );
        let summary = report.summary();

        assert_eq!(summary.total, 4);
        assert_eq!(summary.high, 1);
        assert_eq!(summary.medium, 2);
        assert_eq!(summary.low, 1);
    }
}
