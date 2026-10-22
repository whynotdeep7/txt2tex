//! txt2tex — plain-text resumes into typeset PDFs.
//!
//! The pipeline runs one way, and each stage is a plain function with no
//! shared state, which is why the whole thing is testable without a terminal
//! or a LaTeX install:
//!
//! ```text
//!   resume.txt ──parser──▶ Resume ──template──▶ LaTeX ──pdf──▶ resume.pdf
//!                            │  ▲
//!                   analyzer │  │ writer
//!                            ▼  │
//!                     Vec<Finding>, resume.txt
//! ```

pub mod analyzer;
pub mod editor;
pub mod latex;
pub mod model;
pub mod open;
pub mod parser;
pub mod pdf;
pub mod report;
pub mod template;
pub mod tui;
pub mod which;
pub mod writer;

/// The starter resume that `txt2tex new` writes.
///
/// Baked into the binary at compile time, so there is no templates/ directory
/// to install or find at runtime.
pub const TEMPLATE: &str = include_str!("../templates/jake.txt");
