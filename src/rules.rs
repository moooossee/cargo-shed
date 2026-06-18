use crate::Project;
use crate::fix::Fix;
use crate::issue::{Evidence, Issue, Severity, Suggestion};
use crate::manifest::{Dependency, DependencySection};

pub trait Rule {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn severity(&self) -> Severity;
    fn check(&self, project: &Project) -> Vec<Issue>;
    fn fix(&self, project: &Project, issue: &Issue) -> Option<Fix>;
}

pub fn registry() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(TokioFullRule),
        Box::new(ReqwestDefaultFeaturesRule),
        Box::new(UnusedDependencyRule),
        Box::new(DuplicateVersionsRule),
        Box::new(HeavyCrateRule),
    ]
}

pub fn run_all(project: &Project) -> Vec<Issue> {
    let mut issues = Vec::new();

    for rule in registry() {
        issues.extend(rule.check(project));
    }

    issues.sort_by(|left, right| {
        right
            .severity
            .cmp(&left.severity)
            .then_with(|| left.rule_id.cmp(&right.rule_id))
            .then_with(|| left.id.cmp(&right.id))
    });

    issues
}

struct TokioFullRule;

impl Rule for TokioFullRule {
    fn id(&self) -> &'static str {
        "tokio-full"
    }

    fn title(&self) -> &'static str {
        "Tokio full feature set"
    }

    fn severity(&self) -> Severity {
        Severity::High
    }

    fn check(&self, project: &Project) -> Vec<Issue> {
        project
            .manifest
            .dependencies
            .iter()
            .filter(|dependency| dependency_matches_package(dependency, "tokio"))
            .filter(|dependency| dependency.features.iter().any(|feature| feature == "full"))
            .map(|dependency| {
                let inferred_features = infer_tokio_features(project);
                let mut issue = Issue::new(
                    self.id(),
                    self.id(),
                    self.severity(),
                    "tokio uses the full feature set",
                )
                .with_evidence(Evidence::new("Dependency", dependency_path(dependency)))
                .with_evidence(Evidence::new(
                    "Current features",
                    format_features(&dependency.features),
                ));

                if inferred_features.is_empty() {
                    issue = issue.with_evidence(Evidence::new(
                        "Inferred features",
                        "no clear Tokio feature usage was found",
                    ));
                } else {
                    issue = issue.with_evidence(Evidence::new(
                        "Inferred features",
                        format_features(&inferred_features),
                    ));
                }

                issue.with_suggestion(Suggestion::new(
                    tokio_suggestion(&inferred_features),
                    None::<String>,
                ))
            })
            .collect()
    }

    fn fix(&self, _project: &Project, _issue: &Issue) -> Option<Fix> {
        None
    }
}

struct ReqwestDefaultFeaturesRule;

impl Rule for ReqwestDefaultFeaturesRule {
    fn id(&self) -> &'static str {
        "reqwest-default-features"
    }

    fn title(&self) -> &'static str {
        "Reqwest default features"
    }

    fn severity(&self) -> Severity {
        Severity::Low
    }

    fn check(&self, project: &Project) -> Vec<Issue> {
        project
            .manifest
            .dependencies
            .iter()
            .filter(|dependency| dependency_matches_package(dependency, "reqwest"))
            .filter(|dependency| !dependency.workspace)
            .filter(|dependency| dependency.default_features != Some(false))
            .map(|dependency| {
                let severity = reqwest_severity(dependency);

                Issue::new(
                    self.id(),
                    self.id(),
                    severity,
                    "reqwest keeps default features enabled",
                )
                .with_evidence(Evidence::new("Dependency", dependency_path(dependency)))
                .with_evidence(Evidence::new(
                    "Current default-features",
                    dependency
                        .default_features
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "implicit true".to_owned()),
                ))
                .with_evidence(Evidence::new(
                    "Current features",
                    format_features(&dependency.features),
                ))
                .with_suggestion(Suggestion::new(
                    reqwest_suggestion(dependency),
                    None::<String>,
                ))
            })
            .collect()
    }

    fn fix(&self, _project: &Project, _issue: &Issue) -> Option<Fix> {
        None
    }
}

struct UnusedDependencyRule;

