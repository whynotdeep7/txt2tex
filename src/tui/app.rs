//! TUI state and key handling.
//!
//! Deliberately free of terminal I/O: `App` takes key events and produces
//! state. That keeps the entire interaction model unit-testable without a
//! pty — which matters, because an event loop is otherwise the one part of
//! this program that nothing can check.

use std::fs;
use std::path::{Path, PathBuf};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::edit::LineEditor;
use super::field::Field;
use super::rows::{self, Add, Delete, Row};
use crate::analyzer::{self, Finding};
use crate::model::{Experience, Project, Resume, Section, SkillGroup};
use crate::{pdf, template, writer};

pub enum Mode {
    Normal,
    Editing {
        field: Field,
        editor: LineEditor,
    },
    /// A yes/no question. Anything that destroys work goes through here.
    Confirm {
        prompt: String,
        action: Confirm,
    },
    Help,
}

#[derive(Clone, Copy)]
pub enum Confirm {
    Delete(Delete),
    Quit,
}

pub struct App {
    pub path: PathBuf,
    pub resume: Resume,
    pub section: usize,
    pub rows: Vec<Row>,
    pub selected: usize,
    pub mode: Mode,
    pub findings: Vec<Finding>,
    pub status: String,
    pub dirty: bool,
    pub should_quit: bool,
    /// Set by `b`. The loop draws one frame showing "typesetting…" before
    /// running the build, which otherwise blocks for a second with the screen
    /// showing nothing at all.
    pub build_pending: bool,
}

impl App {
    pub fn new(path: PathBuf, mut resume: Resume) -> Self {
        resume.use_explicit_bullets();
        let mut app = Self {
            path,
            resume,
            section: 0,
            rows: Vec::new(),
            selected: 0,
            mode: Mode::Normal,
            findings: Vec::new(),
            status: "press ? for help".into(),
            dirty: false,
            should_quit: false,
            build_pending: false,
        };
        app.refresh();
        app.select_first();
        app
    }

    pub fn section(&self) -> Section {
        Section::ALL[self.section]
    }

    pub fn pdf_path(&self) -> PathBuf {
        self.path.with_extension("pdf")
    }

    /// Rebuilds everything derived from the resume. Called after every change,
    /// so the row list and the analysis can never drift from the model.
    fn refresh(&mut self) {
        self.rows = rows::rows(&self.resume, self.section());
        self.findings = analyzer::analyze(&self.resume);
        if !self.rows.get(self.selected).is_some_and(Row::selectable) {
            self.select_first();
        }
    }

    fn select_first(&mut self) {
        self.selected = self.rows.iter().position(Row::selectable).unwrap_or(0);
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        // Windows reports Press *and* Release; without this every keystroke
        // would land twice.
        if key.kind != KeyEventKind::Press {
            return;
        }
        match self.mode {
            Mode::Help => self.mode = Mode::Normal,
            Mode::Confirm { .. } => self.confirm_key(key),
            Mode::Editing { .. } => self.editing_key(key),
            Mode::Normal => self.normal_key(key),
        }
    }

