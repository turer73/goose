use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::tui::acp_client::ProviderInfo;
use crate::tui::theme::Theme;

pub struct OnboardingState {
    pub providers: Vec<ProviderInfo>,
    pub selected_idx: usize,
    pub search_query: String,
}

impl OnboardingState {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            selected_idx: 0,
            search_query: String::new(),
        }
    }

    pub fn filtered_providers(&self) -> Vec<&ProviderInfo> {
        if self.search_query.is_empty() {
            self.providers.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.providers
                .iter()
                .filter(|p| p.name.to_lowercase().contains(&q) || p.id.to_lowercase().contains(&q))
                .collect()
        }
    }

    pub fn move_up(&mut self) {
        let count = self.filtered_providers().len();
        if count > 0 && self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let count = self.filtered_providers().len();
        if count > 0 && self.selected_idx < count - 1 {
            self.selected_idx += 1;
        }
    }
}

pub fn render_onboarding(f: &mut Frame, area: Rect, state: &OnboardingState) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .split(area);

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled("◆ ", Style::default().fg(Theme::CRANBERRY)),
        Span::styled("Setup Provider", Theme::title()),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Subtitle
    let subtitle = Paragraph::new(Line::from(Span::styled(
        "Choose an AI provider to get started",
        Theme::subtitle(),
    )))
    .alignment(Alignment::Center);
    f.render_widget(subtitle, chunks[1]);

    // Search
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_active())
        .title(Span::styled(" search ", Theme::dim()));
    let search_text = if state.search_query.is_empty() {
        Span::styled("type to filter...", Theme::dim())
    } else {
        Span::styled(
            state.search_query.clone(),
            Style::default().fg(Theme::TEXT_PRIMARY),
        )
    };
    let search = Paragraph::new(Line::from(search_text)).block(search_block);
    f.render_widget(search, chunks[2]);

    // Provider list
    let filtered = state.filtered_providers();
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, provider)| {
            let marker = if provider.configured { "●" } else { "○" };
            let color = if provider.configured {
                Theme::SUCCESS
            } else {
                Theme::TEXT_DIM
            };
            let style = if i == state.selected_idx {
                Theme::selected()
            } else {
                Style::default().fg(Theme::TEXT_PRIMARY)
            };
            let line = Line::from(vec![
                Span::styled(format!(" {marker} "), Style::default().fg(color)),
                Span::styled(&provider.name, style),
                Span::styled(format!("  {}", provider.description), Theme::dim()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::border())
            .title(Span::styled(
                format!(" providers ({}) ", filtered.len()),
                Theme::subtitle(),
            )),
    );
    f.render_widget(list, chunks[3]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Theme::GOLD)),
        Span::styled(" navigate  ", Theme::dim()),
        Span::styled("enter", Style::default().fg(Theme::GOLD)),
        Span::styled(" select  ", Theme::dim()),
        Span::styled("esc", Style::default().fg(Theme::GOLD)),
        Span::styled(" skip", Theme::dim()),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(help, chunks[4]);
}
