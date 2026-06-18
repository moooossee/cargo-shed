# Changelog

## 0.1.0

- Initial public release.
- Cargo subcommand and direct binary entry points.
- Library API with `analyze` and `apply_fixes`.
- Manifest parser for normal, dev, build, and workspace dependencies.
- Source scanner for `src/` and `tests/`.
- Cargo.lock parser for package data and duplicate-version grouping.
- MVP rules: `tokio-full`, `reqwest-default-features`, `unused-dependency`, `duplicate-versions`, and `heavy-crate`.
- Human and JSON reports with scoring and severity summaries.
- Safe fixes for clear `tokio/full` feature reductions and simple unused dependencies.
- CI-oriented `--check` mode.
