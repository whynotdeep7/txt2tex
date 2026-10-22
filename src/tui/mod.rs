//! The interactive editor.
//!
//! Split so that the thinking is testable and only the loop touches a
//! terminal:
//!
//! - [`app`] holds the state machine — key in, state out, no I/O
//! - [`rows`] flattens a resume section into the lines on screen
//! - [`edit`] is the one-line text editor
//! - [`field`] addresses a single string inside the resume
//! - [`ui`] draws, and decides nothing

mod app;
mod edit;
mod field;
mod rows;
#[cfg(test)]
mod svg;
mod ui;

pub use app::App;

use crate::model::Resume;
use ratatui::crossterm::event::{self, Event};
use ratatui::widgets::ListState;
use std::io;
use std::path::PathBuf;

/// Runs the editor until the user quits.
///
/// `ratatui::run` owns the terminal lifecycle: raw mode, the alternate screen,
/// and — the part that matters — a panic hook that restores both. Without it a
/// panic anywhere in here would drop the user back to a shell with no echo and
/// no cursor, needing a blind `reset`.
pub fn run(path: PathBuf, resume: Resume) -> io::Result<()> {
    let mut app = App::new(path, resume);
    let mut list = ListState::default();

    ratatui::run(|terminal| {
        while !app.should_quit {
            terminal.draw(|frame| ui::draw(frame, &app, &mut list))?;

            // tectonic blocks for about a second, so the build runs *after*
            // the frame that announces it. Otherwise the screen simply freezes
            // with no explanation.
            if app.build_pending {
                app.run_build();
                continue;
            }

            // A blocking read: nothing animates, so there is no reason to spin
            // on a poll timeout and wake the CPU sixty times a second.
            if let Event::Key(key) = event::read()? {
                app.on_key(key);
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_content;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::Terminal;

    /// Renders a frame to an in-memory backend. Lets the layout be checked
    /// without a terminal, which is the only way any of this is testable in CI.
    fn render(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut list = ListState::default();
        terminal
            .draw(|frame| ui::draw(frame, app, &mut list))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn demo() -> App {
        App::new(PathBuf::from("resume.txt"), parse_content(crate::TEMPLATE))
    }

    #[test]
    fn the_frame_shows_the_essentials() {
        let screen = render(&demo(), 100, 30);
        assert!(screen.contains("txt2tex"));
        assert!(screen.contains("resume.txt"));
        assert!(screen.contains("Sections"));
        assert!(screen.contains("Analyzer"));
        assert!(screen.contains("score"));
        assert!(screen.contains("Personal Info"));
        assert!(screen.contains("Alex Johnson"));
        assert!(screen.contains("q quit"));
    }

    /// A resume with nothing in it must still draw, not panic on an empty list.
    #[test]
    fn an_empty_resume_renders() {
        let app = App::new(PathBuf::from("x.txt"), crate::model::Resume::default());
        assert!(render(&app, 100, 30).contains("Sections"));
    }

    /// Terminals get resized to silly sizes; none of them may panic.
    #[test]
    fn extreme_terminal_sizes_do_not_panic() {
        let app = demo();
        for (width, height) in [(20, 10), (250, 80), (40, 12)] {
            render(&app, width, height);
        }
    }

    #[test]
    fn the_help_overlay_draws_over_the_page() {
        let mut app = demo();
        app.on_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        let screen = render(&app, 100, 30);
        assert!(screen.contains("Keys"));
        assert!(screen.contains("quitting without saving"));
    }

    /// The selection marker and the block cursor are the only things telling
    /// the user where they are, and neither shows up in the plain text of the
    /// buffer — both are styling, so both need checking directly.
    #[test]
    fn editing_shows_a_block_cursor_and_the_row_is_marked() {
        let mut app = demo();
        app.on_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));

        let mut terminal = Terminal::new(TestBackend::new(96, 28)).unwrap();
        let mut list = ListState::default();
        terminal
            .draw(|frame| ui::draw(frame, &app, &mut list))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();

        let reversed = buffer
            .content()
            .iter()
            .any(|cell| cell.modifier.contains(ratatui::style::Modifier::REVERSED));
        assert!(reversed, "no block cursor rendered while editing");

        assert!(
            (0..buffer.area.height).any(|y| buffer[(1, y)].symbol() == "▸"),
            "no selection marker in the field pane"
        );
    }

    #[test]
    fn the_confirm_overlay_names_what_it_will_delete() {
        let mut app = demo();
        app.on_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        app.on_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        let screen = render(&app, 100, 30);
        assert!(screen.contains("Delete"), "{screen}");
        assert!(screen.contains("Google Summer of Code"), "{screen}");
    }

    /// Labels are DarkGray; a DarkGray highlight once made the selected
    /// row's label invisible.
    #[test]
    fn the_selected_rows_label_stays_visible() {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut list = ListState::default();
        terminal
            .draw(|frame| ui::draw(frame, &demo(), &mut list))
            .unwrap();
        let buffer = terminal.backend().buffer();

        let y = (0..buffer.area.height)
            .find(|&y| buffer[(1, y)].symbol() == "▸")
            .expect("a selected row");
        let label = (2..buffer.area.width)
            .map(|x| &buffer[(x, y)])
            .find(|cell| cell.symbol().trim() != "")
            .expect("text on the selected row");
        assert_ne!(
            label.fg, label.bg,
            "label is drawn in its own background colour"
        );
    }

    /// Writes the README's screenshot of the editor. Ignored because it
    /// writes into the repo; run it after changing the UI:
    ///
    ///     cargo test readme_screenshot -- --ignored
    #[test]
    #[ignore]
    fn readme_screenshot() {
        let mut app = demo();
        let key = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        app.on_key(key('3'));
        for _ in 0..3 {
            app.on_key(key('j'));
        }

        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut list = ListState::default();
        terminal
            .draw(|frame| ui::draw(frame, &app, &mut list))
            .unwrap();

        let svg = svg::render(terminal.backend().buffer(), "txt2tex — resume.txt");
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/tui.svg");
        std::fs::write(path, svg).unwrap();
    }
}
