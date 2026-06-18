mod fixtures;

use cargo_shed::Project;
use fixtures::TestProject;

#[test]
fn loads_manifest_dependencies() {
    let project = TestProject::new()
        .cargo_toml(
            r#"
            [package]
            name = "demo"
            version = "0.1.0"
            edition = "2024"

            [dependencies]
            tokio = { version = "1", features = ["full"] }

            [dev-dependencies]
            assert_cmd = "2"
            "#,
        )
        .src("main.rs", "fn main() {}");

    let loaded = Project::load(Some(&project.path().join("Cargo.toml"))).unwrap();

    assert_eq!(loaded.manifest.dependencies.len(), 2);
    assert_eq!(loaded.manifest.package_name(), Some("demo"));
}
