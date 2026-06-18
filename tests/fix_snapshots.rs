mod fixtures;

use std::fs;

use cargo_shed::{Config, apply_fixes};
use fixtures::TestProject;

fn fix_project(project: &TestProject, selected_rule: Option<&str>) -> cargo_shed::FixReport {
    apply_fixes(Config {
        manifest_path: Some(project.path().join("Cargo.toml")),
        fix: true,
        selected_rule: selected_rule.map(ToOwned::to_owned),
        ..Config::default()
    })
    .unwrap()
}

#[test]
fn empty_fix_report_is_human_readable() {
    let report = cargo_shed::FixReport {
        backup_path: None,
        applied: Vec::new(),
        skipped: Vec::new(),
        failed: false,
    };

    assert_eq!(report.to_human(), "No safe fixes are available yet.\n");
}

#[test]
fn fixes_tokio_full_when_features_are_inferred() {
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
    let manifest_path = project.path().join("Cargo.toml");
    let before = fs::read_to_string(&manifest_path).unwrap();

    let report = fix_project(&project, Some("tokio-full"));
    let after = fs::read_to_string(&manifest_path).unwrap();
    let backup = fs::read_to_string(report.backup_path.unwrap()).unwrap();

    assert!(!report.failed);
    assert_eq!(backup, before);
    assert_ne!(after, before);
    assert!(after.contains(r#"features = ["macros", "rt-multi-thread", "time"]"#));
}

#[test]
fn removes_simple_unused_dependency() {
    let project = TestProject::new()
        .cargo_toml(
            r#"
            [package]
            name = "demo"
            version = "0.1.0"
            edition = "2024"

            [dependencies]
            chrono = "0.4"
            serde = "1"
            "#,
        )
        .src("main.rs", "fn main() {}");
    let manifest_path = project.path().join("Cargo.toml");
    let before = fs::read_to_string(&manifest_path).unwrap();

    let report = fix_project(&project, None);
    let after = fs::read_to_string(&manifest_path).unwrap();
    let backup = fs::read_to_string(report.backup_path.unwrap()).unwrap();

    assert!(!report.failed);
    assert_eq!(backup, before);
    assert!(!after.contains(r#"chrono = "0.4""#));
    assert!(after.contains(r#"serde = "1""#));
}

#[test]
fn skips_tokio_full_when_usage_is_unknown() {
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
                let _stdin = tokio::io::stdin();
            }
            "#,
        );
    let manifest_path = project.path().join("Cargo.toml");
    let before = fs::read_to_string(&manifest_path).unwrap();

    let report = fix_project(&project, Some("tokio-full"));
    let after = fs::read_to_string(&manifest_path).unwrap();

    assert!(report.failed);
    assert!(report.backup_path.is_none());
    assert_eq!(after, before);
    assert_eq!(report.skipped, vec!["tokio-full has no safe automatic fix"]);
}
