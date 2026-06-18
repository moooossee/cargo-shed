mod fixtures;

use cargo_shed::Project;
use fixtures::TestProject;

#[test]
fn loads_duplicate_lockfile_versions() {
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

    let loaded = Project::load(Some(&project.path().join("Cargo.toml"))).unwrap();
    let duplicates = loaded.lockfile.unwrap().duplicate_versions();

    assert_eq!(
        duplicates.get("syn"),
        Some(&vec!["1.0.109".to_owned(), "2.0.106".to_owned()])
    );
}
