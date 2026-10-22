//! Compiles LaTeX to PDF by shelling out to tectonic.
//!
//! tectonic is the one runtime dependency txt2tex cannot ship in the binary:
//! it fetches the TeX packages it needs on first run and is ~100 MB installed.

use crate::which::on_path;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

const MISSING: &str = "\
tectonic is not installed. txt2tex needs it to typeset the PDF.

  macOS    brew install tectonic
  Linux    cargo install tectonic
  Windows  choco install tectonic

  Or see  https://tectonic-typesetting.github.io/en-US/install.html";

pub fn is_installed() -> bool {
    on_path("tectonic")
}

/// Compiles `tex` and writes the PDF to `output`.
pub fn compile(tex: &str, output: &Path) -> Result<(), Box<dyn Error>> {
    if !is_installed() {
        return Err(MISSING.into());
    }

    // Compile in a temp dir so the .aux/.log litter never touches the user's
    // folder; it is removed when `workspace` drops.
    let workspace = tempfile::tempdir()?;
    let source = workspace.path().join("resume.tex");
    fs::write(&source, tex)?;

    let result = Command::new("tectonic")
        .arg("--outdir")
        .arg(workspace.path())
        .arg(&source)
        .output()?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!(
            "tectonic could not typeset the document.\n\n{}",
            diagnose(&stderr)
        )
        .into());
    }

    let built = workspace.path().join("resume.pdf");
    if !built.exists() {
        return Err("tectonic reported success but produced no PDF".into());
    }

    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    // Copy rather than rename: the temp dir is usually on another filesystem,
    // where rename fails with a cross-device error.
    fs::copy(&built, output)?;
    Ok(())
}

/// Picks the useful lines out of tectonic's very chatty stderr.
///
/// A failure buries two or three real messages under a page of package noise,
/// and the real ones are what the user needs to fix their resume.
fn diagnose(stderr: &str) -> String {
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with('!') || line.contains("error:")
        })
        .collect();

    if !errors.is_empty() {
        return errors.join("\n");
    }

    // Nothing matched, so show the tail rather than nothing at all.
    let tail: Vec<&str> = stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    tail[tail.len().saturating_sub(10)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnose_keeps_the_real_errors() {
        let stderr = "\
note: connecting to https://example.com
note: downloading foo.sty
! Undefined control sequence.
note: more chatter
error: something went wrong";
        let out = diagnose(stderr);
        assert!(out.contains("! Undefined control sequence."));
        assert!(out.contains("error: something went wrong"));
        assert!(!out.contains("chatter"));
    }

    #[test]
    fn diagnose_falls_back_to_the_tail() {
        let stderr = "note: one\nnote: two";
        assert_eq!(diagnose(stderr), "note: one\nnote: two");
    }
}
