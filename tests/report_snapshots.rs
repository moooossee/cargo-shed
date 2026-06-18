mod fixtures;

use camino::Utf8PathBuf;
use cargo_shed::{Config, Report, analyze};
use fixtures::TestProject;

fn phase_five_project() -> TestProject {
    TestProject::new()
        .cargo_toml(
            r#"
            [package]
            name = "demo"
            version = "0.1.0"
            edition = "2024"

            [dependencies]
            chrono = "0.4"
            openssl = "0.10"
            reqwest = { version = "0.12", features = ["json"] }
            tokio = { version = "1", features = ["full"] }
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
        .src(
            "main.rs",
            r#"
            #[tokio::main]
            async fn main() {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let _client = reqwest::Client::new();
                let _method = openssl::ssl::SslMethod::tls();
            }
            "#,
        )
}

fn analyze_snapshot_project(project: &TestProject) -> Report {
    let mut report = analyze(Config {
        manifest_path: Some(project.path().join("Cargo.toml")),
        ..Config::default()
    })
    .unwrap();
    report.root = Utf8PathBuf::from("[PROJECT]");
    report
}

#[test]
fn human_report_is_stable() {
    let project = phase_five_project();
    let report = analyze_snapshot_project(&project);

    insta::assert_snapshot!("human_report", report.to_human());
}

#[test]
fn json_report_is_stable_and_valid() {
    let project = phase_five_project();
    let report = analyze_snapshot_project(&project);
    let json = report.to_json().unwrap();
    let value = serde_json::from_str::<serde_json::Value>(&json).unwrap();

    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["score"], 51);
    assert_eq!(value["summary"]["total"], 5);

    insta::assert_snapshot!("json_report", json);
}
