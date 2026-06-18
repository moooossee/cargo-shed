# cargo-shed

Find what slows your Rust project down.

`cargo-shed` is a Cargo subcommand and Rust library for static dependency health checks. It reads `Cargo.toml`, `Cargo.lock`, and Rust files under `src/` and `tests/`, then reports dependency choices that may increase compile time, binary size, or manifest maintenance cost.

It does not compile the project, run build scripts, or execute Cargo commands while analyzing.

## Install

```sh
cargo install cargo-shed
```

For local development:

```sh
cargo install --path .
```

## Quickstart

Run the Cargo subcommand:

```sh
cargo shed
```

Run the binary directly:

```sh
cargo-shed shed
```

Analyze a specific manifest:

```sh
cargo shed --manifest-path ./Cargo.toml
```

Ask for JSON:

```sh
cargo shed --format json
```

Explain a rule:

```sh
cargo shed explain tokio-full
```

## Example Output

```text
cargo-shed report

Project: /path/to/project
Score: 77/100
Issues: 1 high, 1 medium

Problems found:

[HIGH] tokio-full
Reason: tokio uses the full feature set
Evidence:
- Dependency: dependencies.tokio
- Current features: ["full"]
- Inferred features: ["macros", "rt-multi-thread", "time"]
Suggested: Replace "full" with the inferred feature set ["macros", "rt-multi-thread", "time"] after confirming it covers the project
Run: cargo shed --fix tokio-full
```

## Safe Fixes

`cargo shed --fix` applies only high-confidence edits. Every write creates a backup before changing the manifest:

```text
Cargo.toml.shed.bak
```

Available MVP fixes:

- replace `tokio/full` with inferred features when source usage is clear
- remove a simple unused normal or dev dependency when the scan is unambiguous

Target a specific rule or issue:

```sh
cargo shed --fix tokio-full
cargo shed --fix unused-dependency:chrono
```

If `cargo-shed` cannot prove a fix is safe, it reports the issue and leaves `Cargo.toml` unchanged.

## CI

Use `--check` in GitHub Actions or another CI system:

```sh
cargo shed --check
```

Exit codes:

- `0`: no high or medium severity issues
- `1`: at least one high or medium severity issue
- `2`: runtime, IO, or parse error

Example workflow step:

```yaml
- run: cargo install cargo-shed
- run: cargo shed --check
```

The human `--check` output is intentionally short. Use `cargo shed --format json` for machine-readable reporting.

## Rules

| Rule | Severity | Auto-fix |
| --- | --- | --- |
| `tokio-full` | HIGH | Yes, when inferred safely |
| `reqwest-default-features` | LOW/MEDIUM | No by default |
| `unused-dependency` | MEDIUM | Yes, only obvious cases |
| `duplicate-versions` | MEDIUM/HIGH | No |
| `heavy-crate` | LOW | No |

## Library Use

```rust,no_run
use cargo_shed::{Config, analyze};

let report = analyze(Config {
    manifest_path: Some("Cargo.toml".into()),
    ..Config::default()
})?;

println!("{}", report.to_human());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Known Limitations

`cargo-shed` uses static heuristics. It does not resolve macros, run rustc, evaluate cfg expressions, inspect generated code, or fully model complex workspaces.

The unused dependency rule can miss usage through reexports, proc macros, feature-only dependencies, generated files, or unusual include patterns. For that reason, auto-fix is intentionally narrower than reporting.

The Tokio feature fix understands common usage patterns. Unknown `tokio::` modules make the fix unavailable until the project can be reviewed manually.

## Roadmap

- richer workspace support
- configurable allow and deny lists
- interactive fix mode
- SARIF output for code scanning
- baseline support for gradual CI adoption
- more ecosystem-specific dependency rules

## Safety Model

The guiding contract is:

```text
problem -> reason -> exact change -> command
```

The tool should be useful without being surprising. It treats `Cargo.toml` as a user-owned document, creates backups before writes, and prefers skipping a fix over applying an edit with weak evidence.
