//! Build script for runa
//!
//! Embeds build details into the `rn` binary for `rn -V` and `rn -vV`:
//! - git commit
//! - rustc version
//! - target
//! - profile (release, debug)

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Ask git only in runa checkouts
    if Path::new(".git").exists()
        && let Some(git) = run("git", &["log", "-1", "--date=short", "--format=%h %cd"])
    {
        println!("cargo:rustc-env=RUNA_GIT_INFO={git}");
        println!("cargo:rerun-if-changed=.git/HEAD");
        println!("cargo:rerun-if-changed=.git/refs/heads");
    }

    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    if let Some(version) = run(&rustc, &["--version"])
        && let Some(number) = version.split_whitespace().nth(1)
    {
        println!("cargo:rustc-env=RUNA_RUSTC={number}");
    }

    println!(
        "cargo:rustc-env=RUNA_TARGET={}",
        env::var("TARGET").unwrap_or_default()
    );

    println!("cargo:rerun-if-changed=build.rs");
}

/// Run a command and return its output in a trimmed format, or None on failure.
fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;

    if !out.status.success() {
        return None;
    }

    let text = String::from_utf8(out.stdout).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}
