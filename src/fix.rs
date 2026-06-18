use std::fmt::Write as _;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use toml_edit::{Array, DocumentMut, Item, TableLike};

use crate::error::ShedError;
use crate::manifest::DependencySection;
use crate::{Config, Project, rules};

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
    pub backup_path: Option<Utf8PathBuf>,
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

        if let Some(path) = &self.backup_path {
            let _ = writeln!(out, "Backup created: {path}\n");
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
    let project = Project::load(config.manifest_path.as_ref())?;
    let selection = config.selected_rule.as_deref();
    let mut report = FixReport {
        backup_path: None,
        applied: Vec::new(),
        skipped: Vec::new(),
        failed: false,
    };
    let collected = collect_fixes(&project, selection)?;

    report.skipped = collected.skipped;

    if collected.matched_requested_issue
        && (!report.skipped.is_empty() || collected.fixes.is_empty())
    {
        report.failed = true;
    }

    if collected.fixes.is_empty() {
        return Ok(report);
    }

    let mut document = project.manifest.document.clone();
    let mut applied = Vec::new();

    for fix in &collected.fixes {
        for edit in &fix.edits {
            if let Err(message) = apply_edit(&mut document, edit) {
                report.skipped.push(message);
                report.failed = true;
                return Ok(report);
            }

            applied.push(describe_edit(edit));
        }
    }

    let backup_path = create_backup(&project.manifest.path, &project.manifest.raw)?;
    write_atomic(&project.manifest.path, document.to_string().as_bytes())?;

    report.backup_path = Some(backup_path);
    report.applied = applied;
    Ok(report)
}

struct CollectedFixes {
    fixes: Vec<Fix>,
    skipped: Vec<String>,
    matched_requested_issue: bool,
}

fn collect_fixes(project: &Project, selection: Option<&str>) -> Result<CollectedFixes, ShedError> {
    let registry = rules::registry();
    let known_rule = selection.is_some_and(|selected| {
        registry.iter().any(|rule| rule.id() == selected) || selected.contains(':')
    });

    if let Some(selected) = selection
        && !known_rule
    {
        return Err(ShedError::UnknownRule {
            rule_id: selected.to_owned(),
        });
    }

    let mut fixes = Vec::new();
    let mut skipped = Vec::new();
    let mut matched_any_issue = false;
    let mut matched_requested_issue = false;

    for rule in registry {
        for issue in rule.check(project) {
            if !matches_selection(selection, rule.id(), &issue.id) {
                continue;
            }

            matched_any_issue = true;
            matched_requested_issue |= selection.is_some();

            let Some(fix) = rule.fix(project, &issue) else {
                skipped.push(format!("{} has no safe automatic fix", issue.id));
                continue;
            };

            if fix.safety == FixSafety::Safe {
                fixes.push(fix);
            } else {
                skipped.push(format!("{} requires manual review", issue.id));
            }
        }
    }

    if let Some(selected) = selection
        && !matched_any_issue
    {
        skipped.push(format!("No issues matched {selected}"));
    }

    Ok(CollectedFixes {
        fixes,
        skipped,
        matched_requested_issue,
    })
}

fn matches_selection(selection: Option<&str>, rule_id: &str, issue_id: &str) -> bool {
    match selection {
        Some(selected) => selected == rule_id || selected == issue_id,
        None => true,
    }
}

fn apply_edit(document: &mut DocumentMut, edit: &Edit) -> Result<(), String> {
    match edit {
        Edit::RemoveDependency { section, name } => {
            let table = dependency_table_mut(document, section)
                .ok_or_else(|| format!("could not find {}", section.manifest_key()))?;

            table
                .remove(name)
                .ok_or_else(|| format!("could not remove {}", dependency_path(section, name)))?;
        }
        Edit::ReplaceDependencyFeatures {
            section,
            name,
            features,
            default_features,
        } => {
            let dependency = dependency_item_mut(document, section, name)?;
            let table = dependency.as_table_like_mut().ok_or_else(|| {
                format!(
                    "{} is not an editable dependency table",
                    dependency_path(section, name)
                )
            })?;

            table.insert("features", feature_array(features));

            if let Some(value) = default_features {
                table.insert("default-features", toml_edit::value(*value));
            }

            table.fmt();
        }
        Edit::SetDefaultFeatures {
            section,
            name,
            value: enabled,
        } => {
            let dependency = dependency_item_mut(document, section, name)?;
            let table = dependency.as_table_like_mut().ok_or_else(|| {
                format!(
                    "{} is not an editable dependency table",
                    dependency_path(section, name)
                )
            })?;

            table.insert("default-features", toml_edit::value(*enabled));
            table.fmt();
        }
    }

    Ok(())
}

