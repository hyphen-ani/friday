use ratatui::{
    prelude::*,
    widgets::*,
    style::{
        Color,
        Modifier,
        Style,
    },
};

use crate::app::{
    App, MessageRole
};

pub fn render(
    frame: &mut Frame,
    app: &App,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // logo
            Constraint::Min(1),    // messages
            Constraint::Length(3), // input
            Constraint::Length(1), // status
        ])
        .split(frame.area());

    // ========================================
    // LOGO
    // ========================================

    let logo = Paragraph::new(
        Line::from(vec![
            Span::styled(
                "✦ ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Friday",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    );

    frame.render_widget(logo, layout[0]);

    // ========================================
    // MESSAGES
    // ========================================

    let mut lines: Vec<Line> = vec![];

    for message in &app.messages {
        match message.role {
            MessageRole::User => {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(
                        "● You",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(message.content.clone()));
            }

            MessageRole::Assistant => {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(
                        "✦ Friday",
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(message.content.clone()));
            }

            MessageRole::System => {
                lines.push(Line::from(vec![
                    Span::styled(
                        &message.content,
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }
    }

    if let Some(streaming) = &app.streaming_message {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "✦ Friday",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(streaming.clone()));
    }

    let messages = Paragraph::new(lines)
        .wrap(Wrap { trim: false });

    frame.render_widget(
        messages,
        layout[1],
    );

    // ========================================
    // INPUT BAR
    // ========================================

    let input_text = if app.input.is_empty() {
        Line::from(vec![
            Span::styled(
                "Ask anything or type /",
                Style::default()
                    .fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                &app.input,
                Style::default()
                    .fg(Color::White),
            ),
        ])
    };

    let input = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(Color::DarkGray),
                ),
        );

    frame.render_widget(
        input,
        layout[2],
    );

    // ========================================
    // COMMAND PALETTE
    // ========================================

    if !app.filtered_commands.is_empty() {

        let suggestions: Vec<Line> = app
            .filtered_commands
            .iter()
            .enumerate()
            .map(|(i, cmd)| {

                if i == app.selected_command {

                    Line::from(
                        Span::styled(
                            format!("› {}", cmd),
                            Style::default()
                                .bg(Color::DarkGray)
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    )

                } else {

                    Line::from(
                        Span::styled(
                            format!("  {}", cmd),
                            Style::default()
                                .fg(Color::Gray),
                        ),
                    )
                }
            })
            .collect();

        let popup_height =
            app.filtered_commands
                .len()
                .min(8) as u16 + 2;

        let popup_y =
            layout[2]
                .y
                .saturating_sub(
                    popup_height,
                );

        let popup = Paragraph::new(
            suggestions,
        )
        .block(
            Block::default()
                .title(" friday commands ")
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(Color::DarkGray),
                ),
        );

        frame.render_widget(
            popup,
            Rect {
                x: 2,
                y: popup_y,
                width: 40,
                height: popup_height,
            },
        );
    }

    // ========================================
    // STATUS
    // ========================================

    let status = Paragraph::new(
        "openai-4o-mini • streaming enabled",
    )
    .style(
        Style::default()
            .fg(Color::DarkGray),
    )
    .alignment(
        Alignment::Right,
    );

    frame.render_widget(
        status,
        layout[3],
    );

    // ========================================
    // CURSOR
    // ========================================

    frame.set_cursor_position((
        layout[2].x
            + app.input.len() as u16
            + 1,
        layout[2].y + 1,
    ));
}