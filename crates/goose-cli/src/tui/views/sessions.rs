use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::tui::acp_client::SessionInfo;
use crate::tui::theme::Theme;

pub struct SessionsState {
    pub sessions: Vec<SessionInfo>,
    pub selected_idx: usize,
}

impl SessionsState {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            selected_idx: 0,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.sessions.is_empty() && self.selected_idx < self.sessions.len() - 1 {
            self.selected_idx += 1;
        }
    }
}

pub fn render_sessions(f: &mut Frame, area: Rect, state: &SessionsState) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .split(area);

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled("◆ ", Style::default().fg(Theme::TEAL)),
        Span::styled("Sessions", Theme::title()),
        Span::styled(format!("  ({} total)", state.sessions.len()), Theme::dim()),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Session list
    if state.sessions.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No sessions yet. Start chatting!",
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
            .sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let style = if i == state.selected_idx {
                    Theme::selected()
                } else {
                    Style::default().fg(Theme::TEXT_PRIMARY)
                };
                let title_text = if session.title.is_empty() {
                    "Untitled Session"
                } else {
                    &session.title
                };
                let line = Line::from(vec![
                    Span::styled(
                        if i == state.selected_idx {
                            " ▸ "
                        } else {
                            "   "
                        },
                        Style::default().fg(Theme::CRANBERRY),
                    ),
                    Span::styled(title_text, style),
                    Span::styled(format!("  {}", &session.updated_at), Theme::dim()),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Theme::border())
                .title(Span::styled(" recent sessions ", Theme::subtitle())),
        );
        f.render_widget(list, chunks[1]);
    }

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Theme::GOLD)),
        Span::styled(" navigate  ", Theme::dim()),
        Span::styled("enter", Style::default().fg(Theme::GOLD)),
        Span::styled(" resume  ", Theme::dim()),
        Span::styled("n", Style::default().fg(Theme::GOLD)),
        Span::styled(" new session  ", Theme::dim()),
        Span::styled("esc", Style::default().fg(Theme::GOLD)),
        Span::styled(" back", Theme::dim()),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}
