//! Addressing one editable string inside a [`Resume`].
//!
//! A `Field` is a path, not a reference. It can sit in the app state across
//! redraws without borrowing the resume, and it stays sane when the thing it
//! points at is deleted — every lookup goes through `get`/`get_mut`, so a
//! stale index reads as empty instead of panicking.

use crate::model::Resume;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Email,
    Phone,
    Github,

    Degree,
    University,
    Year,

    JobCompany(usize),
    JobRole(usize),
    JobDuration(usize),
    JobBullet(usize, usize),

    ProjectTitle(usize),
    ProjectTech(usize),
    ProjectBullet(usize, usize),

    SkillCategory(usize),
    SkillItems(usize),
}

impl Field {
    pub fn get(self, resume: &Resume) -> String {
        match self {
            Field::Name => resume.name.clone(),
            Field::Email => resume.email.clone(),
            Field::Phone => resume.phone.clone(),
            Field::Github => resume.github.clone(),

            Field::Degree => resume.degree.clone(),
            Field::University => resume.university.clone(),
            Field::Year => resume.year.clone(),

            Field::JobCompany(i) => job(resume, i, |j| j.company.clone()),
            Field::JobRole(i) => job(resume, i, |j| j.role.clone()),
            Field::JobDuration(i) => job(resume, i, |j| j.duration.clone()),
            Field::JobBullet(i, b) => {
                job(resume, i, |j| j.details.get(b).cloned().unwrap_or_default())
            }

            Field::ProjectTitle(i) => project(resume, i, |p| p.title.clone()),
            // Lists are edited the way they are written in the file: as one
            // comma-separated line.
            Field::ProjectTech(i) => project(resume, i, |p| p.tech.join(", ")),
            Field::ProjectBullet(i, b) => {
                project(resume, i, |p| p.details.get(b).cloned().unwrap_or_default())
            }

            Field::SkillCategory(i) => skill(resume, i, |g| g.category.clone()),
            Field::SkillItems(i) => skill(resume, i, |g| g.items.join(", ")),
        }
    }

    pub fn set(self, resume: &mut Resume, value: String) {
        match self {
            Field::Name => resume.name = value,
            Field::Email => resume.email = value,
            Field::Phone => resume.phone = value,
            Field::Github => resume.github = value,

            Field::Degree => resume.degree = value,
            Field::University => resume.university = value,
            Field::Year => resume.year = value,

            Field::JobCompany(i) => set_job(resume, i, |j| j.company = value),
            Field::JobRole(i) => set_job(resume, i, |j| j.role = value),
            Field::JobDuration(i) => set_job(resume, i, |j| j.duration = value),
            Field::JobBullet(i, b) => set_job(resume, i, |j| {
                if let Some(bullet) = j.details.get_mut(b) {
                    *bullet = value;
                }
            }),

            Field::ProjectTitle(i) => set_project(resume, i, |p| p.title = value),
            Field::ProjectTech(i) => set_project(resume, i, |p| p.tech = split_list(&value)),
            Field::ProjectBullet(i, b) => set_project(resume, i, |p| {
                if let Some(bullet) = p.details.get_mut(b) {
                    *bullet = value;
                }
            }),

            Field::SkillCategory(i) => set_skill(resume, i, |g| g.category = value),
            Field::SkillItems(i) => set_skill(resume, i, |g| g.items = split_list(&value)),
        }
    }
}

fn job<T: Default>(resume: &Resume, i: usize, f: impl FnOnce(&crate::model::Experience) -> T) -> T {
    resume.experience.get(i).map(f).unwrap_or_default()
}

fn project<T: Default>(
    resume: &Resume,
    i: usize,
    f: impl FnOnce(&crate::model::Project) -> T,
) -> T {
    resume.projects.get(i).map(f).unwrap_or_default()
}

fn skill<T: Default>(
    resume: &Resume,
    i: usize,
    f: impl FnOnce(&crate::model::SkillGroup) -> T,
) -> T {
    resume.skills.get(i).map(f).unwrap_or_default()
}

fn set_job(resume: &mut Resume, i: usize, f: impl FnOnce(&mut crate::model::Experience)) {
    if let Some(entry) = resume.experience.get_mut(i) {
        f(entry);
    }
}

fn set_project(resume: &mut Resume, i: usize, f: impl FnOnce(&mut crate::model::Project)) {
    if let Some(entry) = resume.projects.get_mut(i) {
        f(entry);
    }
}

fn set_skill(resume: &mut Resume, i: usize, f: impl FnOnce(&mut crate::model::SkillGroup)) {
    if let Some(entry) = resume.skills.get_mut(i) {
        f(entry);
    }
}

/// Same rule as the parser's list splitting, so what you type in the TUI and
/// what you type in the file mean the same thing.
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
    use crate::model::{Experience, Project};

    #[test]
    fn reads_and_writes_a_plain_field() {
        let mut resume = Resume::default();
        Field::Name.set(&mut resume, "Alex".into());
        assert_eq!(resume.name, "Alex");
        assert_eq!(Field::Name.get(&resume), "Alex");
    }

    #[test]
    fn lists_are_edited_as_comma_separated_text() {
        let mut resume = Resume {
            projects: vec![Project::default()],
            ..Default::default()
        };
        Field::ProjectTech(0).set(&mut resume, "Rust, Docker , SQLite".into());
        assert_eq!(resume.projects[0].tech, ["Rust", "Docker", "SQLite"]);
        assert_eq!(Field::ProjectTech(0).get(&resume), "Rust, Docker, SQLite");
    }

    #[test]
    fn nested_bullets_round_trip() {
        let mut resume = Resume {
            experience: vec![Experience {
                details: vec!["old".into()],
                ..Default::default()
            }],
            ..Default::default()
        };
        Field::JobBullet(0, 0).set(&mut resume, "new".into());
        assert_eq!(Field::JobBullet(0, 0).get(&resume), "new");
    }

    /// A field can outlive the entry it points at — the row list is rebuilt
    /// after a delete, but a key event may still carry the old index.
    #[test]
    fn a_stale_index_reads_empty_instead_of_panicking() {
        let mut resume = Resume::default();
        assert_eq!(Field::JobCompany(7).get(&resume), "");
        assert_eq!(Field::JobBullet(7, 3).get(&resume), "");
        assert_eq!(Field::SkillItems(2).get(&resume), "");

        Field::JobCompany(7).set(&mut resume, "ignored".into());
        assert!(resume.experience.is_empty());
    }
}