    fn normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit(),
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Tab => self.change_section(1),
            KeyCode::BackTab => self.change_section(-1),
            KeyCode::Char(n @ '1'..='5') => self.goto_section(n as usize - '1' as usize),
            KeyCode::Char('i') | KeyCode::Enter => self.activate(),
            KeyCode::Char('a') => self.add_to_section(),
            KeyCode::Char('d') => self.delete_selected(),
            KeyCode::Char('s') => self.save(),
            KeyCode::Char('b') => {
                self.status = "typesetting…".into();
                self.build_pending = true;
            }
            KeyCode::Char('o') => self.open_pdf(),
            KeyCode::Char('?') => self.mode = Mode::Help,
            _ => {}
        }
    }

    fn editing_key(&mut self, key: KeyEvent) {
        // Ctrl-U clears the line, as in a shell. Other Ctrl chords must not
        // fall through and type themselves.
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if let (KeyCode::Char('u'), Mode::Editing { editor, .. }) = (key.code, &mut self.mode) {
                editor.clear();
            }
            return;
        }

        // Esc and Enter both commit, matching vim: leaving insert mode keeps
        // what you typed. Quitting without saving is the way back.
        if matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
            let Mode::Editing { field, editor } = std::mem::replace(&mut self.mode, Mode::Normal)
            else {
                return;
            };
            self.commit(field, editor.into_text());
            return;
        }

        let Mode::Editing { editor, .. } = &mut self.mode else {
            return;
        };
        match key.code {
            KeyCode::Char(c) => editor.insert(c),
            KeyCode::Backspace => editor.backspace(),
            KeyCode::Delete => editor.delete(),
            KeyCode::Left => editor.left(),
            KeyCode::Right => editor.right(),
            KeyCode::Home => editor.home(),
            KeyCode::End => editor.end(),
            _ => {}
        }
    }

    fn confirm_key(&mut self, key: KeyEvent) {
        // Only an explicit yes proceeds; every other key cancels.
        if !matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
            self.mode = Mode::Normal;
            self.status = "cancelled".into();
            return;
        }
        let Mode::Confirm { action, .. } = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };
        match action {
            Confirm::Delete(delete) => self.apply_delete(delete),
            Confirm::Quit => self.should_quit = true,
        }
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.rows.len();
        if count == 0 {
            return;
        }
        // Step over gaps, and wrap at the ends.
        let mut index = self.selected as isize;
        for _ in 0..count {
            index = (index + delta).rem_euclid(count as isize);
            if self.rows[index as usize].selectable() {
                self.selected = index as usize;
                return;
            }
        }
    }

    fn change_section(&mut self, delta: isize) {
        let count = Section::ALL.len() as isize;
        self.goto_section(((self.section as isize + delta).rem_euclid(count)) as usize);
    }

    fn goto_section(&mut self, index: usize) {
        if index >= Section::ALL.len() {
            return;
        }
        self.section = index;
        self.refresh();
        self.select_first();
    }

    fn activate(&mut self) {
        match self.rows.get(self.selected) {
            Some(Row::Field { field, .. }) => self.edit(*field),
            Some(Row::Add { action, .. }) => self.apply_add(*action),
            _ => {}
        }
    }

    fn edit(&mut self, field: Field) {
        self.mode = Mode::Editing {
            field,
            editor: LineEditor::new(field.get(&self.resume)),
        };
    }

    fn commit(&mut self, field: Field, value: String) {
        // Only mark dirty on a real change, so opening a field and leaving it
        // does not claim there is work to save.
        if field.get(&self.resume) != value {
            field.set(&mut self.resume, value);
            self.dirty = true;
        }
        self.refresh();
    }

    fn add_to_section(&mut self) {
        match self.section() {
            Section::Experience => self.apply_add(Add::Job),
            Section::Projects => self.apply_add(Add::Project),
            Section::Skills => self.apply_add(Add::SkillGroup),
            _ => self.status = "this section has a fixed set of fields".into(),
        }
    }

    fn apply_add(&mut self, action: Add) {
        // Every add lands the cursor in the new entry's first field and opens
        // it for typing — which is what you came here to do.
        let focus = match action {
            Add::Job => {
                self.resume.experience.push(Experience::default());
                Field::JobCompany(self.resume.experience.len() - 1)
            }
            Add::Project => {
                self.resume.projects.push(Project::default());
                Field::ProjectTitle(self.resume.projects.len() - 1)
            }
            Add::SkillGroup => {
                self.resume.skills.push(SkillGroup::default());
                Field::SkillCategory(self.resume.skills.len() - 1)
            }
            Add::JobBullet(i) => {
                let Some(job) = self.resume.experience.get_mut(i) else {
                    return;
                };
                job.details.push(String::new());
                Field::JobBullet(i, job.details.len() - 1)
            }
            Add::ProjectBullet(i) => {
                let Some(project) = self.resume.projects.get_mut(i) else {
                    return;
                };
                project.details.push(String::new());
                Field::ProjectBullet(i, project.details.len() - 1)
            }
        };

        self.dirty = true;
        self.refresh();
        self.focus(focus);
        self.edit(focus);
    }

    fn focus(&mut self, target: Field) {
        if let Some(index) = self
            .rows
            .iter()
            .position(|row| matches!(row, Row::Field { field, .. } if *field == target))
        {
            self.selected = index;
        }
    }

    fn delete_selected(&mut self) {
        let Some(delete) = self.rows.get(self.selected).and_then(Row::delete) else {
            self.status = "nothing to delete here".into();
            return;
        };
        self.mode = Mode::Confirm {
            prompt: format!("Delete {}?", self.describe(delete)),
            action: Confirm::Delete(delete),
        };
    }

    fn describe(&self, delete: Delete) -> String {
        let name = |text: &str| match text.trim() {
            "" => "this untitled entry".to_string(),
            text => format!("'{text}'"),
        };
        match delete {
            Delete::Job(i) => self
                .resume
                .experience
                .get(i)
                .map_or("this job".into(), |j| name(&j.company)),
            Delete::Project(i) => self
                .resume
                .projects
                .get(i)
                .map_or("this project".into(), |p| name(&p.title)),
            Delete::SkillGroup(i) => self
                .resume
                .skills
                .get(i)
                .map_or("this group".into(), |g| name(&g.category)),
            Delete::JobBullet(..) | Delete::ProjectBullet(..) => "this bullet".into(),
        }
    }

    fn apply_delete(&mut self, delete: Delete) {
        // Bounds-checked: the row list is rebuilt on every change, but the
        // confirmation carries an index captured before it.
        match delete {
            Delete::Job(i) if i < self.resume.experience.len() => {
                self.resume.experience.remove(i);
            }
            Delete::Project(i) if i < self.resume.projects.len() => {
                self.resume.projects.remove(i);
            }
            Delete::SkillGroup(i) if i < self.resume.skills.len() => {
                self.resume.skills.remove(i);
            }
            Delete::JobBullet(i, b) => {
                if let Some(job) = self
                    .resume
                    .experience
                    .get_mut(i)
                    .filter(|j| b < j.details.len())
                {
                    job.details.remove(b);
                }
            }
            Delete::ProjectBullet(i, b) => {
                if let Some(project) = self
                    .resume
                    .projects
                    .get_mut(i)
                    .filter(|p| b < p.details.len())
                {
                    project.details.remove(b);
                }
            }
            _ => return,
        }
        self.dirty = true;
        self.status = "deleted".into();
        self.refresh();
    }

    fn quit(&mut self) {
        if self.dirty {
            self.mode = Mode::Confirm {
                prompt: "Unsaved changes. Quit without saving?".into(),
                action: Confirm::Quit,
            };
        } else {
            self.should_quit = true;
        }
    }

    pub fn save(&mut self) {
        match write(&self.path, &self.resume) {
            Ok(()) => {
                self.dirty = false;
                self.status = format!("saved {}", self.path.display());
            }
            Err(error) => self.status = format!("could not save: {error}"),
        }
    }

    /// Runs the build. Called by the loop after the "typesetting…" frame is on
    /// screen, because tectonic blocks for about a second.
    pub fn run_build(&mut self) {
        self.build_pending = false;

        if let Err(error) = write(&self.path, &self.resume) {
            self.status = format!("could not save: {error}");
            return;
        }
        self.dirty = false;

        let missing = self.resume.missing_required();
        if !missing.is_empty() {
            self.status = format!("needs a {} before it will build", missing.join(" and "));
            return;
        }

        let output = self.pdf_path();
        match pdf::compile(&template::render(&self.resume), &output) {
            Ok(()) => self.status = format!("built {}", output.display()),
            Err(error) => {
                self.status = format!("build failed — {}", first_line(&error.to_string()))
            }
        }
    }

    fn open_pdf(&mut self) {
        let pdf = self.pdf_path();
        if !pdf.exists() {
            self.status = "no PDF yet — press b to build one".into();
            return;
        }
        crate::open::in_viewer(&pdf);
        self.status = format!("opened {}", pdf.display());
    }
}

