//! Handing a finished file to the system's default application.

use std::path::Path;
use std::process::Command;

/// Opens `path` in whatever the system uses for its file type.
///
/// Best effort on purpose: the file is already written, so a viewer that will
/// not start is not worth failing a build over. `cfg!` rather than `#[cfg]`
/// keeps all three branches compiling on every platform.
pub fn in_viewer(path: &Path) {
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(path);
        command
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", ""]).arg(path);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };
    let _ = command.spawn();
}
