use ratatui::{
    prelude::*,
    widgets::*,
    style::{Color, Modifier, Style},
};

use crate::app::{App, MessageRole, COMMANDS};
use crate::models::{MODELS, unique_providers};

const ACCENT: Color = Color::Rgb(230, 170, 80);
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

    if app.model_picker_open {
        render_model_picker(frame, app, layout[3]);
    }

    if app.keys_panel_open {
        render_keys_panel(frame, app, layout[3]);
    }

    if app.key_input_open {
        render_key_input(frame, app, layout[3]);
    }

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

    let model_id = app.current_model.read().unwrap().clone();
    let model_label = MODELS.iter()
        .find(|m| m.id == model_id)
        .map(|m| m.label)
        .unwrap_or(&model_id);
    let right_text = format!("{}  ◆ {}  ", model_label, app.mode.label());
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

    // Pre-split every logical Line into single-visual-row lines (≤ col_width chars).
    // This gives an exact row count — no estimation, no Paragraph::wrap mismatch.
    let col_width = area.width.max(1) as usize;
    let flat = split_into_rows(lines, col_width);

    let total   = flat.len();
    let visible = area.height as usize;
    let max_scroll = total.saturating_sub(visible);
    let scroll_up  = app.scroll_offset.min(max_scroll);

    let start = max_scroll.saturating_sub(scroll_up);
    let end   = (start + visible).min(total);

    // No Wrap needed — each line already fits in col_width.
    frame.render_widget(Paragraph::new(flat[start..end].to_vec()), area);
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
    let popup_rect = Rect { x: input_area.x + 1, y: popup_y, width: popup_width, height: popup_height };

    frame.render_widget(Clear, popup_rect);
    frame.render_widget(
        Paragraph::new(suggestions).block(
            Block::default()
                .title(Span::styled(" friday commands ", Style::default().fg(MUTED)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(MUTED))
                .style(Style::default().bg(Color::Rgb(16, 16, 16))),
        ),
        popup_rect,
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

fn render_model_picker(frame: &mut Frame, app: &App, input_area: Rect) {
    const ID_COL: usize = 28;

    let current_id = app.current_model.read().unwrap().clone();
    let mut rows: Vec<Line> = vec![];
    let mut last_provider = "";
    let mut selected_visual_row: u16 = 0;

    // ── Cloud models ──────────────────────────────────────────────────────────
    for (i, m) in MODELS.iter().enumerate() {
        if m.provider != last_provider {
            if !rows.is_empty() { rows.push(Line::from("")); }
            rows.push(Line::from(Span::styled(
                format!("  {}", m.provider),
                Style::default().fg(DIM).add_modifier(Modifier::BOLD),
            )));
            last_provider = m.provider;
        }

        if i == app.model_picker_selected {
            selected_visual_row = rows.len() as u16;
        }

        let is_selected = i == app.model_picker_selected;
        let is_active   = m.id == current_id;
        let marker      = if is_active { "✦ " } else if is_selected { "› " } else { "  " };
        let id_padded   = format!("{:<width$}", m.id,    width = ID_COL);
        let lbl_padded  = format!("{:<10}", m.label);

        rows.push(if is_selected {
            Line::from(vec![
                Span::styled(marker,     Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(id_padded,  Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(lbl_padded, Style::default().fg(TEXT)),
            ])
        } else if is_active {
            Line::from(vec![
                Span::styled(marker,     Style::default().fg(ACCENT)),
                Span::styled(id_padded,  Style::default().fg(TEXT)),
                Span::styled(lbl_padded, Style::default().fg(MUTED)),
            ])
        } else {
            Line::from(vec![
                Span::raw(marker),
                Span::styled(id_padded,  Style::default().fg(MUTED)),
                Span::styled(lbl_padded, Style::default().fg(DIM)),
            ])
        });
    }

    // ── Local Ollama models ───────────────────────────────────────────────────
    rows.push(Line::from(""));
    if !app.local_models.is_empty() {
        rows.push(Line::from(Span::styled(
            "  Local (Ollama)",
            Style::default().fg(DIM).add_modifier(Modifier::BOLD),
        )));
        for (j, name) in app.local_models.iter().enumerate() {
            let idx = MODELS.len() + j;

            if idx == app.model_picker_selected {
                selected_visual_row = rows.len() as u16;
            }

            let is_selected = idx == app.model_picker_selected;
            let is_active   = *name == current_id;
            let marker      = if is_active { "✦ " } else if is_selected { "› " } else { "  " };
            let id_padded   = format!("{:<width$}", name, width = ID_COL);

            rows.push(if is_selected {
                Line::from(vec![
                    Span::styled(marker,    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                    Span::styled(id_padded, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                    Span::styled("local",   Style::default().fg(TEXT)),
                ])
            } else if is_active {
                Line::from(vec![
                    Span::styled(marker,    Style::default().fg(ACCENT)),
                    Span::styled(id_padded, Style::default().fg(TEXT)),
                    Span::styled("local",   Style::default().fg(MUTED)),
                ])
            } else {
                Line::from(vec![
                    Span::raw(marker),
                    Span::styled(id_padded, Style::default().fg(MUTED)),
                    Span::styled("local",   Style::default().fg(DIM)),
                ])
            });
        }
    } else {
        rows.push(Line::from(Span::styled(
            "  Local (Ollama not detected)",
            Style::default().fg(DIM),
        )));
    }

    // Cap popup to available space above the input box (minus 2 for borders)
    let available    = input_area.y.saturating_sub(2);
    let visible_rows = (available as usize).min(rows.len());
    let popup_height = visible_rows as u16 + 2;
    let popup_y      = input_area.y.saturating_sub(popup_height);
    let popup_width  = 58u16.min(input_area.width.saturating_sub(2));
    let popup_rect   = Rect { x: input_area.x + 1, y: popup_y, width: popup_width, height: popup_height };

    // Scroll so the selected row is always visible
    let scroll: u16 = if selected_visual_row >= visible_rows as u16 {
        selected_visual_row.saturating_sub(visible_rows as u16 - 1)
    } else {
        0
    };

    frame.render_widget(Clear, popup_rect);
    frame.render_widget(
        Paragraph::new(rows)
            .scroll((scroll, 0))
            .block(
                Block::default()
                    .title(Span::styled(" switch model ", Style::default().fg(MUTED)))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(ACCENT))
                    .style(Style::default().bg(Color::Rgb(16, 16, 16))),
            ),
        popup_rect,
    );
}

/// Split logical Lines into single-visual-row Lines (at most `col_width` chars each).
/// We split at character boundaries rather than word boundaries so the row count
/// is exact — no dependency on ratatui's WordWrapper behaviour.
fn split_into_rows(lines: Vec<Line<'static>>, col_width: usize) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = Vec::with_capacity(lines.len() + 8);
    for line in lines {
        if line.width() <= col_width {
            out.push(line);
        } else {
            let mut row: Vec<Span<'static>> = Vec::new();
            let mut col = 0usize;

            for span in line.spans {
                let style = span.style;
                let mut s = span.content.into_owned();

                while !s.is_empty() {
                    let space = col_width.saturating_sub(col);
                    if space == 0 {
                        out.push(Line::from(std::mem::take(&mut row)));
                        col = 0;
                        continue;
                    }
                    let (head, tail) = take_chars(s, space);
                    col += head.chars().count();
                    if !head.is_empty() {
                        row.push(Span::styled(head, style));
                    }
                    if col >= col_width {
                        out.push(Line::from(std::mem::take(&mut row)));
                        col = 0;
                    }
                    s = tail;
                }
            }
            out.push(Line::from(row));
        }
    }
    out
}

/// Take the first `n` chars of `s`, returning (head, tail) as owned Strings.
fn take_chars(s: String, n: usize) -> (String, String) {
    let idx = s.char_indices().nth(n).map(|(i, _)| i).unwrap_or(s.len());
    let tail = s[idx..].to_string();
    let mut head = s;
    head.truncate(idx);
    (head, tail)
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

/// /keys panel — lists all providers with key status; Enter opens the key-input overlay.
fn render_keys_panel(frame: &mut Frame, app: &App, input_area: Rect) {
    let cfg = config::Config::load().unwrap_or_default();
    let providers = unique_providers();

    let mut rows: Vec<Line> = vec![];

    for (i, p) in providers.iter().enumerate() {
        let is_selected = i == app.keys_panel_selected;
        let stored = cfg.key_for_provider(p.provider);

        let (indicator, key_preview) = match &stored {
            Some(k) => {
                let tail = if k.len() > 4 { &k[k.len() - 4..] } else { k };
                let dots = "●";
                (dots, format!("···{}", tail))
            }
            None => ("○", "not set".to_string()),
        };

        let ind_style = if stored.is_some() {
            Style::default().fg(ACCENT)
        } else {
            Style::default().fg(DIM)
        };
        let key_style = if stored.is_some() {
            Style::default().fg(MUTED)
        } else {
            Style::default().fg(DIM)
        };

        let marker = if is_selected { "› " } else { "  " };
        let name_padded = format!("{:<12}", p.provider);
        let ind_padded  = format!("{:<2}", indicator);

        let row = if is_selected {
            Line::from(vec![
                Span::styled(marker,       Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(name_padded,  Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(ind_padded,   ind_style.add_modifier(Modifier::BOLD)),
                Span::styled(key_preview,  Style::default().fg(TEXT)),
            ])
        } else {
            Line::from(vec![
                Span::raw(marker),
                Span::styled(name_padded, Style::default().fg(MUTED)),
                Span::styled(ind_padded,  ind_style),
                Span::styled(key_preview, key_style),
            ])
        };
        rows.push(row);
    }

    let popup_height = (providers.len() as u16) + 2;
    let popup_y      = input_area.y.saturating_sub(popup_height);
    let popup_width  = 46u16.min(input_area.width.saturating_sub(2));
    let popup_rect   = Rect { x: input_area.x + 1, y: popup_y, width: popup_width, height: popup_height };

    frame.render_widget(Clear, popup_rect);
    frame.render_widget(
        Paragraph::new(rows).block(
            Block::default()
                .title(Span::styled(" manage API keys ", Style::default().fg(MUTED)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ACCENT))
                .style(Style::default().bg(Color::Rgb(16, 16, 16))),
        ),
        popup_rect,
    );
}

/// Inline API-key input overlay — shown when switching to a provider with no stored key,
/// or when editing a key from the /keys panel.
fn render_key_input(frame: &mut Frame, app: &App, input_area: Rect) {
    let title = format!(" API key for {} ", app.key_input_provider);
    let hint   = format!("or set {} env var", app.key_input_env_hint);

    // Mask all but the last 4 typed characters
    let display = if app.key_input_value.is_empty() {
        String::new()
    } else {
        let chars: Vec<char> = app.key_input_value.chars().collect();
        let visible = 4.min(chars.len());
        let masked  = chars.len().saturating_sub(visible);
        format!("{}{}", "•".repeat(masked), &chars[masked..].iter().collect::<String>())
    };

    let content = vec![
        Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(&display, Style::default().fg(ACCENT)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {} ", hint), Style::default().fg(DIM)),
        ]),
        Line::from(vec![
            Span::styled("  Enter ", Style::default().fg(MUTED)),
            Span::styled("save · ", Style::default().fg(DIM)),
            Span::styled("Esc ", Style::default().fg(MUTED)),
            Span::styled("cancel", Style::default().fg(DIM)),
        ]),
    ];

    let popup_height = 6u16;
    let popup_y      = input_area.y.saturating_sub(popup_height);
    let popup_width  = 54u16.min(input_area.width.saturating_sub(2));
    let popup_rect   = Rect { x: input_area.x + 1, y: popup_y, width: popup_width, height: popup_height };

    frame.render_widget(Clear, popup_rect);
    frame.render_widget(
        Paragraph::new(content).block(
            Block::default()
                .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ACCENT))
                .style(Style::default().bg(Color::Rgb(16, 16, 16))),
        ),
        popup_rect,
    );

    // Position cursor at end of masked input line
    let cursor_x = popup_rect.x + 1 + display.chars().count() as u16 + 1;
    let cursor_y = popup_rect.y + 1;
    frame.set_cursor_position((cursor_x.min(popup_rect.x + popup_rect.width - 2), cursor_y));
}