fn dependency_item_mut<'a>(
    document: &'a mut DocumentMut,
    section: &DependencySection,
    name: &str,
) -> Result<&'a mut Item, String> {
    dependency_table_mut(document, section)
        .ok_or_else(|| format!("could not find {}", section.manifest_key()))?
        .get_mut(name)
        .ok_or_else(|| format!("could not find {}", dependency_path(section, name)))
}

fn dependency_table_mut<'a>(
    document: &'a mut DocumentMut,
    section: &DependencySection,
) -> Option<&'a mut dyn TableLike> {
    match section {
        DependencySection::Normal => document.get_mut("dependencies")?.as_table_like_mut(),
        DependencySection::Dev => document.get_mut("dev-dependencies")?.as_table_like_mut(),
        DependencySection::Build => document.get_mut("build-dependencies")?.as_table_like_mut(),
        DependencySection::Workspace => document
            .get_mut("workspace")?
            .as_table_like_mut()?
            .get_mut("dependencies")?
            .as_table_like_mut(),
    }
}

fn feature_array(features: &[String]) -> Item {
    let mut array = Array::new();

    for feature in features {
        array.push(feature.as_str());
    }

    array.fmt();
    toml_edit::value(array)
}

fn describe_edit(edit: &Edit) -> String {
    match edit {
        Edit::RemoveDependency { section, name } => {
            format!("removed {name} from {}", section.manifest_key())
        }
        Edit::ReplaceDependencyFeatures { section, name, .. } => {
            format!("optimized {name} features in {}", section.manifest_key())
        }
        Edit::SetDefaultFeatures {
            section,
            name,
            value,
        } => {
            format!(
                "set default-features = {value} for {name} in {}",
                section.manifest_key()
            )
        }
    }
}

fn dependency_path(section: &DependencySection, name: &str) -> String {
    format!("{}.{}", section.manifest_key(), name)
}

fn create_backup(manifest_path: &Utf8Path, raw: &str) -> Result<Utf8PathBuf, ShedError> {
    let backup_path = available_backup_path(manifest_path);
    fs::write(&backup_path, raw).map_err(|source| ShedError::Write {
        path: backup_path.clone(),
        source,
    })?;
    Ok(backup_path)
}

fn available_backup_path(manifest_path: &Utf8Path) -> Utf8PathBuf {
    let file_name = manifest_path.file_name().unwrap_or("Cargo.toml");
    let backup_name = format!("{file_name}.shed.bak");
    let first = manifest_path.with_file_name(&backup_name);

    if !first.exists() {
        return first;
    }

    for index in 1.. {
        let candidate = manifest_path.with_file_name(format!("{backup_name}.{index}"));

        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!()
}

fn write_atomic(path: &Utf8Path, contents: &[u8]) -> Result<(), ShedError> {
    let temp_path = available_temp_path(path);

    fs::write(&temp_path, contents).map_err(|source| ShedError::Write {
        path: temp_path.clone(),
        source,
    })?;

    fs::rename(&temp_path, path).map_err(|source| {
        let _ = fs::remove_file(&temp_path);
        ShedError::Write {
            path: path.to_path_buf(),
            source,
        }
    })
}

fn available_temp_path(path: &Utf8Path) -> Utf8PathBuf {
    let file_name = path.file_name().unwrap_or("Cargo.toml");
    let first = path.with_file_name(format!("{file_name}.shed.tmp"));

    if !first.exists() {
        return first;
    }

    for index in 1.. {
        let candidate = path.with_file_name(format!("{file_name}.shed.tmp.{index}"));

        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!()
}
