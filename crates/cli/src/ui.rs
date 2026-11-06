use ratatui::{
    prelude::*,
    widgets::*,
    style::{Color, Modifier, Style},
};

use crate::app::{App, MessageRole, COMMANDS};

const ACCENT: Color = Color::Rgb(163, 230, 53);
const MUTED:  Color = Color::DarkGray;
const TEXT:   Color = Color::White;
const DIM:    Color = Color::Rgb(100, 100, 100);

// Height of the welcome box (border + content rows)
const WELCOME_HEIGHT: u16 = 14;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let welcome_height = if app.welcome_dismissed { 0 } else { WELCOME_HEIGHT };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),              // header bar
            Constraint::Length(welcome_height), // welcome box (gone after first message)
            Constraint::Min(1),                 // conversation
            Constraint::Length(3),              // input box
            Constraint::Length(1),              // key hints
        ])
        .split(area);

    render_header(frame, layout[0], app);
    if !app.welcome_dismissed {
        render_welcome(frame, layout[1]);
    }
    render_conversation(frame, app, layout[2]);
    render_input(frame, app, layout[3]);
    render_command_palette(frame, app, layout[3]);
    render_hints(frame, app, layout[4]);

    if app.confirm_quit {
        render_quit_confirm(frame, area);
    }

    // Cursor: +1 for left border, then N chars of input text
    frame.set_cursor_position((
        layout[3].x + app.input.chars().count() as u16 + 1,
        layout[3].y + 1,
    ));
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let left = Line::from(vec![
        Span::styled(" ✦ Friday", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("  v0.1.0", Style::default().fg(DIM)),
    ]);

    let right_text = format!("gpt-4o-mini  ◆ {}  ", app.mode.label());
    let right = Paragraph::new(Line::from(Span::styled(right_text, Style::default().fg(MUTED))))
        .alignment(Alignment::Right);

    frame.render_widget(Paragraph::new(left), area);
    frame.render_widget(right, area);
}

fn render_welcome(frame: &mut Frame, area: Rect) {
    let box_width = area.width.min(90);
    let box_area = Rect { x: area.x, y: area.y, width: box_width, height: area.height };

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .title(Line::from(vec![
            Span::styled(" Friday ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("v0.1.0 ", Style::default().fg(MUTED)),
        ]));

    let inner = outer.inner(box_area);
    frame.render_widget(outer, box_area);

    // Split into left (mascot) and right (tips) with a divider
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(1),
            Constraint::Min(20),
        ])
        .split(inner);

    // Vertical divider
    let rule: Vec<Line> = (0..cols[1].height)
        .map(|_| Line::from(Span::styled("│", Style::default().fg(MUTED))))
        .collect();
    frame.render_widget(Paragraph::new(rule), cols[1]);

    render_welcome_left(frame, cols[0]);
    render_welcome_right(frame, cols[2]);
}

fn render_welcome_left(frame: &mut Frame, area: Rect) {
    const MASCOT: [&str; 6] = [
        "  ▄▄  ▄▄  ",
        " ████████ ",
        " █ ▄  ▄ █ ",
        " ████████ ",
        "  █    █  ",
        "  █    █  ",
    ];

    let mut content: Vec<Line> = vec![
        Line::from(Span::styled(
            "Welcome back!",
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for row in MASCOT {
        content.push(Line::from(Span::styled(row, Style::default().fg(ACCENT))));
    }
    content.push(Line::from(""));
    content.push(Line::from(Span::styled("Friday CLI  ·  gpt-4o-mini", Style::default().fg(MUTED))));

    let top_pad = area.height.saturating_sub(content.len() as u16) / 2;
    let mut padded: Vec<Line> = vec![Line::from(""); top_pad as usize];
    padded.extend(content);

    frame.render_widget(
        Paragraph::new(padded).alignment(Alignment::Center),
        area,
    );
}

fn render_welcome_right(frame: &mut Frame, area: Rect) {
    let rule_width = area.width.saturating_sub(2) as usize;
    let lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Tips for getting started",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("Type / to browse commands", Style::default().fg(TEXT))),
        Line::from(Span::styled("Shift+Tab to switch modes", Style::default().fg(TEXT))),
        Line::from(Span::styled("Esc to clear input or quit", Style::default().fg(TEXT))),
        Line::from(""),
        Line::from(Span::styled("─".repeat(rule_width), Style::default().fg(MUTED))),
        Line::from(""),
        Line::from(Span::styled(
            "Modes",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("chat · command · agent", Style::default().fg(MUTED))),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().padding(Padding::new(1, 1, 0, 0))),
        area,
    );
}

fn render_conversation(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = vec![];

    for message in &app.messages {
        match message.role {
            MessageRole::User => {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "● You",
                    Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
                )));
                for content_line in message.content.lines() {
                    lines.push(Line::from(content_line.to_string()));
                }
            }

            MessageRole::Assistant => {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "✦ Friday",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                )));
                lines.extend(crate::markdown::to_lines(&message.content));
            }

            MessageRole::System => {
                for content_line in message.content.lines() {
                    lines.push(Line::from(Span::styled(
                        content_line.to_string(),
                        Style::default().fg(MUTED),
                    )));
                }
            }
        }
    }

    if let Some(streaming) = &app.streaming_message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "✦ Friday",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )));

        let mut md_lines = crate::markdown::to_lines(streaming);

        // Append the streaming cursor to the last line, or add it on its own
        let cursor = Span::styled("▋", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));
        if let Some(last) = md_lines.last_mut() {
            last.spans.push(cursor);
        } else {
            md_lines.push(Line::from(cursor));
        }

        lines.extend(md_lines);
    }

    // Count total visual rows after wrapping so Paragraph::scroll works correctly.
    // Manual slicing + Wrap causes row/line mismatch; this avoids that entirely.
    let col_width = area.width.max(1);
    let total_visual: u16 = lines.iter().map(|l| {
        let w = l.width() as u16;
        if w == 0 { 1 } else { (w + col_width - 1) / col_width }
    }).sum();

    let max_scroll = total_visual.saturating_sub(area.height);
    let scroll_up = (app.scroll_offset as u16).min(max_scroll);
    let scroll_from_top = max_scroll.saturating_sub(scroll_up);

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll_from_top, 0)),
        area,
    );
}

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let mode_label = app.mode.label();
    let placeholder = app.mode.placeholder();

    let input_text = if app.input.is_empty() {
        Line::from(Span::styled(placeholder, Style::default().fg(MUTED)))
    } else {
        Line::from(Span::styled(app.input.clone(), Style::default().fg(TEXT)))
    };

    let border_color = if !app.input.is_empty() { ACCENT } else { MUTED };

    let title = Line::from(vec![
        Span::styled(" [", Style::default().fg(DIM)),
        Span::styled(mode_label, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("] ", Style::default().fg(DIM)),
    ]);

    frame.render_widget(
        Paragraph::new(input_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(title),
        ),
        area,
    );
}