fn write(path: &Path, resume: &Resume) -> std::io::Result<()> {
    fs::write(path, writer::to_text(resume))
}

/// Status is one line, and a tectonic failure is many.
fn first_line(text: &str) -> String {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(text)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_content;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn app() -> App {
        App::new(PathBuf::from("resume.txt"), parse_content(crate::TEMPLATE))
    }

    fn type_text(app: &mut App, text: &str) {
        for ch in text.chars() {
            app.on_key(press(KeyCode::Char(ch)));
        }
    }

    #[test]
    fn opens_on_the_first_section_with_something_selected() {
        let app = app();
        assert_eq!(app.section(), Section::Personal);
        assert!(app.rows[app.selected].selectable());
        assert!(!app.dirty);
    }

    #[test]
    fn tab_cycles_sections_and_wraps() {
        let mut app = app();
        for expected in [
            Section::Education,
            Section::Experience,
            Section::Projects,
            Section::Skills,
            Section::Personal,
        ] {
            app.on_key(press(KeyCode::Tab));
            assert_eq!(app.section(), expected);
        }
    }

    #[test]
    fn number_keys_jump_straight_to_a_section() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('4')));
        assert_eq!(app.section(), Section::Projects);
    }

    #[test]
    fn editing_a_field_writes_through_to_the_resume() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('i')));
        app.on_key(press(KeyCode::Char('!')));
        app.on_key(press(KeyCode::Enter));

        assert!(app.resume.name.ends_with('!'));
        assert!(app.dirty);
    }

    /// Esc keeps the text, as in vim. Losing a line to a reflex Esc would be
    /// the worst possible surprise.
    #[test]
    fn escape_commits_rather_than_discards() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('i')));
        app.on_key(press(KeyCode::Char('X')));
        app.on_key(press(KeyCode::Esc));
        assert!(app.resume.name.ends_with('X'));
    }

    #[test]
    fn opening_a_field_and_leaving_it_unchanged_is_not_a_change() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('i')));
        app.on_key(press(KeyCode::Enter));
        assert!(!app.dirty);
    }

    #[test]
    fn keys_do_not_act_as_commands_while_typing() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('i')));
        type_text(&mut app, "qbs");
        app.on_key(press(KeyCode::Enter));

        assert!(
            app.resume.name.ends_with("qbs"),
            "got {:?}",
            app.resume.name
        );
        assert!(!app.should_quit);
        assert!(!app.build_pending);
    }

    #[test]
    fn ctrl_u_clears_the_line_without_typing_a_u() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('i')));
        app.on_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        app.on_key(press(KeyCode::Enter));
        assert_eq!(app.resume.name, "");
    }

    #[test]
    fn key_releases_are_ignored() {
        let mut app = app();
        let before = app.section;
        let mut release = press(KeyCode::Tab);
        release.kind = KeyEventKind::Release;
        app.on_key(release);
        assert_eq!(app.section, before);
    }

    #[test]
    fn adding_an_entry_starts_editing_it() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('3')));
        let before = app.resume.experience.len();
        app.on_key(press(KeyCode::Char('a')));

        assert_eq!(app.resume.experience.len(), before + 1);
        assert!(
            matches!(app.mode, Mode::Editing { .. }),
            "should be editing the new entry"
        );

        type_text(&mut app, "New Co");
        app.on_key(press(KeyCode::Enter));
        assert_eq!(app.resume.experience.last().unwrap().company, "New Co");
    }

    #[test]
    fn deleting_asks_first_and_only_yes_proceeds() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('3')));
        let before = app.resume.experience.len();

        app.on_key(press(KeyCode::Char('d')));
        assert!(matches!(app.mode, Mode::Confirm { .. }));

        app.on_key(press(KeyCode::Char('n')));
        assert_eq!(app.resume.experience.len(), before, "n must not delete");

        app.on_key(press(KeyCode::Char('d')));
        app.on_key(press(KeyCode::Char('y')));
        assert_eq!(app.resume.experience.len(), before - 1);
    }

    #[test]
    fn quitting_with_unsaved_changes_asks_first() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('i')));
        app.on_key(press(KeyCode::Char('z')));
        app.on_key(press(KeyCode::Enter));

        app.on_key(press(KeyCode::Char('q')));
        assert!(!app.should_quit);
        assert!(matches!(app.mode, Mode::Confirm { .. }));

        app.on_key(press(KeyCode::Char('y')));
        assert!(app.should_quit);
    }

    #[test]
    fn quitting_clean_leaves_immediately() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn navigation_never_lands_on_a_gap() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('3')));
        for _ in 0..60 {
            app.on_key(press(KeyCode::Char('j')));
            assert!(app.rows[app.selected].selectable());
        }
        for _ in 0..60 {
            app.on_key(press(KeyCode::Char('k')));
            assert!(app.rows[app.selected].selectable());
        }
    }

    #[test]
    fn an_empty_resume_is_still_navigable() {
        let mut app = App::new(PathBuf::from("x.txt"), Resume::default());
        for section in 0..5 {
            app.on_key(press(KeyCode::Char((b'1' + section) as char)));
            app.on_key(press(KeyCode::Char('j')));
            app.on_key(press(KeyCode::Char('k')));
        }
        assert!(!app.should_quit);
    }

    #[test]
    fn deleting_the_last_entry_leaves_a_valid_selection() {
        let mut app = App::new(
            PathBuf::from("x.txt"),
            Resume {
                experience: vec![Experience::default()],
                ..Default::default()
            },
        );
        app.on_key(press(KeyCode::Char('3')));
        app.on_key(press(KeyCode::Char('d')));
        app.on_key(press(KeyCode::Char('y')));

        assert!(app.resume.experience.is_empty());
        assert!(app.rows[app.selected].selectable());
    }

    #[test]
    fn the_analysis_updates_as_the_resume_changes() {
        let mut app = App::new(PathBuf::from("x.txt"), Resume::default());
        let before = app.findings.len();
        app.on_key(press(KeyCode::Char('i')));
        type_text(&mut app, "Alex");
        app.on_key(press(KeyCode::Enter));
        assert!(
            app.findings.len() < before,
            "fixing the name should clear a finding"
        );
    }

    #[test]
    fn help_opens_and_any_key_closes_it() {
        let mut app = app();
        app.on_key(press(KeyCode::Char('?')));
        assert!(matches!(app.mode, Mode::Help));
        app.on_key(press(KeyCode::Char('x')));
        assert!(matches!(app.mode, Mode::Normal));
    }

    /// The TUI edits bullets, so prose descriptions are converted on load.
    #[test]
    fn projects_arrive_as_editable_bullets() {
        let app = App::new(PathBuf::from("x.txt"), parse_content(crate::TEMPLATE));
        assert!(app.resume.projects.iter().all(|p| p.description.is_empty()));
        assert!(app.resume.projects.iter().all(|p| !p.details.is_empty()));
    }
}
