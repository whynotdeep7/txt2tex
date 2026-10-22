//! Reads the plain-text resume format into a [`Resume`].
//!
//! The format is line-oriented and forgiving by design: unknown keys and
//! unknown sections are skipped rather than rejected, so a typo costs you one
//! field instead of the whole build.

use crate::model::{Experience, Project, Resume, Section, SkillGroup};
use std::fs;
use std::io;
use std::path::Path;

pub fn parse_file(path: &Path) -> io::Result<Resume> {
    Ok(parse_content(&fs::read_to_string(path)?))
}

/// Infallible: every line either contributes a field or is ignored.
pub fn parse_content(content: &str) -> Resume {
    let mut resume = Resume::default();
    let mut section = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(header) = line.strip_prefix('#') {
            section = header_section(header);
            continue;
        }

        match section {
            // Both are flat key/value blocks, so one arm handles them: an
            // unknown key in either is simply ignored.
            Some(Section::Personal | Section::Education) => {
                let Some((key, value)) = split_key_value(line) else {
                    continue;
                };
                match key {
                    "name" => resume.name = value.into(),
                    "email" => resume.email = value.into(),
                    "phone" => resume.phone = value.into(),
                    "github" => resume.github = value.into(),
                    "degree" => resume.degree = value.into(),
                    "university" => resume.university = value.into(),
                    "year" => resume.year = value.into(),
                    _ => {}
                }
            }

            Some(Section::Skills) => {
                if let Some((category, items)) = split_key_value(line) {
                    resume.skills.push(SkillGroup {
                        category: category.into(),
                        items: split_list(items),
                    });
                }
            }

            // `experience:` and `project:` open a new entry. Pushing it
            // immediately and editing through `last_mut` avoids carrying a
            // half-built entry alongside the loop and flushing it at every
            // section boundary.
            Some(Section::Experience) => {
                if line == "experience:" {
                    resume.experience.push(Experience::default());
                    continue;
                }
                let Some(entry) = resume.experience.last_mut() else {
                    continue;
                };
                if let Some(bullet) = line.strip_prefix("- ") {
                    entry.details.push(bullet.trim().into());
                } else if let Some((key, value)) = split_key_value(line) {
                    match key {
                        "company" => entry.company = value.into(),
                        "role" => entry.role = value.into(),
                        "duration" => entry.duration = value.into(),
                        _ => {}
                    }
                }
            }

            Some(Section::Projects) => {
                if line == "project:" {
                    resume.projects.push(Project::default());
                    continue;
                }
                let Some(entry) = resume.projects.last_mut() else {
                    continue;
                };
                if let Some(bullet) = line.strip_prefix("- ") {
                    entry.details.push(bullet.trim().into());
                } else if let Some((key, value)) = split_key_value(line) {
                    match key {
                        "title" => entry.title = value.into(),
                        "description" => entry.description = value.into(),
                        "tech" => entry.tech = split_list(value),
                        _ => {}
                    }
                }
            }

            None => {}
        }
    }

    resume
}

/// Recognises a section header. Case and the space after `#` are both
/// optional, because `#SKILLS` and `# skills` were the two mistakes people
/// actually made.
fn header_section(header: &str) -> Option<Section> {
    match header.trim().to_ascii_uppercase().as_str() {
        "PERSONAL INFORMATION" | "PERSONAL INFO" | "PERSONAL" => Some(Section::Personal),
        "EDUCATION" => Some(Section::Education),
        "EXPERIENCE" | "WORK EXPERIENCE" => Some(Section::Experience),
        "PROJECTS" | "PROJECT" => Some(Section::Projects),
        "SKILLS" | "TECHNICAL SKILLS" => Some(Section::Skills),
        _ => None,
    }
}

/// Splits `key: value` at the first colon. Values keep any later colons, which
/// is what makes `github: https://github.com/x` work.
fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(':')?;
    Some((key.trim(), value.trim()))
}

/// Splits a comma-separated list, dropping empties so a trailing comma does
/// not become a blank skill.
fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_tolerate_case_and_missing_space() {
        for header in ["# SKILLS", "#SKILLS", "# skills", "# Technical Skills"] {
            let resume = parse_content(&format!("{header}\nLanguages: Rust\n"));
            assert_eq!(resume.skills.len(), 1, "failed on {header:?}");
        }
    }

    #[test]
    fn values_may_contain_colons() {
        let resume = parse_content("# PERSONAL\ngithub: https://github.com/alex\n");
        assert_eq!(resume.github, "https://github.com/alex");
    }

    #[test]
    fn trailing_commas_do_not_create_blank_items() {
        let resume = parse_content("# SKILLS\nLanguages: Rust, Python,\n");
        assert_eq!(resume.skills[0].items, ["Rust", "Python"]);
    }

    #[test]
    fn projects_accept_explicit_bullets() {
        let resume = parse_content("# PROJECTS\nproject:\ntitle: T\n- First\n- Second\n");
        assert_eq!(resume.projects[0].details, ["First", "Second"]);
    }

    #[test]
    fn content_before_any_header_is_ignored() {
        let resume = parse_content("name: Nobody\n# PERSONAL\nname: Alex\n");
        assert_eq!(resume.name, "Alex");
    }

    #[test]
    fn unknown_sections_and_keys_are_skipped() {
        let resume = parse_content("# HOBBIES\nchess: yes\n# PERSONAL\nfavourite: x\nname: Alex\n");
        assert_eq!(resume.name, "Alex");
    }
}
