use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::ShedError;
use crate::project::read_to_string;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFile {
    pub path: Utf8PathBuf,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceIndex {
    pub files: Vec<SourceFile>,
}

impl SourceIndex {
    pub fn scan(root: &Utf8Path) -> Result<Self, ShedError> {
        let mut files = Vec::new();

        scan_dir(&root.join("src"), &mut files)?;
        scan_dir(&root.join("tests"), &mut files)?;

        files.sort_by(|left, right| left.path.cmp(&right.path));

        Ok(Self { files })
    }

    pub fn contains_token(&self, token: &str) -> bool {
        self.files.iter().any(|file| file.text.contains(token))
    }

    pub fn crate_appears_used(&self, crate_name: &str) -> bool {
        self.files
            .iter()
            .any(|file| source_mentions_crate(file.text.as_str(), crate_name))
    }

    pub fn has_ambiguous_generation(&self) -> bool {
        self.contains_token("include!(")
            || self.contains_token("macro_rules!")
            || self.contains_token("proc_macro")
    }
}

fn scan_dir(path: &Utf8Path, files: &mut Vec<SourceFile>) -> Result<(), ShedError> {
    if !path.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(path).map_err(|source| ShedError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| ShedError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|path| ShedError::NonUtf8Path { path })?;

        if path.is_dir() {
            scan_dir(&path, files)?;
        } else if path.extension() == Some("rs") {
            let text = read_to_string(&path)?;
            files.push(SourceFile { path, text });
        }
    }

    Ok(())
}

fn source_mentions_crate(text: &str, crate_name: &str) -> bool {
    contains_crate_suffix(text, crate_name, "::")
        || contains_crate_suffix(text, crate_name, "!")
        || contains_crate_suffix(text, crate_name, ";")
        || contains_extern_crate(text, crate_name)
        || contains_dynamic(text, format!("use {crate_name} as "))
        || contains_dynamic(text, format!("#[{crate_name}]"))
        || contains_dynamic(text, format!("#[{crate_name}("))
}

fn contains_extern_crate(text: &str, crate_name: &str) -> bool {
    let needle = format!("extern crate {crate_name}");
    let mut offset = 0;

    while let Some(index) = text[offset..].find(needle.as_str()) {
        let end = offset + index + needle.len();

        if text
            .as_bytes()
            .get(end)
            .is_none_or(|byte| !is_ident_byte(*byte))
        {
            return true;
        }

        offset = end;
    }

    false
}

fn contains_crate_suffix(text: &str, crate_name: &str, suffix: &str) -> bool {
    let needle = format!("{crate_name}{suffix}");
    let mut offset = 0;

    while let Some(index) = text[offset..].find(needle.as_str()) {
        let absolute = offset + index;

        if absolute == 0 || !is_ident_byte(text.as_bytes()[absolute - 1]) {
            return true;
        }

        offset = absolute + crate_name.len();
    }

    false
}

fn is_ident_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

fn contains_dynamic(text: &str, pattern: String) -> bool {
    text.contains(pattern.as_str())
}

#[cfg(test)]
mod tests {
    use super::source_mentions_crate;

    #[test]
    fn detects_crate_paths() {
        assert!(source_mentions_crate(
            "use serde_json::Value;",
            "serde_json"
        ));
    }

    #[test]
    fn ignores_longer_identifier_prefixes() {
        assert!(!source_mentions_crate("extern crate serde_json;", "serde"));
    }
}
