use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::theme::Theme;

pub fn render_markdown(text: &str, width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lines: Vec<String> = Vec::new();

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                for code_line in &code_block_lines {
                    let truncated = truncate_str(code_line, width.saturating_sub(2));
                    lines.push(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(Theme::BORDER)),
                        Span::styled(
                            truncated,
                            Style::default()
                                .fg(Theme::TEXT_SECONDARY)
                                .bg(Theme::BG_ELEVATED),
                        ),
                    ]));
                }
                code_block_lines.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_block_lines.push(line.to_string());
            continue;
        }

        if let Some(content) = line.strip_prefix("### ") {
            lines.push(Line::from(Span::styled(
                truncate_str(content, width),
                Style::default()
                    .fg(Theme::TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if let Some(content) = line.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                truncate_str(content, width),
                Style::default()
                    .fg(Theme::GOLD)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if let Some(content) = line.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                truncate_str(content, width),
                Style::default()
                    .fg(Theme::CRANBERRY)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }

        if let Some(content) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            let spans = parse_inline_markdown(content);
            let mut all_spans = vec![Span::styled("  • ", Style::default().fg(Theme::TEAL))];
            all_spans.extend(spans);
            lines.push(Line::from(all_spans));
            continue;
        }

        // Numbered list items
        if let Some(dot_pos) = line.find(". ") {
            if dot_pos <= 3 && line.chars().take(dot_pos).all(|c| c.is_ascii_digit()) {
                let prefix: String = line.chars().take(dot_pos + 1).collect();
                let content: String = line.chars().skip(dot_pos + 2).collect();
                let spans = parse_inline_markdown(&content);
                let mut all_spans = vec![Span::styled(
                    format!("  {prefix} "),
                    Style::default().fg(Theme::TEAL),
                )];
                all_spans.extend(spans);
                lines.push(Line::from(all_spans));
                continue;
            }
        }

        if line.is_empty() {
            lines.push(Line::from(""));
            continue;
        }

        let wrapped = wrap_text(line, width);
        for wrapped_line in wrapped {
            let spans = parse_inline_markdown(&wrapped_line);
            lines.push(Line::from(spans));
        }
    }

    if in_code_block {
        for code_line in &code_block_lines {
            let truncated = truncate_str(code_line, width.saturating_sub(2));
            lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(Theme::BORDER)),
                Span::styled(
                    truncated,
                    Style::default()
                        .fg(Theme::TEXT_SECONDARY)
                        .bg(Theme::BG_ELEVATED),
                ),
            ]));
        }
    }

    lines
}

fn parse_inline_markdown(text: &str) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '`' => {
                if !current.is_empty() {
                    spans.push(Span::styled(
                        std::mem::take(&mut current),
                        Style::default().fg(Theme::TEXT_PRIMARY),
                    ));
                }
                let mut code = String::new();
                for c in chars.by_ref() {
                    if c == '`' {
                        break;
                    }
                    code.push(c);
                }
                spans.push(Span::styled(
                    code,
                    Style::default()
                        .fg(Color::Rgb(200, 180, 140))
                        .bg(Theme::BG_ELEVATED),
                ));
            }
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                if !current.is_empty() {
                    spans.push(Span::styled(
                        std::mem::take(&mut current),
                        Style::default().fg(Theme::TEXT_PRIMARY),
                    ));
                }
                let mut bold = String::new();
                loop {
                    match chars.next() {
                        Some('*') if chars.peek() == Some(&'*') => {
                            chars.next();
                            break;
                        }
                        Some(c) => bold.push(c),
                        None => break,
                    }
                }
                spans.push(Span::styled(
                    bold,
                    Style::default()
                        .fg(Theme::TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            '*' | '_' => {
                if !current.is_empty() {
                    spans.push(Span::styled(
                        std::mem::take(&mut current),
                        Style::default().fg(Theme::TEXT_PRIMARY),
                    ));
                }
                let delimiter = ch;
                let mut italic = String::new();
                for c in chars.by_ref() {
                    if c == delimiter {
                        break;
                    }
                    italic.push(c);
                }
                spans.push(Span::styled(
                    italic,
                    Style::default()
                        .fg(Theme::TEXT_SECONDARY)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
            '[' => {
                if !current.is_empty() {
                    spans.push(Span::styled(
                        std::mem::take(&mut current),
                        Style::default().fg(Theme::TEXT_PRIMARY),
                    ));
                }
                let mut link_text = String::new();
                let mut found_close = false;
                for c in chars.by_ref() {
                    if c == ']' {
                        found_close = true;
                        break;
                    }
                    link_text.push(c);
                }
                if found_close && chars.peek() == Some(&'(') {
                    chars.next();
                    for c in chars.by_ref() {
                        if c == ')' {
                            break;
                        }
                    }
                }
                spans.push(Span::styled(
                    link_text,
                    Style::default()
                        .fg(Theme::INFO)
                        .add_modifier(Modifier::UNDERLINED),
                ));
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        spans.push(Span::styled(
            current,
            Style::default().fg(Theme::TEXT_PRIMARY),
        ));
    }

    if spans.is_empty() {
        spans.push(Span::raw(""));
    }

    spans
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_len = word.len();
        if current_width == 0 {
            current_line = word.to_string();
            current_width = word_len;
        } else if current_width + 1 + word_len <= width {
            current_line.push(' ');
            current_line.push_str(word);
            current_width += 1 + word_len;
        } else {
            lines.push(std::mem::take(&mut current_line));
            current_line = word.to_string();
            current_width = word_len;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn truncate_str(s: &str, max_width: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_width {
        s.to_string()
    } else if max_width > 1 {
        let truncated: String = s.chars().take(max_width - 1).collect();
        format!("{truncated}…")
    } else {
        "…".to_string()
    }
}
