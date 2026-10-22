//! Serializes a [`Resume`] back into the text format [`crate::parser`] reads.
//!
//! The inverse of the parser, which is what lets the TUI edit the model and
//! save it back without ever losing the hand-written file's shape.
//! `parse_content(&to_text(&r)) == r` — see the round-trip test.

use crate::model::Resume;
use std::fmt::Write;

pub fn to_text(resume: &Resume) -> String {
    let mut out = String::new();

    out.push_str("# PERSONAL INFORMATION\n");
    field(&mut out, "name", &resume.name);
    field(&mut out, "email", &resume.email);
    field(&mut out, "phone", &resume.phone);
    field(&mut out, "github", &resume.github);

    out.push_str("\n# EDUCATION\n");
    field(&mut out, "degree", &resume.degree);
    field(&mut out, "university", &resume.university);
    field(&mut out, "year", &resume.year);

    if !resume.experience.is_empty() {
        out.push_str("\n# EXPERIENCE\n");
        for job in &resume.experience {
            out.push_str("\nexperience:\n");
            field(&mut out, "company", &job.company);
            field(&mut out, "role", &job.role);
            field(&mut out, "duration", &job.duration);
            for bullet in &job.details {
                let _ = writeln!(out, "- {bullet}");
            }
        }
    }

    if !resume.projects.is_empty() {
        out.push_str("\n# PROJECTS\n");
        for project in &resume.projects {
            out.push_str("\nproject:\n");
            field(&mut out, "title", &project.title);
            // Only written when set: an empty `description:` would be noise in
            // a file whose bullets are explicit.
            if !project.description.is_empty() {
                field(&mut out, "description", &project.description);
            }
            field(&mut out, "tech", &project.tech.join(", "));
            for bullet in &project.details {
                let _ = writeln!(out, "- {bullet}");
            }
        }
    }

    if !resume.skills.is_empty() {
        out.push_str("\n# SKILLS\n");
        for group in &resume.skills {
            let _ = writeln!(out, "{}: {}", group.category, group.items.join(", "));
        }
    }

    out
}

/// Writes `key: value`, keeping the key even when the value is empty so the
/// file still shows what can be filled in.
fn field(out: &mut String, key: &str, value: &str) {
    let _ = writeln!(out, "{key}: {value}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Experience, Project, SkillGroup};
    use crate::parser::parse_content;

    fn sample() -> Resume {
        Resume {
            name: "Alex Johnson".into(),
            email: "alex@example.com".into(),
            phone: "+1 555-867-5309".into(),
            github: "github.com/alex".into(),
            degree: "B.Tech Computer Science".into(),
            university: "State Technical University".into(),
            year: "2020 -- 2024".into(),
            experience: vec![Experience {
                company: "Acme & Co".into(),
                role: "Backend Intern".into(),
                duration: "Summer 2022".into(),
                details: vec!["Served 50k requests/day".into(), "Cut latency 80%".into()],
            }],
            projects: vec![Project {
                title: "FastSearch".into(),
                description: "Built with Node.js. Indexed 1M docs.".into(),
                tech: vec!["Rust".into(), "Docker".into()],
                details: vec![],
            }],
            skills: vec![SkillGroup {
                category: "Languages".into(),
                items: vec!["Rust".into(), "Python".into()],
            }],
        }
    }

    #[test]
    fn round_trips_through_the_parser() {
        let original = sample();
        assert_eq!(parse_content(&to_text(&original)), original);
    }

    #[test]
    fn round_trips_an_empty_resume() {
        let empty = Resume::default();
        assert_eq!(parse_content(&to_text(&empty)), empty);
    }

    #[test]
    fn round_trips_explicit_project_bullets() {
        let resume = Resume {
            projects: vec![Project {
                title: "T".into(),
                description: String::new(),
                tech: vec!["Rust".into()],
                details: vec!["First bullet".into(), "Second bullet".into()],
            }],
            ..Default::default()
        };
        assert_eq!(parse_content(&to_text(&resume)), resume);
    }

    /// The template ships as text, so it must survive a load/save cycle
    /// unchanged — otherwise opening and saving would silently rewrite it.
    #[test]
    fn round_trips_the_shipped_template() {
        let from_template = parse_content(crate::TEMPLATE);
        assert_eq!(parse_content(&to_text(&from_template)), from_template);
    }
}
