use std::env;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::{ShedError, utf8_path};
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::scan::SourceIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectKind {
    SingleCrate,
    WorkspaceRoot,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub root: Utf8PathBuf,
    pub manifest_path: Utf8PathBuf,
    pub manifest: Manifest,
    pub lockfile: Option<Lockfile>,
    pub source_index: SourceIndex,
    pub kind: ProjectKind,
}

impl Project {
    pub fn load(manifest_path: Option<&Utf8PathBuf>) -> Result<Self, ShedError> {
        let manifest_path = resolve_manifest_path(manifest_path)?;

        if !manifest_path.exists() {
            return Err(ShedError::ManifestNotFound {
                path: manifest_path,
            });
        }

        let root = manifest_path
            .parent()
            .map(Utf8Path::to_path_buf)
            .unwrap_or_else(|| Utf8PathBuf::from("."));
        let manifest = Manifest::load(&manifest_path)?;
        let lockfile_path = root.join("Cargo.lock");
        let lockfile = if lockfile_path.exists() {
            Some(Lockfile::load(&lockfile_path)?)
        } else {
            None
        };
        let source_index = SourceIndex::scan(&root)?;
        let kind = if manifest.is_workspace_root() {
            ProjectKind::WorkspaceRoot
        } else {
            ProjectKind::SingleCrate
        };

        Ok(Self {
            root,
            manifest_path,
            manifest,
            lockfile,
            source_index,
            kind,
        })
    }

    pub fn skipped_checks(&self) -> Vec<String> {
        if self.lockfile.is_none() {
            vec!["Cargo.lock was not found, so lockfile-based rules were skipped".to_owned()]
        } else {
            Vec::new()
        }
    }
}

fn resolve_manifest_path(path: Option<&Utf8PathBuf>) -> Result<Utf8PathBuf, ShedError> {
    match path {
        Some(path) if path.is_dir() => Ok(path.join("Cargo.toml")),
        Some(path) => Ok(path.to_owned()),
        None => {
            let cwd = utf8_path(env::current_dir().map_err(|source| ShedError::Read {
                path: Utf8PathBuf::from("."),
                source,
            })?)?;
            Ok(cwd.join("Cargo.toml"))
        }
    }
}

pub(crate) fn read_to_string(path: &Utf8Path) -> Result<String, ShedError> {
    fs::read_to_string(path).map_err(|source| ShedError::Read {
        path: path.to_path_buf(),
        source,
    })
}
