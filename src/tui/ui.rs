//! Drawing. Reads `App`, writes to the frame, decides nothing.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::app::{App, Mode};
use super::rows::Row;
use crate::analyzer::{score, Finding, Severity};
use crate::model::Section;

/// Width reserved for a field's label, so values line up down the pane.
const LABEL_WIDTH: usize = 10;

pub fn draw(frame: &mut Frame, app: &App, list: &mut ListState) {
    let [title, top, fields, status, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(9),
        Constraint::Min(5),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let [sections_area, analysis_area] =
        Layout::horizontal([Constraint::Length(28), Constraint::Min(20)]).areas(top);

    title_bar(frame, title, app);
    sections(frame, sections_area, app);
    analysis(frame, analysis_area, app);
    field_pane(frame, fields, app, list);
    status_bar(frame, status, app);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(key_hints(app), dim()))),
        hints,
    );

    // Overlays go last so they sit on top of everything.
    match &app.mode {
        Mode::Help => help(frame, frame.area()),
        Mode::Confirm { prompt, .. } => confirm(frame, frame.area(), prompt),
        _ => {}
    }
}

fn title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let [left, right] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(16)]).areas(area);

    let mut spans = vec![
        Span::styled(
            " txt2tex ",
            Style::new().fg(Color::Black).bg(Color::Cyan).bold(),
        ),
        Span::raw(" "),
        Span::raw(app.path.display().to_string()),
    ];
    // A dot for unsaved work, in the one place the eye already goes.
    if app.dirty {
        spans.push(Span::styled(" ●", Style::new().fg(Color::Yellow)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), left);

    let score = score(&app.findings);
    frame.render_widget(
        Paragraph::new(
            Line::from(Span::styled(
                format!("score {score}/100 "),
                Style::new().fg(score_colour(score)).bold(),
            ))
            .right_aligned(),
        ),
        right,
    );
}

fn sections(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = Section::ALL
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let count = app
                .resume
                .count(*section)
                .map(|n| n.to_string())
                .unwrap_or_default();
            let issues = app
                .findings
                .iter()
                .filter(|f| f.section == *section && f.severity == Severity::Issue)
                .count();

            let name_style = if index == app.section {
                Style::new().fg(Color::Cyan).bold()
            } else {
                Style::new()
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    if index == app.section { " ▸ " } else { "   " },
                    Style::new().fg(Color::Cyan),
                ),
                Span::styled(format!("{:<14}", section.title()), name_style),
                Span::styled(format!("{count:>2} "), dim()),
                // Mark the sections with real problems, so the section list
                // doubles as a to-do list.
                Span::styled(
                    if issues > 0 { "✗" } else { " " },
                    Style::new().fg(Color::Red),
                ),
            ]))
        })
        .collect();

    frame.render_widget(List::new(items).block(pane("Sections")), area);
}

fn analysis(frame: &mut Frame, area: Rect, app: &App) {
    if app.findings.is_empty() {
        let line = Line::from(Span::styled(
            " ✓ nothing to fix",
            Style::new().fg(Color::Green),
        ));
        frame.render_widget(Paragraph::new(line).block(pane("Analyzer")), area);
        return;
    }

    // Findings for the section you are looking at come first and stay bright;
    // the rest are context.
    let mut findings: Vec<&Finding> = app.findings.iter().collect();
    findings.sort_by_key(|f| f.section != app.section());

    let lines: Vec<Line> = findings
        .iter()
        .map(|finding| {
            let (mark, colour) = match finding.severity {
                Severity::Issue => ("✗", Color::Red),
                Severity::Tip => ("→", Color::Yellow),
            };
            let text = if finding.section == app.section() {
                Style::new()
            } else {
                dim()
            };
            Line::from(vec![
                Span::styled(format!(" {mark} "), Style::new().fg(colour)),
                Span::styled(finding.message.clone(), text),
            ])
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(pane("Analyzer")),
        area,
    );
}

fn field_pane(frame: &mut Frame, area: Rect, app: &App, list: &mut ListState) {
    // Two borders and the label column are not available to the value.
    let value_width = (area.width as usize).saturating_sub(LABEL_WIDTH + 8);

    let items: Vec<ListItem> = app
        .rows
        .iter()
        .map(|row| ListItem::new(row_line(app, row, value_width)))
        .collect();

    list.select(Some(app.selected));

    // Stateful, so the list scrolls the selection into view on its own. A
    // plain render_widget would silently ignore the state: no cursor, and a
    // section taller than the pane could never be reached.
    frame.render_stateful_widget(
        List::new(items)
            .block(pane(app.section().title()))
            .highlight_style(
                Style::new()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ")
            // Keep a little context visible either side when scrolling.
            .scroll_padding(2),
        area,
        list,
    );
}

fn row_line(app: &App, row: &Row, value_width: usize) -> Line<'static> {
    match row {
        Row::Gap => Line::from(""),

        Row::Entry { label, .. } => Line::from(Span::styled(
            format!(" {label}"),
            Style::new().fg(Color::Cyan).bold(),
        )),

        Row::Add { label, .. } => Line::from(Span::styled(
            format!("  {label}"),
            Style::new().fg(Color::Green),
        )),

        Row::Field { label, field, .. } => {
            let label = Span::styled(format!(" {label:<LABEL_WIDTH$} "), dim());

            match &app.mode {
                // The field being typed into: draw the text around a block
                // cursor. ratatui has no text input, so the cursor is a styled
                // cell rather than the terminal's own.
                Mode::Editing {
                    field: editing,
                    editor,
                } if editing == field => {
                    let (before, under, after) = editor.view(value_width);
                    Line::from(vec![
                        label,
                        Span::raw(before),
                        Span::styled(under, Style::new().add_modifier(Modifier::REVERSED)),
                        Span::raw(after),
                    ])
                }
                _ => {
                    let value = field.get(&app.resume);
                    if value.is_empty() {
                        // An em-dash reads as "nothing here yet" rather than
                        // leaving the eye to wonder if the row is broken.
                        Line::from(vec![label, Span::styled("—", dim())])
                    } else {
                        Line::from(vec![label, Span::raw(truncate(&value, value_width))])
                    }
                }
            }
        }
    }
}

fn status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let colour = if app.status.contains("failed") || app.status.contains("could not") {
        Color::Red
    } else {
        Color::Green
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" {}", app.status),
            Style::new().fg(colour),
        ))),
        area,
    );
}

