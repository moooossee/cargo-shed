use std::process::ExitCode;

fn main() -> ExitCode {
    match cargo_shed::cli::run_from_env() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("cargo-shed failed: {error}");
            ExitCode::from(2)
        }
    }
}
