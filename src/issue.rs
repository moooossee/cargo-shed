use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
}

impl Severity {
    pub fn penalty(self) -> u8 {
        match self {
            Self::Low => 3,
            Self::Medium => 8,
            Self::High => 15,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub label: String,
    pub value: String,
}

impl Evidence {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggestion {
    pub summary: String,
    pub command: Option<String>,
}

impl Suggestion {
    pub fn new(summary: impl Into<String>, command: Option<impl Into<String>>) -> Self {
        Self {
            summary: summary.into(),
            command: command.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub evidence: Vec<Evidence>,
    pub suggestion: Option<Suggestion>,
}

impl Issue {
    pub fn new(
        id: impl Into<String>,
        rule_id: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            rule_id: rule_id.into(),
            severity,
            message: message.into(),
            evidence: Vec::new(),
            suggestion: None,
        }
    }

    pub fn with_evidence(mut self, evidence: Evidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestion = Some(suggestion);
        self
    }
}
