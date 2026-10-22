//! Flattens a section of the resume into the lines shown in the editor pane.
//!
//! The nested model (jobs holding bullets) becomes one flat list, because the
//! thing the user drives is a cursor moving down a list. Adding is an
//! on-screen row rather than a hidden key, so the affordance is visible
//! without reading the help.

use super::field::Field;
use crate::model::{Resume, Section};

pub enum Row {
    /// Heading for one entry, e.g. "Experience 1 · Acme". Selecting it and
    /// pressing `d` removes the whole entry.
    Entry { label: String, delete: Delete },
    /// An editable value.
    Field {
        label: String,
        field: Field,
        delete: Option<Delete>,
    },
    /// An affordance: "+ add experience".
    Add { label: String, action: Add },
    /// Blank line. Never selectable.
    Gap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Add {
    Job,
    JobBullet(usize),
    Project,
    ProjectBullet(usize),
    SkillGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delete {
    Job(usize),
    JobBullet(usize, usize),
    Project(usize),
    ProjectBullet(usize, usize),
    SkillGroup(usize),
}

impl Row {
    pub fn selectable(&self) -> bool {
        !matches!(self, Row::Gap)
    }

    pub fn delete(&self) -> Option<Delete> {
        match self {
            Row::Entry { delete, .. } => Some(*delete),
            Row::Field { delete, .. } => *delete,
            _ => None,
        }
    }

    fn field(label: &str, field: Field) -> Self {
        Row::Field {
            label: label.to_string(),
            field,
            delete: None,
        }
    }

    fn bullet(field: Field, delete: Delete) -> Self {
        Row::Field {
            label: "•".to_string(),
            field,
            delete: Some(delete),
        }
    }
}

pub fn rows(resume: &Resume, section: Section) -> Vec<Row> {
    match section {
        Section::Personal => vec![
            Row::field("name", Field::Name),
            Row::field("email", Field::Email),
            Row::field("phone", Field::Phone),
            Row::field("github", Field::Github),
        ],
        Section::Education => vec![
            Row::field("degree", Field::Degree),
            Row::field("university", Field::University),
            Row::field("year", Field::Year),
        ],
        Section::Experience => experience(resume),
        Section::Projects => projects(resume),
        Section::Skills => skills(resume),
    }
}

fn experience(resume: &Resume) -> Vec<Row> {
    let mut rows = Vec::new();
    for (i, job) in resume.experience.iter().enumerate() {
        if i > 0 {
            rows.push(Row::Gap);
        }
        rows.push(Row::Entry {
            label: heading("Experience", i, &job.company),
            delete: Delete::Job(i),
        });
        rows.push(Row::field("company", Field::JobCompany(i)));
        rows.push(Row::field("role", Field::JobRole(i)));
        rows.push(Row::field("duration", Field::JobDuration(i)));
        for b in 0..job.details.len() {
            rows.push(Row::bullet(Field::JobBullet(i, b), Delete::JobBullet(i, b)));
        }
        rows.push(Row::Add {
            label: "+ bullet".into(),
            action: Add::JobBullet(i),
        });
    }
    rows.push(Row::Gap);
    rows.push(Row::Add {
        label: "+ add experience".into(),
        action: Add::Job,
    });
    rows
}

fn projects(resume: &Resume) -> Vec<Row> {
    let mut rows = Vec::new();
    for (i, project) in resume.projects.iter().enumerate() {
        if i > 0 {
            rows.push(Row::Gap);
        }
        rows.push(Row::Entry {
            label: heading("Project", i, &project.title),
            delete: Delete::Project(i),
        });
        rows.push(Row::field("title", Field::ProjectTitle(i)));
        rows.push(Row::field("tech", Field::ProjectTech(i)));
        for b in 0..project.details.len() {
            rows.push(Row::bullet(
                Field::ProjectBullet(i, b),
                Delete::ProjectBullet(i, b),
            ));
        }
        rows.push(Row::Add {
            label: "+ bullet".into(),
            action: Add::ProjectBullet(i),
        });
    }
    rows.push(Row::Gap);
    rows.push(Row::Add {
        label: "+ add project".into(),
        action: Add::Project,
    });
    rows
}

fn skills(resume: &Resume) -> Vec<Row> {
    let mut rows = Vec::new();
    for (i, group) in resume.skills.iter().enumerate() {
        if i > 0 {
            rows.push(Row::Gap);
        }
        rows.push(Row::Entry {
            label: heading("Group", i, &group.category),
            delete: Delete::SkillGroup(i),
        });
        rows.push(Row::field("category", Field::SkillCategory(i)));
        rows.push(Row::field("items", Field::SkillItems(i)));
    }
    rows.push(Row::Gap);
    rows.push(Row::Add {
        label: "+ add skill group".into(),
        action: Add::SkillGroup,
    });
    rows
}

/// "Experience 2 · Acme", or just "Experience 2" before it has a name.
fn heading(kind: &str, index: usize, name: &str) -> String {
    match name.trim() {
        "" => format!("{kind} {}", index + 1),
        name => format!("{kind} {} · {name}", index + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Experience, Project};

    fn with_one_job() -> Resume {
        Resume {
            experience: vec![Experience {
                company: "Acme".into(),
                details: vec!["did a thing".into()],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn every_section_offers_a_way_to_add() {
        for section in [Section::Experience, Section::Projects, Section::Skills] {
            let rows = rows(&Resume::default(), section);
            assert!(
                rows.iter().any(|r| matches!(r, Row::Add { .. })),
                "{section:?} has no add row"
            );
        }
    }

    #[test]
    fn an_empty_section_is_still_navigable() {
        let rows = rows(&Resume::default(), Section::Projects);
        assert!(rows.iter().any(Row::selectable));
    }

    #[test]
    fn gaps_are_never_selectable() {
        assert!(!Row::Gap.selectable());
        assert!(Row::field("name", Field::Name).selectable());
    }

    /// The row holding `field`, if it is on screen.
    fn row_for(rows: &[Row], field: Field) -> Option<&Row> {
        rows.iter()
            .find(|row| matches!(row, Row::Field { field: f, .. } if *f == field))
    }

    fn has_add(rows: &[Row], action: Add) -> bool {
        rows.iter()
            .any(|row| matches!(row, Row::Add { action: a, .. } if *a == action))
    }

    #[test]
    fn a_job_lists_its_fields_bullets_and_an_add_row() {
        let rows = experience(&with_one_job());
        assert!(matches!(&rows[0], Row::Entry { label, .. } if label == "Experience 1 · Acme"));
        assert!(row_for(&rows, Field::JobBullet(0, 0)).is_some());
        assert!(has_add(&rows, Add::JobBullet(0)));
        assert!(has_add(&rows, Add::Job));
    }

    #[test]
    fn entries_and_bullets_are_deletable_but_plain_fields_are_not() {
        let rows = experience(&with_one_job());

        let entry = rows
            .iter()
            .find(|row| matches!(row, Row::Entry { .. }))
            .unwrap();
        assert_eq!(entry.delete(), Some(Delete::Job(0)));
        assert_eq!(
            row_for(&rows, Field::JobBullet(0, 0)).unwrap().delete(),
            Some(Delete::JobBullet(0, 0))
        );
        assert_eq!(row_for(&rows, Field::JobCompany(0)).unwrap().delete(), None);
    }

    #[test]
    fn headings_fall_back_when_the_entry_is_unnamed() {
        assert_eq!(heading("Project", 0, ""), "Project 1");
        assert_eq!(heading("Project", 1, "  "), "Project 2");
        assert_eq!(
            heading("Project", 2, "FastSearch"),
            "Project 3 · FastSearch"
        );
    }

    #[test]
    fn projects_expose_bullets_not_the_prose_description() {
        let resume = Resume {
            projects: vec![Project {
                title: "T".into(),
                details: vec!["b".into()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let rows = projects(&resume);
        assert!(row_for(&rows, Field::ProjectBullet(0, 0)).is_some());
    }
}
