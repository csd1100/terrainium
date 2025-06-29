use std::process::Command;
use std::{env, fs};

use anyhow::{Context, Result};
fn write_git_hash() -> Result<()> {
    // Get the git commit hash
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .context("failed to run git command")?;

    let git_hash = String::from_utf8_lossy(&output.stdout);

    // Write to $OUT_DIR/git_hash.txt
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("git_hash.txt");
    fs::write(dest_path, git_hash.trim()).context("failed to write git hash to the file")
}

fn main() -> Result<()> {
    write_git_hash()
}
