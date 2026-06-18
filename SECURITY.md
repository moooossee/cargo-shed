# Security

`cargo-shed` should be safe to run on an untrusted Rust project because analysis is file-based.

It must not:

- run build scripts
- compile the target project
- execute project-local Cargo configuration
- apply fixes without a backup

Please report security issues privately through the repository security advisory flow when available.
