//! Resume quality checks.
//!
//! Produces findings as data rather than printing them. The CLI and the TUI
//! then render the same results in their own shapes — the rules live in one
//! place and cannot drift apart.

use crate::model::{Resume, Section};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Something is missing or wrong. Fix these first.
    Issue,
    /// Advice. The resume works without it, but it would be stronger with it.
    Tip,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    /// Which section to jump to — the TUI turns this into navigation.
    pub section: Section,
    pub message: String,
}

pub fn analyze(resume: &Resume) -> Vec<Finding> {
    let mut found = Findings::default();
    personal(&mut found, resume);
    education(&mut found, resume);
    experience(&mut found, resume);
    projects(&mut found, resume);
    skills(&mut found, resume);
    found.0
}

/// Issues and tips, in that order.
pub fn counts(findings: &[Finding]) -> (usize, usize) {
    let issues = findings
        .iter()
        .filter(|f| f.severity == Severity::Issue)
        .count();
    (issues, findings.len() - issues)
}

/// 100, less 10 per issue and 3 per tip. A rough signal to steer by, not a grade.
pub fn score(findings: &[Finding]) -> u8 {
    let penalty: u32 = findings
        .iter()
        .map(|f| match f.severity {
            Severity::Issue => 10,
            Severity::Tip => 3,
        })
        .sum();
    100u32.saturating_sub(penalty) as u8
}

#[derive(Default)]
struct Findings(Vec<Finding>);

impl Findings {
    fn issue(&mut self, section: Section, message: impl Into<String>) {
        self.0.push(Finding {
            severity: Severity::Issue,
            section,
            message: message.into(),
        });
    }

    fn tip(&mut self, section: Section, message: impl Into<String>) {
        self.0.push(Finding {
            severity: Severity::Tip,
            section,
            message: message.into(),
        });
    }
}

fn personal(found: &mut Findings, resume: &Resume) {
    if resume.name.trim().is_empty() {
        found.issue(Section::Personal, "Missing name");
    }
    if resume.email.trim().is_empty() {
        found.issue(Section::Personal, "Missing email address");
    } else if !resume.email.contains('@') {
        found.issue(
            Section::Personal,
            format!("'{}' is not an email address", resume.email),
        );
    }
    if resume.phone.trim().is_empty() {
        found.issue(Section::Personal, "Missing phone number");
    }
    if resume.github.trim().is_empty() {
        found.tip(Section::Personal, "Add your GitHub — recruiters do look");
    }
}

fn education(found: &mut Findings, resume: &Resume) {
    if resume.degree.trim().is_empty() {
        found.issue(Section::Education, "Missing degree");
    }
    if resume.university.trim().is_empty() {
        found.issue(Section::Education, "Missing university");
    }
    if resume.year.trim().is_empty() {
        found.tip(Section::Education, "Add the years you attended");
    }
}

fn experience(found: &mut Findings, resume: &Resume) {
    if resume.experience.is_empty() {
        found.tip(
            Section::Experience,
            "No experience yet — internships, open source and part-time work all count",
        );
        return;
    }

    for job in &resume.experience {
        let who = label(&job.company);
        if job.details.is_empty() {
            found.issue(
                Section::Experience,
                format!("{who} has no bullet points — say what you did"),
            );
        }
        for detail in &job.details {
            // A bullet with a number in it describes impact; one without
            // usually describes a job description.
            if !detail.chars().any(|c| c.is_ascii_digit()) {
                found.tip(
                    Section::Experience,
                    format!("Add a number to \"{}\"", truncate(detail, 40)),
                );
            }
        }
    }
}

fn projects(found: &mut Findings, resume: &Resume) {
    if resume.projects.is_empty() {
        found.issue(Section::Projects, "No projects — add at least two");
        return;
    }

    for project in &resume.projects {
        let who = label(&project.title);
        let bullets = project.bullets();
        if bullets.is_empty() {
            found.issue(Section::Projects, format!("{who} has no description"));
        } else if bullets.iter().map(|b| b.len()).sum::<usize>() < 30 {
            found.tip(
                Section::Projects,
                format!("{who} is described very briefly — add detail"),
            );
        }
        if project.tech.is_empty() {
            found.issue(Section::Projects, format!("{who} lists no tech stack"));
        }
    }
}

fn skills(found: &mut Findings, resume: &Resume) {
    match resume.skills.len() {
        0 => found.issue(Section::Skills, "No skills listed"),
        1 => found.tip(
            Section::Skills,
            "Add more categories (Frameworks, Tools, Libraries)",
        ),
        _ => {}
    }
}

/// Names an entry for a message, falling back when the user has not titled it.
fn label(name: &str) -> String {
    match name.trim() {
        "" => "An untitled entry".to_string(),
        name => format!("'{name}'"),
    }
}

/// Truncates to `max_chars` characters.
///
/// Counts characters, not bytes: `&text[..40]` panics outright the moment a
/// resume contains an em-dash or an accent near the cut.
fn truncate(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((byte, _)) => format!("{}…", text[..byte].trim_end()),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Experience;

    /// The regression: this panicked on any multi-byte character near byte 40.
    #[test]
    fn truncate_splits_on_character_boundaries() {
        let text = format!("{} — trailing words", "a".repeat(38));
        assert!(truncate(&text, 40).ends_with('…'));
        assert_eq!(
            truncate("Zürich — a long line about work here", 10),
            "Zürich — a…"
        );
    }

    #[test]
    fn truncate_leaves_short_text_alone() {
        assert_eq!(truncate("short", 40), "short");
    }

    #[test]
    fn analyzing_a_resume_with_accents_does_not_panic() {
        let resume = Resume {
            experience: vec![Experience {
                company: "Zürich Tech".into(),
                details: vec![format!("{} — no numbers in this bullet", "a".repeat(38))],
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(!analyze(&resume).is_empty());
    }

    #[test]
    fn bullets_with_numbers_draw_no_tip() {
        let with_metric = Resume {
            experience: vec![Experience {
                company: "Acme".into(),
                details: vec!["Served 10k requests per day".into()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let tips: Vec<_> = analyze(&with_metric)
            .into_iter()
            .filter(|f| f.section == Section::Experience)
            .collect();
        assert!(tips.is_empty(), "unexpected: {tips:?}");
    }

    #[test]
    fn score_falls_as_findings_accumulate() {
        assert_eq!(score(&[]), 100);
        let perfect = Resume {
            name: "Alex".into(),
            email: "a@b.com".into(),
            ..Default::default()
        };
        assert!(score(&analyze(&perfect)) < 100);
    }

    #[test]
    fn a_malformed_email_is_reported() {
        let resume = Resume {
            email: "not-an-email".into(),
            ..Default::default()
        };
        assert!(analyze(&resume)
            .iter()
            .any(|f| f.message.contains("is not an email")));
    }
}
