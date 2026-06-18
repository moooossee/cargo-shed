mod fixtures;

use assert_cmd::Command;
use predicates::prelude::*;

use fixtures::TestProject;

#[test]
fn prints_help_for_cargo_style_invocation() {
    let mut command = Command::cargo_bin("cargo-shed").unwrap();

    command
        .arg("shed")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Find what slows your Rust project down",
        ));
}

#[test]
fn prints_json_report() {
    let project = TestProject::new()
        .cargo_toml(
            r#"
            [package]
            name = "demo"
            version = "0.1.0"
            edition = "2024"
            "#,
        )
        .src("main.rs", "fn main() {}");
    let mut command = Command::cargo_bin("cargo-shed").unwrap();

    command
        .arg("shed")
        .arg("--manifest-path")
        .arg(project.path().join("Cargo.toml").as_str())
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"score\""));
}
