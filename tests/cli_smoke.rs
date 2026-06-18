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

#[test]
fn check_exits_zero_without_blocking_issues() {
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
        .arg("--check")
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo-shed check passed"));
}

#[test]
fn check_exits_one_with_blocking_issues() {
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
    let mut command = Command::cargo_bin("cargo-shed").unwrap();

    command
        .arg("shed")
        .arg("--manifest-path")
        .arg(project.path().join("Cargo.toml").as_str())
        .arg("--check")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("cargo-shed check failed"))
        .stdout(predicate::str::contains("unused-dependency:chrono"));
}
