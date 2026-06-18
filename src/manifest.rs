use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Item, TableLike, Value};

use crate::error::ShedError;
use crate::project::read_to_string;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencySection {
    Normal,
    Dev,
    Build,
    Workspace,
}

impl DependencySection {
    pub fn manifest_key(&self) -> &'static str {
        match self {
            Self::Normal => "dependencies",
            Self::Dev => "dev-dependencies",
            Self::Build => "build-dependencies",
            Self::Workspace => "workspace.dependencies",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub section: DependencySection,
    pub name: String,
    pub package: Option<String>,
    pub version: Option<String>,
    pub default_features: Option<bool>,
    pub features: Vec<String>,
    pub optional: bool,
    pub path: Option<Utf8PathBuf>,
    pub workspace: bool,
}

impl Dependency {
    pub fn crate_name(&self) -> String {
        self.name.replace('-', "_")
    }
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub path: Utf8PathBuf,
    pub raw: String,
    pub document: DocumentMut,
    pub dependencies: Vec<Dependency>,
}

impl Manifest {
    pub fn load(path: &Utf8Path) -> Result<Self, ShedError> {
        let raw = read_to_string(path)?;
        let document = raw
            .parse::<DocumentMut>()
            .map_err(|error| ShedError::Parse {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        let dependencies = collect_dependencies(&document);

        Ok(Self {
            path: path.to_path_buf(),
            raw,
            document,
            dependencies,
        })
    }

    pub fn is_workspace_root(&self) -> bool {
        self.document.get("workspace").is_some()
    }

    pub fn package_name(&self) -> Option<&str> {
        self.document
            .get("package")
            .and_then(Item::as_table_like)
            .and_then(|table| table.get("name"))
            .and_then(Item::as_str)
    }

    pub fn dependency_item(&self, section: &DependencySection, name: &str) -> Option<&Item> {
        dependency_table(&self.document, section)?.get(name)
    }

    pub fn dependency_is_simple_string(&self, section: &DependencySection, name: &str) -> bool {
        self.dependency_item(section, name)
            .is_some_and(|item| item.as_str().is_some())
    }
}

fn dependency_table<'a>(
    document: &'a DocumentMut,
    section: &DependencySection,
) -> Option<&'a dyn TableLike> {
    match section {
        DependencySection::Normal => document.get("dependencies")?.as_table_like(),
        DependencySection::Dev => document.get("dev-dependencies")?.as_table_like(),
        DependencySection::Build => document.get("build-dependencies")?.as_table_like(),
        DependencySection::Workspace => document
            .get("workspace")?
            .as_table_like()?
            .get("dependencies")?
            .as_table_like(),
    }
}

fn collect_dependencies(document: &DocumentMut) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    collect_table(
        document.get("dependencies"),
        DependencySection::Normal,
        &mut dependencies,
    );
    collect_table(
        document.get("dev-dependencies"),
        DependencySection::Dev,
        &mut dependencies,
    );
    collect_table(
        document.get("build-dependencies"),
        DependencySection::Build,
        &mut dependencies,
    );

    let workspace_dependencies = document
        .get("workspace")
        .and_then(Item::as_table_like)
        .and_then(|workspace| workspace.get("dependencies"));
    collect_table(
        workspace_dependencies,
        DependencySection::Workspace,
        &mut dependencies,
    );

    dependencies
}

fn collect_table(
    item: Option<&Item>,
    section: DependencySection,
    dependencies: &mut Vec<Dependency>,
) {
    let Some(table) = item.and_then(Item::as_table_like) else {
        return;
    };

    for (name, value) in table.iter() {
        if let Some(dependency) = parse_dependency(section.clone(), name, value) {
            dependencies.push(dependency);
        }
    }
}

fn parse_dependency(section: DependencySection, name: &str, item: &Item) -> Option<Dependency> {
    if let Some(version) = item.as_str() {
        return Some(Dependency {
            section,
            name: name.to_owned(),
            package: None,
            version: Some(version.to_owned()),
            default_features: None,
            features: Vec::new(),
            optional: false,
            path: None,
            workspace: false,
        });
    }

    let table = item.as_table_like()?;

    Some(Dependency {
        section,
        name: name.to_owned(),
        package: get_str(table.get("package")),
        version: get_str(table.get("version")),
        default_features: get_bool(table.get("default-features")),
        features: get_string_array(table.get("features")),
        optional: get_bool(table.get("optional")).unwrap_or(false),
        path: get_str(table.get("path")).map(Utf8PathBuf::from),
        workspace: get_bool(table.get("workspace")).unwrap_or(false),
    })
}

fn get_str(item: Option<&Item>) -> Option<String> {
    item.and_then(Item::as_str).map(ToOwned::to_owned)
}

fn get_bool(item: Option<&Item>) -> Option<bool> {
    item.and_then(Item::as_bool)
}

fn get_string_array(item: Option<&Item>) -> Vec<String> {
    let Some(array) = item.and_then(Item::as_value).and_then(Value::as_array) else {
        return Vec::new();
    };

    array
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{DependencySection, collect_dependencies};

    #[test]
    fn parses_string_dependency() {
        let document = r#"
            [dependencies]
            serde = "1"
        "#
        .parse()
        .unwrap();

        let dependencies = collect_dependencies(&document);

        assert_eq!(dependencies[0].name, "serde");
        assert_eq!(dependencies[0].version.as_deref(), Some("1"));
    }

    #[test]
    fn parses_inline_dependency() {
        let document = r#"
            [dependencies]
            tokio = { version = "1", default-features = false, features = ["macros"], optional = true }
        "#
        .parse()
        .unwrap();

        let dependencies = collect_dependencies(&document);

        assert_eq!(dependencies[0].name, "tokio");
        assert_eq!(dependencies[0].default_features, Some(false));
        assert_eq!(dependencies[0].features, vec!["macros".to_owned()]);
        assert!(dependencies[0].optional);
    }

    #[test]
    fn parses_workspace_dependency() {
        let document = r#"
            [workspace.dependencies]
            anyhow = "1"
        "#
        .parse()
        .unwrap();

        let dependencies = collect_dependencies(&document);

        assert_eq!(dependencies[0].section, DependencySection::Workspace);
        assert_eq!(dependencies[0].name, "anyhow");
    }
}