impl Rule for UnusedDependencyRule {
    fn id(&self) -> &'static str {
        "unused-dependency"
    }

    fn title(&self) -> &'static str {
        "Possibly unused dependency"
    }

    fn severity(&self) -> Severity {
        Severity::Medium
    }

    fn check(&self, project: &Project) -> Vec<Issue> {
        project
            .manifest
            .dependencies
            .iter()
            .filter(|dependency| dependency.section != DependencySection::Workspace)
            .filter(|dependency| dependency.section != DependencySection::Build)
            .filter(|dependency| !dependency.optional)
            .filter(|dependency| !dependency.workspace)
            .filter(|dependency| {
                !project
                    .source_index
                    .crate_appears_used(dependency.crate_name().as_str())
            })
            .map(|dependency| {
                let crate_name = dependency.crate_name();
                let allowlisted = is_unused_allowlisted(&crate_name);
                let message = if allowlisted {
                    format!("{crate_name} may be unused, but it is commonly used indirectly")
                } else {
                    format!("{crate_name} appears to be unused in scanned Rust sources")
                };
                let mut issue = Issue::new(
                    unused_issue_id(dependency),
                    self.id(),
                    self.severity(),
                    message,
                )
                .with_evidence(Evidence::new("Dependency", dependency_path(dependency)))
                .with_evidence(Evidence::new("Crate name checked", crate_name.clone()))
                .with_evidence(Evidence::new(
                    "Scanned Rust files",
                    project.source_index.files.len().to_string(),
                ));

                if project.source_index.has_ambiguous_generation() {
                    issue = issue.with_evidence(Evidence::new(
                        "Source scan",
                        "generated or macro-heavy source was detected",
                    ));
                }

                issue.with_suggestion(Suggestion::new(
                    unused_suggestion(&crate_name, allowlisted),
                    None::<String>,
                ))
            })
            .collect()
    }

    fn fix(&self, _project: &Project, _issue: &Issue) -> Option<Fix> {
        None
    }
}

struct DuplicateVersionsRule;

impl Rule for DuplicateVersionsRule {
    fn id(&self) -> &'static str {
        "duplicate-versions"
    }

    fn title(&self) -> &'static str {
        "Duplicate crate versions"
    }

    fn severity(&self) -> Severity {
        Severity::Medium
    }

    fn check(&self, project: &Project) -> Vec<Issue> {
        let Some(lockfile) = &project.lockfile else {
            return Vec::new();
        };

        lockfile
            .duplicate_versions()
            .into_iter()
            .map(|(name, versions)| {
                let impact = duplicate_impact(name.as_str());
                let severity = if versions.len() >= 3 || impact.is_some() {
                    Severity::High
                } else {
                    self.severity()
                };
                let mut issue = Issue::new(
                    format!("duplicate-versions:{name}"),
                    self.id(),
                    severity,
                    format!("{name} appears in multiple versions"),
                )
                .with_evidence(Evidence::new("Crate", name.clone()))
                .with_evidence(Evidence::new("Versions", versions.join(", ")));

                if let Some(impact) = impact {
                    issue = issue.with_evidence(Evidence::new("Impact area", impact));
                }

                issue.with_suggestion(Suggestion::new(
                    "Run cargo tree -d to inspect which dependencies keep each version",
                    Some("cargo tree -d"),
                ))
            })
            .collect()
    }

    fn fix(&self, _project: &Project, _issue: &Issue) -> Option<Fix> {
        None
    }
}

struct HeavyCrateRule;

impl Rule for HeavyCrateRule {
    fn id(&self) -> &'static str {
        "heavy-crate"
    }

    fn title(&self) -> &'static str {
        "Heavy direct dependency"
    }

    fn severity(&self) -> Severity {
        Severity::Low
    }

    fn check(&self, project: &Project) -> Vec<Issue> {
        project
            .manifest
            .dependencies
            .iter()
            .filter_map(|dependency| {
                let package = package_name(dependency);
                heavy_crate_reason(package).map(|reason| (dependency, package, reason))
            })
            .map(|(dependency, package, reason)| {
                Issue::new(
                    format!("heavy-crate:{package}"),
                    self.id(),
                    self.severity(),
                    format!("{package} is often expensive to compile"),
                )
                .with_evidence(Evidence::new("Dependency", dependency_path(dependency)))
                .with_evidence(Evidence::new("Reason", reason))
                .with_suggestion(Suggestion::new(
                    "Keep it if it is required; consider limiting features or making it optional if compile time matters",
                    None::<String>,
                ))
            })
            .collect()
    }

    fn fix(&self, _project: &Project, _issue: &Issue) -> Option<Fix> {
        None
    }
}

fn dependency_matches_package(dependency: &Dependency, package: &str) -> bool {
    package_name(dependency) == package
}

fn package_name(dependency: &Dependency) -> &str {
    dependency
        .package
        .as_deref()
        .unwrap_or(dependency.name.as_str())
}

fn dependency_path(dependency: &Dependency) -> String {
    format!(
        "{}.{}",
        dependency.section.manifest_key(),
        dependency_label(dependency)
    )
}

fn dependency_label(dependency: &Dependency) -> String {
    match dependency.package.as_deref() {
        Some(package) => format!("{} (package {package})", dependency.name),
        None => dependency.name.clone(),
    }
}

