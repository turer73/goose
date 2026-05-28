use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::tui::markdown::render_markdown;
use crate::tui::theme::Theme;

#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ToolCallDisplay {
    pub title: String,
    pub id: String,
    pub status: String,
}

pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub tool_calls: Vec<ToolCallDisplay>,
    pub input: String,
    pub input_cursor: usize,
    pub streaming_text: String,
    pub is_streaming: bool,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            tool_calls: Vec::new(),
            input: String::new(),
            input_cursor: 0,
            streaming_text: String::new(),
            is_streaming: false,
        }
    }

    pub fn push_user_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content,
        });
    }

    pub fn finalize_stream(&mut self) {
        if !self.streaming_text.is_empty() {
            self.messages.push(ChatMessage {
                role: MessageRole::Assistant,
                content: std::mem::take(&mut self.streaming_text),
            });
        }
        self.is_streaming = false;
        self.tool_calls.clear();
    }

    pub fn input_insert(&mut self, ch: char) {
        self.input.insert(self.input_cursor, ch);
        self.input_cursor += ch.len_utf8();
    }

    #[allow(clippy::string_slice)]
    pub fn input_backspace(&mut self) {
        if self.input_cursor > 0 {
            let prev = self.input[..self.input_cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.input_cursor -= prev;
            self.input.remove(self.input_cursor);
        }
    }

    pub fn input_delete(&mut self) {
        if self.input_cursor < self.input.len() {
            self.input.remove(self.input_cursor);
        }
    }

    #[allow(clippy::string_slice)]
    pub fn input_left(&mut self) {
        if self.input_cursor > 0 {
            let prev = self.input[..self.input_cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.input_cursor -= prev;
        }
    }

    #[allow(clippy::string_slice)]
    pub fn input_right(&mut self) {
        if self.input_cursor < self.input.len() {
            let next = self.input[self.input_cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.input_cursor += next;
        }
    }

    pub fn input_home(&mut self) {
        self.input_cursor = 0;
    }

    pub fn input_end(&mut self) {
        self.input_cursor = self.input.len();
    }

    pub fn take_input(&mut self) -> String {
        self.input_cursor = 0;
        std::mem::take(&mut self.input)
    }
}

pub fn render_chat(f: &mut Frame, area: Rect, state: &ChatState, tick: u64) {
    // Fill background
    let bg = Block::default().style(Style::default().bg(Theme::BG_PRIMARY));
    f.render_widget(bg, area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // top padding
        Constraint::Length(1), // status bar
        Constraint::Length(1), // separator under status
        Constraint::Min(4),   // messages
        Constraint::Length(1), // separator above input
        Constraint::Length(3), // input
        Constraint::Length(1), // help bar
    ])
    .split(area);

    render_status_bar(f, chunks[1], state, tick);
    render_separator(f, chunks[2]);
    render_messages(f, chunks[3], state, tick);
    render_separator(f, chunks[4]);
    render_input(f, chunks[5], state);
    render_help_bar(f, chunks[6]);
}

fn render_status_bar(f: &mut Frame, area: Rect, state: &ChatState, tick: u64) {
    let spans = if state.is_streaming {
        let frames = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
        let s = frames[(tick as usize) % frames.len()];
        vec![
            Span::styled(format!(" {s} "), Style::default().fg(Theme::TEAL)),
            Span::styled(
                "generating…",
                Style::default().fg(Theme::TEXT_SECONDARY),
            ),
        ]
    } else {
        let msg_count = state.messages.len();
        let count_text = if msg_count == 1 {
            "1 message".to_string()
        } else {
            format!("{msg_count} messages")
        };
        vec![
            Span::styled(" ◆ ", Style::default().fg(Theme::CRANBERRY)),
            Span::styled(
                "goose",
                Style::default()
                    .fg(Theme::TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  ", Style::default().fg(Theme::BORDER)),
            Span::styled(count_text, Style::default().fg(Theme::TEXT_DIM)),
        ]
    };

    let bar = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Theme::BG_SURFACE));
    f.render_widget(bar, area);
}

fn render_separator(f: &mut Frame, area: Rect) {
    let w = area.width as usize;
    let line_str: String = "─".repeat(w);
    let sep = Paragraph::new(Line::from(Span::styled(
        line_str,
        Style::default().fg(Theme::BORDER),
    )));
    f.render_widget(sep, area);
}

fn render_messages(f: &mut Frame, area: Rect, state: &ChatState, tick: u64) {
    let content_width = (area.width as usize).saturating_sub(8);
    let mut lines: Vec<Line<'static>> = Vec::new();

    for msg in &state.messages {
        match msg.role {
            MessageRole::User => {
                if !lines.is_empty() {
                    lines.push(Line::from(""));
                }
                lines.push(Line::from(vec![
                    Span::styled("  ▸ ", Style::default().fg(Theme::GOLD)),
                    Span::styled(
                        "you",
                        Style::default()
                            .fg(Theme::GOLD)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(""));
                for l in render_markdown(&msg.content, content_width) {
                    let mut padded = vec![Span::raw("    ")];
                    padded.extend(l.spans);
                    lines.push(Line::from(padded));
                }
                lines.push(Line::from(""));
            }
            MessageRole::Assistant => {
                if !lines.is_empty() {
                    lines.push(Line::from(""));
                }
                lines.push(Line::from(vec![
                    Span::styled("  ◆ ", Style::default().fg(Theme::CRANBERRY)),
                    Span::styled(
                        "goose",
                        Style::default()
                            .fg(Theme::CRANBERRY)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(""));
                for l in render_markdown(&msg.content, content_width) {
                    let mut padded = vec![Span::raw("    ")];
                    padded.extend(l.spans);
                    lines.push(Line::from(padded));
                }
                lines.push(Line::from(""));
            }
            MessageRole::System => {
                if !lines.is_empty() {
                    lines.push(Line::from(""));
                }
                let wrapped = word_wrap(&msg.content, content_width);
                for (i, text) in wrapped.iter().enumerate() {
                    let prefix = if i == 0 {
                        "  ℹ  "
                    } else {
                        "     "
                    };
                    lines.push(Line::from(vec![
                        Span::styled(
                            prefix.to_string(),
                            Style::default().fg(Theme::TEAL),
                        ),
                        Span::styled(
                            text.clone(),
                            Style::default().fg(Theme::TEXT_DIM),
                        ),
                    ]));
                }
            }
        }
    }

    // Streaming content
    if state.is_streaming && !state.streaming_text.is_empty() {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(vec![
            Span::styled("  ◆ ", Style::default().fg(Theme::CRANBERRY)),
            Span::styled(
                "goose",
                Style::default()
                    .fg(Theme::CRANBERRY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
        for l in render_markdown(&state.streaming_text, content_width) {
            let mut padded = vec![Span::raw("    ")];
            padded.extend(l.spans);
            lines.push(Line::from(padded));
        }
    }

    // Tool calls
    for tc in &state.tool_calls {
        let frames = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
        let s = frames[(tick as usize) % frames.len()];
        lines.push(Line::from(vec![
            Span::styled(
                format!("    {s} "),
                Style::default().fg(Theme::TEAL),
            ),
            Span::styled(
                tc.title.clone(),
                Style::default().fg(Theme::TEXT_SECONDARY),
            ),
            Span::styled(
                format!("  {}", tc.status),
                Style::default().fg(Theme::TEXT_DIM),
            ),
        ]));
    }

    // Auto-scroll
    let visible = area.height as usize;
    let total = lines.len();
    let scroll = if total > visible {
        (total - visible) as u16
    } else {
        0
    };

    let widget = Paragraph::new(lines)
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(widget, area);
}

fn render_input(f: &mut Frame, area: Rect, state: &ChatState) {
    let border_style = if state.is_streaming {
        Style::default().fg(Theme::BORDER)
    } else {
        Style::default().fg(Theme::BORDER_ACTIVE)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            " message ",
            Style::default()
                .fg(Theme::TEAL)
                .add_modifier(Modifier::BOLD),
        ));

    let text = if state.input.is_empty() && !state.is_streaming {
        Span::styled(
            "Type a message or /help for commands…",
            Style::default().fg(Theme::TEXT_DIM),
        )
    } else {
        Span::styled(
            state.input.clone(),
            Style::default().fg(Theme::TEXT_PRIMARY),
        )
    };

    let widget = Paragraph::new(Line::from(text)).block(block);
    f.render_widget(widget, area);

    if !state.is_streaming {
        let cursor_x = area.x + 1 + state.input_cursor as u16;
        let cursor_y = area.y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_help_bar(f: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" /help ", Style::default().fg(Theme::GOLD)),
        Span::styled("commands", Style::default().fg(Theme::TEXT_DIM)),
        Span::styled("  │  ", Style::default().fg(Theme::BORDER)),
        Span::styled("ctrl+c ", Style::default().fg(Theme::GOLD)),
        Span::styled("quit", Style::default().fg(Theme::TEXT_DIM)),
        Span::styled("  │  ", Style::default().fg(Theme::BORDER)),
        Span::styled("ctrl+l ", Style::default().fg(Theme::GOLD)),
        Span::styled("clear", Style::default().fg(Theme::TEXT_DIM)),
    ]))
    .style(Style::default().bg(Theme::BG_SURFACE));
    f.render_widget(help, area);
}

fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut result = Vec::new();
    let mut line = String::new();
    let mut line_len = 0;

    for word in text.split_whitespace() {
        let wlen = word.len();
        if line_len == 0 {
            line = word.to_string();
            line_len = wlen;
        } else if line_len + 1 + wlen <= width {
            line.push(' ');
            line.push_str(word);
            line_len += 1 + wlen;
        } else {
            result.push(std::mem::take(&mut line));
            line = word.to_string();
            line_len = wlen;
        }
    }
    if !line.is_empty() {
        result.push(line);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}
