mod fixtures;

use cargo_shed::{Config, Report, Severity, analyze};
use fixtures::TestProject;

fn analyze_project(project: &TestProject) -> Report {
    analyze(Config {
        manifest_path: Some(project.path().join("Cargo.toml")),
        ..Config::default()
    })
    .unwrap()
}

#[test]
fn registry_contains_mvp_rules() {
    let rule_ids = cargo_shed::rules::registry()
        .into_iter()
        .map(|rule| rule.id())
        .collect::<Vec<_>>();

    assert_eq!(
        rule_ids,
        vec![
            "tokio-full",
            "reqwest-default-features",
            "unused-dependency",
            "duplicate-versions",
            "heavy-crate",
        ]
    );
}

#[test]
fn tokio_full_reports_inferred_features() {
    let project = TestProject::new()
        .cargo_toml(
            r#"
            [package]
            name = "demo"
            version = "0.1.0"
            edition = "2024"

            [dependencies]
            tokio = { version = "1", features = ["full"] }
            "#,
        )
        .src(
            "main.rs",
            r#"
            #[tokio::main]
            async fn main() {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            "#,
        );

    let report = analyze_project(&project);
    let issue = report
        .issues
        .iter()
        .find(|issue| issue.rule_id == "tokio-full")
        .unwrap();

    assert_eq!(issue.severity, Severity::High);
    assert!(
        issue
            .evidence
            .iter()
            .any(|evidence| evidence.value.contains("rt-multi-thread"))
    );
    assert!(
        issue
            .evidence
            .iter()
            .any(|evidence| evidence.value.contains("time"))
    );
}

#[test]
fn reqwest_default_features_reports_implicit_defaults() {
    let project = TestProject::new()
        .cargo_toml(
            r#"
            [package]
            name = "demo"
            version = "0.1.0"
            edition = "2024"

            [dependencies]
            reqwest = { version = "0.12", features = ["json"] }
            "#,
        )
        .src(
            "main.rs",
            r#"
            fn main() {
                let _client = reqwest::Client::new();
            }
            "#,
        );

    let report = analyze_project(&project);
    let issue = report
        .issues
        .iter()
        .find(|issue| issue.rule_id == "reqwest-default-features")
        .unwrap();

    assert_eq!(issue.severity, Severity::Medium);
    assert!(
        issue
            .evidence
            .iter()
            .any(|evidence| evidence.value == "implicit true")
    );
}

#[test]
fn unused_dependency_reports_missing_source_usage() {
    let project = TestProject::new()
        .cargo_toml(
            r#"
            [package]
            name = "demo"
            version = "0.1.0"
            edition = "2024"

            [dependencies]
            chrono = "0.4"
            "#,
        )
        .src("main.rs", "fn main() {}");

    let report = analyze_project(&project);
    let issue = report
        .issues
        .iter()
        .find(|issue| issue.id == "unused-dependency:chrono")
        .unwrap();

    assert_eq!(issue.rule_id, "unused-dependency");
    assert_eq!(issue.severity, Severity::Medium);
}

#[test]
fn duplicate_versions_reports_lockfile_duplicates() {
    let project = TestProject::new()
        .cargo_toml(
            r#"
            [package]
            name = "demo"
            version = "0.1.0"
            edition = "2024"
            "#,
        )
        .cargo_lock(
            r#"
            version = 4

            [[package]]
            name = "syn"
            version = "1.0.109"

            [[package]]
            name = "syn"
            version = "2.0.106"
            "#,
        )
        .src("main.rs", "fn main() {}");

    let report = analyze_project(&project);
    let issue = report
        .issues
        .iter()
        .find(|issue| issue.id == "duplicate-versions:syn")
        .unwrap();

    assert_eq!(issue.severity, Severity::High);
    assert!(
        issue
            .evidence
            .iter()
            .any(|evidence| evidence.value.contains("proc-macro stack"))
    );
}

#[test]
fn heavy_crate_reports_direct_heavy_dependency() {
    let project = TestProject::new()
        .cargo_toml(
            r#"
            [package]
            name = "demo"
            version = "0.1.0"
            edition = "2024"

            [dependencies]
            openssl = "0.10"
            "#,
        )
        .src(
            "main.rs",
            r#"
            fn main() {
                let _method = openssl::ssl::SslMethod::tls();
            }
            "#,
        );

    let report = analyze_project(&project);
    let issue = report
        .issues
        .iter()
        .find(|issue| issue.id == "heavy-crate:openssl")
        .unwrap();

    assert_eq!(issue.rule_id, "heavy-crate");
    assert_eq!(issue.severity, Severity::Low);
}
