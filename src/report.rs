//! Terminal output for the command line.
//!
//! Progress and errors go to stderr, results go to stdout. That split is what
//! lets `txt2tex build --tex > resume.tex` capture the document and nothing
//! else.

use crate::analyzer::{counts, score, Finding, Severity};
use crate::model::Section;
use colored::Colorize;

pub fn step(message: &str) {
    eprintln!("  {} {}", "▸".cyan(), message);
}

pub fn done(message: &str) {
    eprintln!("  {} {}", "✓".green().bold(), message);
}

pub fn hint(message: &str) {
    eprintln!("  {} {}", "→".yellow(), message.dimmed());
}

/// The full analysis, grouped by section with issues before tips, so the list
/// reads top to bottom in the order you would work through it.
pub fn analysis(findings: &[Finding]) {
    println!();

    if findings.is_empty() {
        println!(
            "  {}  Nothing to fix — this one is in good shape.\n",
            "✓".green().bold()
        );
        return;
    }

    for section in Section::ALL {
        let mut here: Vec<&Finding> = findings.iter().filter(|f| f.section == section).collect();
        if here.is_empty() {
            continue;
        }
        here.sort_by_key(|f| f.severity == Severity::Tip);

        println!("  {}", section.title().bold());
        for finding in here {
            match finding.severity {
                Severity::Issue => println!("    {}  {}", "✗".red().bold(), finding.message),
                Severity::Tip => println!("    {}  {}", "→".yellow(), finding.message.dimmed()),
            }
        }
        println!();
    }

    let (issues, tips) = counts(findings);
    println!(
        "  {}   {}\n",
        format!("score {}/100", score(findings)).bold(),
        summary(issues, tips).dimmed()
    );
}

/// "2 issues, 1 suggestion" — shared by the analysis footer and the build hint.
pub fn summary(issues: usize, tips: usize) -> String {
    let mut parts = Vec::new();
    if issues > 0 {
        parts.push(format!("{issues} {}", plural(issues, "issue")));
    }
    if tips > 0 {
        parts.push(format!("{tips} {}", plural(tips, "suggestion")));
    }
    if parts.is_empty() {
        "all clear".to_string()
    } else {
        parts.join(", ")
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        word.to_string()
    } else {
        format!("{word}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_pluralises_and_omits_empty_halves() {
        assert_eq!(summary(1, 0), "1 issue");
        assert_eq!(summary(2, 1), "2 issues, 1 suggestion");
        assert_eq!(summary(0, 3), "3 suggestions");
        assert_eq!(summary(0, 0), "all clear");
    }
}
