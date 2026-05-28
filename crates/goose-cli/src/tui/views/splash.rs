use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::tui::theme::Theme;

const GOOSE_ART: &[&str] = &[
    r#"                                       "#,
    r#"       ___                             "#,
    r#"      / _ \___  ___  ___  ___          "#,
    r#"     / /_\/ _ \/ _ \/ __|/ _ \         "#,
    r#"    / /_\\  __/ (_) \__ \  __/         "#,
    r#"    \____/\___|\___/|___/\___|         "#,
    r#"                                       "#,
];

const TAGLINE: &str = "your open-source AI agent";

pub fn render_splash(f: &mut Frame, area: Rect, tick: u64) {
    let chunks = Layout::vertical([
        Constraint::Percentage(25),
        Constraint::Length(9),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(area);

    // ASCII art
    let art_lines: Vec<Line> = GOOSE_ART
        .iter()
        .map(|line| Line::from(Span::styled(*line, Style::default().fg(Theme::CRANBERRY))))
        .collect();

    let art = Paragraph::new(art_lines).alignment(Alignment::Center);
    f.render_widget(art, chunks[1]);

    // Tagline
    let tagline = Paragraph::new(Line::from(Span::styled(
        TAGLINE,
        Style::default()
            .fg(Theme::TEXT_SECONDARY)
            .add_modifier(Modifier::ITALIC),
    )))
    .alignment(Alignment::Center);
    f.render_widget(tagline, chunks[2]);

    // Loading indicator
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_chars[(tick as usize / 2) % spinner_chars.len()];
    let loading = Paragraph::new(Line::from(vec![
        Span::styled(format!("{spinner} "), Style::default().fg(Theme::TEAL)),
        Span::styled("initializing...", Style::default().fg(Theme::TEXT_DIM)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(loading, chunks[3]);
}
