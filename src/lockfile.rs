use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Item, Value};

use crate::error::ShedError;
use crate::project::read_to_string;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockPackage {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
    pub checksum: Option<String>,
    pub dependencies: Vec<LockDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockDependency {
    pub raw: String,
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lockfile {
    pub path: Utf8PathBuf,
    pub version: Option<i64>,
    pub packages: Vec<LockPackage>,
}

impl Lockfile {
    pub fn load(path: &Utf8Path) -> Result<Self, ShedError> {
        let raw = read_to_string(path)?;
        let document = raw
            .parse::<DocumentMut>()
            .map_err(|error| ShedError::Parse {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;

        Ok(Self {
            path: path.to_path_buf(),
            version: document.get("version").and_then(Item::as_integer),
            packages: parse_packages(document.get("package")),
        })
    }

    pub fn versions_by_name(&self) -> BTreeMap<String, Vec<String>> {
        let mut grouped = BTreeMap::<String, Vec<String>>::new();

        for package in &self.packages {
            let versions = grouped.entry(package.name.clone()).or_default();

            if !versions.contains(&package.version) {
                versions.push(package.version.clone());
            }
        }

        for versions in grouped.values_mut() {
            versions.sort();
        }

        grouped
    }

    pub fn duplicate_versions(&self) -> BTreeMap<String, Vec<String>> {
        self.versions_by_name()
            .into_iter()
            .filter(|(_, versions)| versions.len() > 1)
            .collect()
    }
}

fn parse_packages(item: Option<&Item>) -> Vec<LockPackage> {
    let Some(packages) = item.and_then(Item::as_array_of_tables) else {
        return Vec::new();
    };

    packages.iter().filter_map(parse_package).collect()
}

fn parse_package(table: &toml_edit::Table) -> Option<LockPackage> {
    Some(LockPackage {
        name: table.get("name")?.as_str()?.to_owned(),
        version: table.get("version")?.as_str()?.to_owned(),
        source: table
            .get("source")
            .and_then(Item::as_str)
            .map(ToOwned::to_owned),
        checksum: table
            .get("checksum")
            .and_then(Item::as_str)
            .map(ToOwned::to_owned),
        dependencies: parse_dependency_array(table.get("dependencies")),
    })
}

fn parse_dependency_array(item: Option<&Item>) -> Vec<LockDependency> {
    let Some(array) = item.and_then(Item::as_value).and_then(Value::as_array) else {
        return Vec::new();
    };

    array
        .iter()
        .filter_map(Value::as_str)
        .map(parse_dependency)
        .collect()
}

fn parse_dependency(raw: &str) -> LockDependency {
    let mut parts = raw.split_whitespace();
    let name = parts.next().unwrap_or(raw).to_owned();
    let version = parts
        .next()
        .filter(|part| part.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
        .map(ToOwned::to_owned);

    LockDependency {
        raw: raw.to_owned(),
        name,
        version,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_dependency, parse_packages};

    #[test]
    fn parses_dependency_name_and_version() {
        let dependency =
            parse_dependency("syn 2.0.106 (registry+https://github.com/rust-lang/crates.io-index)");

        assert_eq!(dependency.name, "syn");
        assert_eq!(dependency.version.as_deref(), Some("2.0.106"));
    }

    #[test]
    fn groups_package_entries() {
        let document = r#"
            version = 4

            [[package]]
            name = "syn"
            version = "1.0.109"

            [[package]]
            name = "syn"
            version = "2.0.106"
            dependencies = [
              "proc-macro2",
              "quote 1.0.40",
            ]
        "#
        .parse::<toml_edit::DocumentMut>()
        .unwrap();

        let packages = parse_packages(document.get("package"));

        assert_eq!(packages.len(), 2);
        assert_eq!(packages[1].dependencies[0].name, "proc-macro2");
        assert_eq!(
            packages[1].dependencies[1].version.as_deref(),
            Some("1.0.40")
        );
    }
}
