//! Hands the file to the user's editor and waits for them to finish.

use crate::which::on_path;
use std::env;
use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::process::Command;

/// Editors to try when neither `$VISUAL` nor `$EDITOR` is set, best first.
const FALLBACKS: &[&str] = &["nvim", "vim", "nano", "vi", "code", "notepad"];

/// Opens `path` in the user's editor, blocking until it exits.
pub fn open(path: &Path) -> io::Result<()> {
    let (program, mut args) = command();

    // VS Code forks and returns instantly; every terminal editor blocks.
    // Without --wait the PDF gets built from the file as it was before the
    // user typed anything.
    if is_vscode(&program) && !args.iter().any(|arg| arg == "-w" || arg == "--wait") {
        args.push("--wait".to_string());
    }

    let status = Command::new(&program)
        .args(&args)
        .arg(path)
        .status()
        .map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "could not start editor '{program}': {e}\nSet one with: export EDITOR=nvim"
                ),
            )
        })?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "editor '{program}' exited with an error"
        )));
    }
    Ok(())
}

/// The editor to run, split into a program and its arguments.
///
/// `$EDITOR` legitimately carries flags — `code -w`, `emacsclient -nw` — so
/// the value is split on whitespace. Passing it whole to `Command::new` makes
/// the OS hunt for a program file literally named "code -w".
///
/// `$VISUAL` wins over `$EDITOR` by long-standing convention: it names the
/// full-screen editor, which is what we want for a file someone has to fill in.
fn command() -> (String, Vec<String>) {
    for variable in ["VISUAL", "EDITOR"] {
        let Some(value) = env::var_os(variable).and_then(|v| v.into_string().ok()) else {
            continue;
        };
        let mut words = value.split_whitespace().map(String::from);
        if let Some(program) = words.next() {
            return (program, words.collect());
        }
    }

    let choice = FALLBACKS
        .iter()
        .find(|program| on_path(program))
        .unwrap_or(&"vi");
    (choice.to_string(), Vec::new())
}

/// Matches the program name only, so `/usr/local/bin/code` counts while an
/// editor living in `~/code/bin/nano` does not. The old check was
/// `editor.contains("code")`, which got that backwards.
fn is_vscode(program: &str) -> bool {
    let name = Path::new(program)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(program);
    matches!(name, "code" | "codium" | "code-insiders" | "cursor")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vscode_is_matched_by_name_not_by_substring() {
        assert!(is_vscode("code"));
        assert!(is_vscode("/usr/local/bin/code"));
        assert!(is_vscode("code.exe"));
        assert!(is_vscode("cursor"));

        // The bug this replaced: any path containing "code" matched.
        assert!(!is_vscode("/home/code-user/bin/nano"));
        assert!(!is_vscode("nvim"));
    }
}
