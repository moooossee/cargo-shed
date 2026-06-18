use std::fs;

use assert_fs::TempDir;
use assert_fs::prelude::*;

pub struct TestProject {
    dir: TempDir,
}

impl Default for TestProject {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl TestProject {
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
        }
    }

    pub fn cargo_toml(self, text: &str) -> Self {
        self.dir.child("Cargo.toml").write_str(text).unwrap();
        self
    }

    pub fn cargo_lock(self, text: &str) -> Self {
        self.dir.child("Cargo.lock").write_str(text).unwrap();
        self
    }

    pub fn src(self, name: &str, text: &str) -> Self {
        self.dir.child("src").create_dir_all().unwrap();
        self.dir.child("src").child(name).write_str(text).unwrap();
        self
    }

    pub fn test(self, name: &str, text: &str) -> Self {
        self.dir.child("tests").create_dir_all().unwrap();
        self.dir.child("tests").child(name).write_str(text).unwrap();
        self
    }

    pub fn path(&self) -> camino::Utf8PathBuf {
        camino::Utf8PathBuf::from_path_buf(fs::canonicalize(self.dir.path()).unwrap()).unwrap()
    }
}
