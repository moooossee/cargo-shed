# Contributing

Thanks for caring about `cargo-shed`.

Keep changes small, clear, and easy to review. The tool should stay conservative: reporting a maybe-problem is fine, but changing a user's project needs strong evidence.

Before opening a PR, please run:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

If you add a rule, include tests for the quiet path, the reported issue, and any fix behavior.