fn format_features(features: &[String]) -> String {
    if features.is_empty() {
        return "[]".to_owned();
    }

    let quoted = features
        .iter()
        .map(|feature| format!("\"{feature}\""))
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{quoted}]")
}

fn infer_tokio_features(project: &Project) -> Vec<String> {
    let mut features = Vec::<String>::new();

    push_feature_if(
        &mut features,
        "macros",
        project.source_index.contains_token("#[tokio::main")
            || project.source_index.contains_token("#[tokio::test"),
    );
    push_feature_if(
        &mut features,
        "rt",
        project.source_index.contains_token("tokio::spawn")
            || project
                .source_index
                .contains_token("tokio::runtime::Runtime"),
    );
    push_feature_if(
        &mut features,
        "rt-multi-thread",
        project
            .source_index
            .files
            .iter()
            .any(|file| file.path.ends_with("src/main.rs") && file.text.contains("#[tokio::main")),
    );

    for (feature, token) in [
        ("fs", "tokio::fs"),
        ("net", "tokio::net"),
        ("process", "tokio::process"),
        ("signal", "tokio::signal"),
        ("sync", "tokio::sync"),
        ("time", "tokio::time"),
    ] {
        push_feature_if(
            &mut features,
            feature,
            project.source_index.contains_token(token),
        );
    }

    features
}

fn push_feature_if(features: &mut Vec<String>, feature: &str, condition: bool) {
    if condition && !features.iter().any(|existing| existing == feature) {
        features.push(feature.to_owned());
    }
}

fn tokio_suggestion(features: &[String]) -> String {
    if features.is_empty() {
        "Review Tokio usage before replacing full; cargo-shed could not infer a safe minimal feature set".to_owned()
    } else {
        format!(
            "Replace \"full\" with the inferred feature set {} after confirming it covers the project",
            format_features(features)
        )
    }
}

fn reqwest_severity(dependency: &Dependency) -> Severity {
    if dependency.section == DependencySection::Normal {
        Severity::Medium
    } else {
        Severity::Low
    }
}

fn reqwest_suggestion(dependency: &Dependency) -> String {
    let version = dependency.version.as_deref().unwrap_or("<version>");
    let mut features = dependency.features.clone();

    if !features.iter().any(|feature| {
        matches!(
            feature.as_str(),
            "rustls-tls" | "native-tls" | "default-tls"
        )
    }) {
        features.push("rustls-tls".to_owned());
    }

    format!(
        "Consider reqwest = {{ version = \"{version}\", default-features = false, features = {} }} after choosing the intended TLS backend",
        format_features(&features)
    )
}

fn unused_issue_id(dependency: &Dependency) -> String {
    format!("unused-dependency:{}", dependency.name)
}

fn is_unused_allowlisted(crate_name: &str) -> bool {
    matches!(
        crate_name,
        "serde"
            | "serde_json"
            | "tracing"
            | "log"
            | "anyhow"
            | "thiserror"
            | "tokio"
            | "async_trait"
            | "clap"
    )
}

fn unused_suggestion(crate_name: &str, allowlisted: bool) -> String {
    if allowlisted {
        format!(
            "Verify whether {crate_name} is used through macros, reexports, features, or generated code before removing it"
        )
    } else {
        format!(
            "Remove {crate_name} if it is not used by features, macros, reexports, or generated code"
        )
    }
}

fn duplicate_impact(name: &str) -> Option<&'static str> {
    match name {
        "syn" | "quote" | "proc-macro2" => Some("proc-macro stack"),
        "ring" | "rustls" | "openssl" => Some("TLS/network stack"),
        "tokio" | "futures" | "mio" => Some("async stack"),
        "serde" | "serde_json" => Some("serialization stack"),
        _ => None,
    }
}

fn heavy_crate_reason(name: &str) -> Option<&'static str> {
    match name {
        "openssl" => Some("native TLS bindings often add platform-specific build cost"),
        "ring" => Some("cryptography crates can add native build cost"),
        "syn" => Some("proc-macro parsing crates are common compile-time hotspots"),
        "tonic" => Some("gRPC stacks can pull in code generation and async networking"),
        "sqlx" => Some("database stacks often add macros, drivers, and TLS dependencies"),
        "diesel" => Some("database ORM crates often add macros and backend-specific code"),
        "polars" => Some("dataframe crates are large and feature-rich"),
        "bevy" => Some("game engine crates bring a large dependency graph"),
        "gtk" => Some("GUI bindings often add native platform dependencies"),
        "wasmtime" => Some("runtime crates tend to have large dependency graphs"),
        "rocksdb" => Some("database bindings often compile native code"),
        _ => None,
    }
}
