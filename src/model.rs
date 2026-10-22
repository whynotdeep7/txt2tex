//! The resume document.
//!
//! Every leaf is a single-line `String`. That is a deliberate constraint: it
//! keeps the parser trivial and means the TUI needs only a one-line editor
//! rather than a full text buffer.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resume {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub github: String,
    pub degree: String,
    pub university: String,
    pub year: String,
    pub experience: Vec<Experience>,
    pub projects: Vec<Project>,
    pub skills: Vec<SkillGroup>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Experience {
    pub company: String,
    pub role: String,
    pub duration: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Project {
    pub title: String,
    /// Free prose, split into bullets at render time. Kept for the original
    /// file format; `details` takes precedence when both are present.
    pub description: String,
    pub tech: Vec<String>,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillGroup {
    pub category: String,
    pub items: Vec<String>,
}

/// The sections of the document, in the order they are typeset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Personal,
    Education,
    Experience,
    Projects,
    Skills,
}

impl Section {
    pub const ALL: [Section; 5] = [
        Section::Personal,
        Section::Education,
        Section::Experience,
        Section::Projects,
        Section::Skills,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::Personal => "Personal Info",
            Section::Education => "Education",
            Section::Experience => "Experience",
            Section::Projects => "Projects",
            Section::Skills => "Skills",
        }
    }
}

impl Resume {
    /// Fields the document heading cannot be built without.
    pub fn missing_required(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.name.trim().is_empty() {
            missing.push("name");
        }
        if self.email.trim().is_empty() {
            missing.push("email");
        }
        missing
    }

    pub fn is_valid(&self) -> bool {
        self.missing_required().is_empty()
    }

    /// Turns every project's prose `description` into explicit bullets.
    ///
    /// The TUI edits bullets, so a project written the old way — one long
    /// `description:` line — is converted once as it loads, and there is only
    /// ever one thing on screen to edit. The PDF is byte-identical either way:
    /// [`Project::bullets`] already splits that prose the same way.
    pub fn use_explicit_bullets(&mut self) {
        for project in &mut self.projects {
            if project.details.is_empty() && !project.description.is_empty() {
                project.details = sentences(&project.description)
                    .into_iter()
                    .map(String::from)
                    .collect();
                project.description.clear();
            }
        }
    }

    /// How many entries a section holds, for the TUI's section list. Personal
    /// and Education are fixed sets of fields, so they have no count.
    pub fn count(&self, section: Section) -> Option<usize> {
        match section {
            Section::Personal | Section::Education => None,
            Section::Experience => Some(self.experience.len()),
            Section::Projects => Some(self.projects.len()),
            Section::Skills => Some(self.skills.len()),
        }
    }
}

impl Project {
    /// The bullets to typeset: explicit ones if the user wrote any, otherwise
    /// the description split into sentences.
    pub fn bullets(&self) -> Vec<&str> {
        if !self.details.is_empty() {
            return self.details.iter().map(String::as_str).collect();
        }
        sentences(&self.description)
    }
}

/// Splits prose into sentence-sized bullets.
///
/// Splits on `". "` rather than `'.'` so that `Node.js`, `1.5s` and `.NET`
/// survive intact — the old `split('.')` turned "Built with Node.js" into two
/// bullets.
pub fn sentences(text: &str) -> Vec<&str> {
    text.split(". ")
        .map(|s| s.trim())
        .map(|s| s.strip_suffix('.').unwrap_or(s))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentences_split_on_sentence_boundaries() {
        assert_eq!(
            sentences("One thing. Two things."),
            ["One thing", "Two things"]
        );
    }

    /// The regression: `split('.')` shattered these into fragments like
    /// "Built with Node" / "js and 1" / "5s latency".
    #[test]
    fn sentences_keep_dotted_words_intact() {
        assert_eq!(
            sentences("Built with Node.js and 1.5s latency"),
            ["Built with Node.js and 1.5s latency"]
        );
        assert_eq!(
            sentences("Used .NET here. Then Vue.js."),
            ["Used .NET here", "Then Vue.js"]
        );
    }

    #[test]
    fn sentences_ignore_empty_fragments() {
        assert_eq!(sentences(""), Vec::<&str>::new());
        assert_eq!(sentences("   "), Vec::<&str>::new());
    }

    #[test]
    fn explicit_bullets_win_over_description() {
        let project = Project {
            description: "Ignored prose.".into(),
            details: vec!["Explicit bullet".into()],
            ..Default::default()
        };
        assert_eq!(project.bullets(), ["Explicit bullet"]);
    }

    /// Converting prose to bullets must not change a single rendered bullet —
    /// otherwise merely opening the TUI would alter someone's PDF.
    #[test]
    fn using_explicit_bullets_preserves_the_rendered_bullets() {
        let mut resume = Resume {
            projects: vec![Project {
                description: "Built with Node.js. Served 10k users. Cut latency to 1.5s.".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let before: Vec<String> = resume.projects[0]
            .bullets()
            .iter()
            .map(|s| s.to_string())
            .collect();
        resume.use_explicit_bullets();
        let after: Vec<String> = resume.projects[0]
            .bullets()
            .iter()
            .map(|s| s.to_string())
            .collect();

        assert_eq!(before, after);
        assert_eq!(
            after,
            [
                "Built with Node.js",
                "Served 10k users",
                "Cut latency to 1.5s"
            ]
        );
        assert!(resume.projects[0].description.is_empty());
    }

    #[test]
    fn using_explicit_bullets_leaves_existing_bullets_alone() {
        let mut resume = Resume {
            projects: vec![Project {
                description: "Prose that must not win.".into(),
                details: vec!["Handwritten".into()],
                ..Default::default()
            }],
            ..Default::default()
        };
        resume.use_explicit_bullets();
        assert_eq!(resume.projects[0].details, ["Handwritten"]);
        assert_eq!(resume.projects[0].description, "Prose that must not win.");
    }

    #[test]
    fn missing_required_names_the_gaps() {
        let resume = Resume::default();
        assert_eq!(resume.missing_required(), ["name", "email"]);
        assert!(!resume.is_valid());
    }
}