fn key_hints(app: &App) -> &'static str {
    match app.mode {
        Mode::Editing { .. } => " Enter/Esc done · Ctrl-U clear · ←→ move · Home/End",
        Mode::Confirm { .. } => " y confirm · any other key cancels",
        Mode::Help => " any key closes",
        Mode::Normal => {
            " ↑↓ move · Tab section · i edit · a add · d delete · s save · b build · o open · ? help · q quit"
        }
    }
}

fn help(frame: &mut Frame, area: Rect) {
    let rows = [
        ("↑ ↓ / j k", "move between fields"),
        ("Tab / ⇧Tab", "next / previous section"),
        ("1 … 5", "jump straight to a section"),
        ("i / Enter", "edit the selected field"),
        ("a", "add an entry to this section"),
        ("d", "delete the selected entry or bullet"),
        ("s", "save resume.txt"),
        ("b", "save and build the PDF"),
        ("o", "open the PDF"),
        ("q", "quit"),
        ("", ""),
        ("Enter / Esc", "finish editing, keeping the text"),
        ("Ctrl-U", "clear the field"),
        ("", ""),
        ("", "Nothing is written until you press s or b,"),
        ("", "so quitting without saving undoes everything."),
    ];

    let lines: Vec<Line> = rows
        .iter()
        .map(|(key, description)| {
            Line::from(vec![
                Span::styled(format!("  {key:<12}"), Style::new().fg(Color::Cyan)),
                Span::raw(*description),
            ])
        })
        .collect();

    let popup = centred(62, lines.len() as u16 + 2, area);
    frame.render_widget(Clear, popup);
    frame.render_widget(Paragraph::new(lines).block(pane("Keys")), popup);
}

fn confirm(frame: &mut Frame, area: Rect, prompt: &str) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::raw(format!("  {prompt}"))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y", Style::new().fg(Color::Red).bold()),
            Span::raw(" yes    "),
            Span::styled("any other key", Style::new().fg(Color::Cyan)),
            Span::raw(" no"),
        ]),
    ];
    let width = (prompt.chars().count() as u16 + 8).max(40);
    let popup = centred(width, 6, area);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().border_style(Style::new().fg(Color::Yellow))),
        popup,
    );
}

fn pane(title: &str) -> Block<'static> {
    Block::bordered()
        .border_style(Style::new().fg(Color::DarkGray))
        .title(Span::styled(
            format!(" {title} "),
            Style::new().fg(Color::White).bold(),
        ))
}

fn centred(width: u16, height: u16, area: Rect) -> Rect {
    let [_, middle, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height.min(area.height)),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [_, centre, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width.min(area.width)),
        Constraint::Fill(1),
    ])
    .areas(middle);
    centre
}

fn dim() -> Style {
    Style::new().fg(Color::DarkGray)
}

fn score_colour(score: u8) -> Color {
    match score {
        90..=100 => Color::Green,
        60..=89 => Color::Yellow,
        _ => Color::Red,
    }
}

/// Character-wise, so a value with an accent in it cannot panic the renderer.
fn truncate(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((byte, _)) => format!("{}…", &text[..byte]),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_counts_characters_not_bytes() {
        assert_eq!(truncate("Zürich — 北京", 3), "Zür…");
        assert_eq!(truncate("short", 40), "short");
    }

    #[test]
    fn score_colour_bands() {
        assert_eq!(score_colour(100), Color::Green);
        assert_eq!(score_colour(75), Color::Yellow);
        assert_eq!(score_colour(10), Color::Red);
    }
}