fn render_command_palette(frame: &mut Frame, app: &App, input_area: Rect) {
    if app.filtered_commands.is_empty() {
        return;
    }

    const NAME_COL: usize = 13;

    let suggestions: Vec<Line> = app
        .filtered_commands
        .iter()
        .enumerate()
        .map(|(i, &cmd_idx)| {
            let cmd = &COMMANDS[cmd_idx];
            let is_selected = i == app.selected_command;
            let name_padded = format!("{:<width$}", cmd.name, width = NAME_COL);

            if is_selected {
                Line::from(vec![
                    Span::styled("› ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                    Span::styled(name_padded, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                    Span::styled(cmd.description, Style::default().fg(TEXT)),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(name_padded, Style::default().fg(TEXT)),
                    Span::styled(cmd.description, Style::default().fg(MUTED)),
                ])
            }
        })
        .collect();

    let popup_height = (app.filtered_commands.len().min(8) as u16) + 2;
    let popup_y = input_area.y.saturating_sub(popup_height);
    let popup_width = 62u16.min(input_area.width.saturating_sub(2));

    frame.render_widget(
        Paragraph::new(suggestions).block(
            Block::default()
                .title(Span::styled(" friday commands ", Style::default().fg(MUTED)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(MUTED)),
        ),
        Rect { x: input_area.x + 1, y: popup_y, width: popup_width, height: popup_height },
    );
}

fn render_hints(frame: &mut Frame, app: &App, area: Rect) {
    let scroll_indicator = if app.scroll_offset > 0 {
        format!(" ↑{}  ", app.scroll_offset)
    } else {
        String::new()
    };

    let hints = Line::from(vec![
        Span::styled(scroll_indicator, Style::default().fg(ACCENT)),
        Span::styled(" [", Style::default().fg(DIM)),
        Span::styled("shift+tab", Style::default().fg(MUTED)),
        Span::styled("] ", Style::default().fg(DIM)),
        Span::styled("mode", Style::default().fg(DIM)),
        Span::styled("   [", Style::default().fg(DIM)),
        Span::styled("↑↓/pgup/pgdn", Style::default().fg(MUTED)),
        Span::styled("] ", Style::default().fg(DIM)),
        Span::styled("scroll", Style::default().fg(DIM)),
        Span::styled("   [", Style::default().fg(DIM)),
        Span::styled("esc", Style::default().fg(MUTED)),
        Span::styled("] ", Style::default().fg(DIM)),
        Span::styled("clear/quit", Style::default().fg(DIM)),
    ]);

    frame.render_widget(Paragraph::new(hints), area);
}

fn render_quit_confirm(frame: &mut Frame, area: Rect) {
    const W: u16 = 36;
    const H: u16 = 3;
    let x = area.x + (area.width.saturating_sub(W)) / 2;
    let y = area.y + (area.height.saturating_sub(H)) / 2;
    let popup = Rect { x, y, width: W, height: H };

    let content = Line::from(vec![
        Span::styled("Quit Friday? ", Style::default().fg(TEXT)),
        Span::styled("[y]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(" yes  ", Style::default().fg(MUTED)),
        Span::styled("[esc]", Style::default().fg(MUTED)),
        Span::styled(" cancel", Style::default().fg(MUTED)),
    ]);

    frame.render_widget(
        Paragraph::new(content)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(ACCENT)),
            ),
        popup,
    );
}
