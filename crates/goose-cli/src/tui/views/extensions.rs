use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::tui::acp_client::ExtensionInfo;
use crate::tui::theme::Theme;

pub struct ExtensionsState {
    pub extensions: Vec<ExtensionInfo>,
    pub selected_idx: usize,
}

impl ExtensionsState {
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
            selected_idx: 0,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.extensions.is_empty() && self.selected_idx < self.extensions.len() - 1 {
            self.selected_idx += 1;
        }
    }
}

pub fn render_extensions(f: &mut Frame, area: Rect, state: &ExtensionsState) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .split(area);

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled("◆ ", Style::default().fg(Theme::GOLD)),
        Span::styled("Extensions", Theme::title()),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Extension list
    if state.extensions.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No extensions configured",
            Theme::dim(),
        )))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Theme::border()),
        );
        f.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = state
            .extensions
            .iter()
            .enumerate()
            .map(|(i, ext)| {
                let style = if i == state.selected_idx {
                    Theme::selected()
                } else {
                    Style::default().fg(Theme::TEXT_PRIMARY)
                };
                let status = if ext.enabled { "✓" } else { "○" };
                let status_color = if ext.enabled {
                    Theme::SUCCESS
                } else {
                    Theme::TEXT_DIM
                };
                let line = Line::from(vec![
                    Span::styled(format!(" {status} "), Style::default().fg(status_color)),
                    Span::styled(&ext.name, style),
                    Span::styled(format!("  [{}]", ext.ext_type), Theme::dim()),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Theme::border())
                .title(Span::styled(
                    format!(" extensions ({}) ", state.extensions.len()),
                    Theme::subtitle(),
                )),
        );
        f.render_widget(list, chunks[1]);
    }

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Theme::GOLD)),
        Span::styled(" navigate  ", Theme::dim()),
        Span::styled("space", Style::default().fg(Theme::GOLD)),
        Span::styled(" toggle  ", Theme::dim()),
        Span::styled("esc", Style::default().fg(Theme::GOLD)),
        Span::styled(" back", Theme::dim()),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}
