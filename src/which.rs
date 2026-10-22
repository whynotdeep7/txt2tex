//! Minimal `which`: does this program exist on `PATH`?
//!
//! Used in place of probing with `program --version`. That trick misfires on
//! exactly the programs we care about: `vi` has no such flag and seizes the
//! terminal, and `notepad --version` opens a window.

use std::env;
use std::path::Path;

#[cfg(windows)]
const EXTENSIONS: &[&str] = &["exe", "cmd", "bat"];
#[cfg(not(windows))]
const EXTENSIONS: &[&str] = &[];

pub fn on_path(program: &str) -> bool {
    // An explicit path (./editor, /usr/bin/vim) is not a PATH lookup at all.
    if program.contains(std::path::MAIN_SEPARATOR) {
        return Path::new(program).is_file();
    }

    let Some(path) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path).any(|dir| {
        let candidate = dir.join(program);
        candidate.is_file()
            || EXTENSIONS
                .iter()
                .any(|ext| candidate.with_extension(ext).is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_program_that_is_always_present() {
        let ubiquitous = if cfg!(windows) { "cmd" } else { "sh" };
        assert!(on_path(ubiquitous));
    }

    #[test]
    fn rejects_a_program_that_does_not_exist() {
        assert!(!on_path("txt2tex-definitely-not-a-real-program"));
    }
}
