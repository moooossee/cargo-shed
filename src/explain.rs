use crate::ShedError;

pub fn rule(rule_id: &str) -> Result<&'static str, ShedError> {
    match rule_id {
        "tokio-full" => Ok(
            "tokio-full\n\nDetects tokio dependencies that enable the full feature set. cargo-shed reports this because full often pulls in more compile-time cost than the project actually needs. The safe fix replaces full only when source usage gives enough evidence for a smaller feature set.",
        ),
        "reqwest-default-features" => Ok(
            "reqwest-default-features\n\nDetects reqwest dependencies that keep default features enabled. This can be totally fine, but it often means TLS and platform behavior were accepted by accident. cargo-shed reports it carefully and does not auto-fix it by default.",
        ),
        "unused-dependency" => Ok(
            "unused-dependency\n\nDetects declared dependencies that do not appear in scanned Rust source files. This is heuristic, so cargo-shed only removes simple non-optional normal and dev dependencies when the source scan is unambiguous.",
        ),
        "duplicate-versions" => Ok(
            "duplicate-versions\n\nDetects multiple versions of the same crate in Cargo.lock. These duplicates can increase compile time and binary size, especially around proc macro, TLS, async, and serialization crates.",
        ),
        "heavy-crate" => Ok(
            "heavy-crate\n\nDetects dependencies that are often expensive to compile. This is not a judgment that the crate is bad. It is a small nudge to check whether the cost is intentional.",
        ),
        _ => Err(ShedError::UnknownRule {
            rule_id: rule_id.to_owned(),
        }),
    }
}
