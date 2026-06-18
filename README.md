# cargo-shed

Find what slows your Rust project down.

`cargo-shed` is a small Cargo subcommand for checking the health of a Rust project. It looks at your `Cargo.toml`, your `Cargo.lock`, and the Rust files in `src/` and `tests/`, then points out dependency choices that may be making builds slower or manifests harder to reason about.

It is meant to be useful, calm, and boring in the good way: read files, explain what it found, and avoid surprising changes.

## Install

```sh
cargo install cargo-shed
```

For local development:

```sh
cargo install --path .
```

## Use

```sh
cargo shed
```

You can also run the binary directly:

```sh
cargo-shed shed
```

Check a specific manifest:

```sh
cargo shed --manifest-path ./Cargo.toml
```

Use it in CI:

```sh
cargo shed --check
```

Ask for JSON:

```sh
cargo shed --format json
```

Ask what a rule means:

```sh
cargo shed explain tokio-full
```

Ask for fixes:

```sh
cargo shed --fix
cargo shed --fix tokio-full
```

The fix engine is intentionally conservative. If `cargo-shed` is not confident, it reports the issue and leaves your files alone.

## What it checks

The first version focuses on dependency health:

- broad features like `tokio/full`
- `reqwest` defaults that may be accidental
- dependencies that look unused
- duplicate crate versions in `Cargo.lock`
- crates that are often expensive to compile

Some of those rules are still being wired up. The parser and CLI are already shaped around that contract.

## Safety

`cargo-shed` does not compile your project, run build scripts, or execute Cargo commands while analyzing. It reads:

- `Cargo.toml`
- `Cargo.lock` when it exists
- Rust files under `src/`
- Rust files under `tests/`

When fixes are implemented, file changes must create a backup first.

## Status

This is early. The goal is not to sound clever. The goal is to give you a clear problem, a clear reason, and the exact next move.
