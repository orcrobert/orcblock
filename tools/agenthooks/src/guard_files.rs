use std::path::Path;

use crate::hook::{Env, HookInput, Outcome};

const GENERATED: &[(&str, &str)] = &[
    (
        "Cargo.lock",
        "Cargo.lock is generated. Change a Cargo.toml, then run cargo build.",
    ),
    (
        "package-lock.json",
        "package-lock.json is generated. Change app/package.json, then run npm install (ask first: new packages need an ADR).",
    ),
];

pub(crate) fn run(input: &HookInput, env: &Env<'_>) -> Outcome {
    let path = &input.tool_input.file_path;
    if path.is_empty() {
        return Outcome::Allow;
    }
    let rel = env.rel_path(path);
    let base = rel.file_name().and_then(|n| n.to_str()).unwrap_or_default();

    if (base == ".env" || base.starts_with(".env.")) && base != ".env.example" {
        return Outcome::Block(
            "Never read or edit .env files: they hold real secrets. Document new variables in .env.example instead.".into(),
        );
    }
    if let Some((_, reason)) = GENERATED.iter().find(|(name, _)| *name == base) {
        return Outcome::Block((*reason).into());
    }
    if rel.is_absolute() && is_system_path(&rel) {
        return Outcome::Block(format!(
            "{} is a system path. Only the installed daemon changes the system; tests and dev mode use fakes and ./data.",
            rel.display()
        ));
    }
    Outcome::Allow
}

fn is_system_path(path: &Path) -> bool {
    [
        "/etc",
        "/private/etc",
        "/Library",
        "/System",
        "/usr",
        "/var",
        "/private/var",
    ]
    .iter()
    .any(|root| path.starts_with(root))
}

#[cfg(test)]
mod tests;
