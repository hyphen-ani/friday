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

    
    // LAYOUT
    

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // logo
            Constraint::Length(1), // path
            Constraint::Min(1),    // messages
            Constraint::Length(1), // separator
            Constraint::Length(3), // input
            Constraint::Length(1), // separator
            Constraint::Length(1), // status
        ])
        .split(frame.area());

    
    // COLORFUL LOGO
    

    let logo = Paragraph::new(
    Line::from(vec![

        Span::styled(
            "Friday",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        ),

        Span::styled(
            " — AI Runtime",
            Style::default()
                .fg(Color::DarkGray)
        ),
    ])
    )
    .alignment(Alignment::Left);

    frame.render_widget(logo, layout[0]);

    
    // PROJECT INFO
    

    let info = Paragraph::new(
        "~/projects/friday • main"
    )
    .style(
        Style::default()
            .fg(Color::DarkGray)
    );

    frame.render_widget(info, layout[1]);

    
    // MESSAGES

let lines: Vec<Line> = app
    .messages
    .iter()
    .map(|message| {

        match message.role {

            
            // USER
            
            MessageRole::User => {

                Line::from(vec![

                    Span::styled(
                        "You ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    ),

                    Span::styled(
                        "› ",
                        Style::default()
                            .fg(Color::DarkGray)
                    ),

                    Span::raw(&message.content),
                ])
            }

            
            // ASSISTANT
            

            MessageRole::Assistant => {

                Line::from(vec![

                    Span::styled(
                        "Friday ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    ),

                    Span::styled(
                        "› ",
                        Style::default()
                            .fg(Color::DarkGray)
                    ),

                    Span::raw(&message.content),
                ])
            }

            
            // SYSTEM
            

            MessageRole::System => {

                Line::from(vec![

                    Span::styled(
                        &message.content,
                        Style::default()
                            .fg(Color::DarkGray)
                    ),
                ])
            }
        }
    })
    .collect();

    let mut all_lines = lines;

if let Some(streaming) =
    &app.streaming_message {

    all_lines.push(

        Line::from(vec![

            Span::styled(
                "Friday ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            ),

            Span::styled(
                "› ",
                Style::default()
                    .fg(Color::DarkGray)
            ),

            Span::raw(streaming),
        ])
    );
}

let messages = Paragraph::new(all_lines)
    .wrap(Wrap { trim: true });

frame.render_widget(
    messages,
    layout[2],
);

    
    // SEPARATOR
    

    let separator = Paragraph::new(
        "─".repeat(frame.area().width as usize)
    )
    .style(
        Style::default()
            .fg(Color::DarkGray)
    );

    frame.render_widget(separator.clone(), layout[3]);

    
    // INPUT
    

    let input_text = if app.input.is_empty() {

        Line::from(vec![
            Span::styled(
                "❯ ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            ),

            Span::styled(
                "Ask anything or type / for commands",
                Style::default()
                    .fg(Color::DarkGray)
            ),
        ])

    } else {

        Line::from(vec![
            Span::styled(
                "❯ ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            ),

            Span::styled(
                &app.input,
                Style::default()
                    .fg(Color::White)
            ),
        ])
    };

    let input = Paragraph::new(input_text);

    frame.render_widget(input, layout[4]);

    
    // COMMAND SUGGESTIONS
    

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
                                .bg(Color::White)
                                .fg(Color::Black)
                                .add_modifier(Modifier::BOLD)
                        )
                    )

                } else {

                    Line::from(
                        Span::styled(
                            format!("  {}", cmd),
                            Style::default()
                                .fg(Color::DarkGray)
                        )
                    )
                }
            })
            .collect();

        let suggestion_box = Paragraph::new(
            suggestions
        );

        let popup_height = app.filtered_commands.len().min(16) as u16;

        let popup_y =
            layout[4]
                .y
                .saturating_sub(popup_height);

        let area = Rect {
            x: 3,
            y: popup_y,
            width: 80,
            height: popup_height,
        };

        frame.render_widget(
            suggestion_box,
            area,
        );
    }

    
    // SECOND SEPARATOR
    

    frame.render_widget(separator, layout[5]);

    
    // STATUS BAR
    

    let status = Paragraph::new(
        "● connected    model: openai-40-mini    phase: explore"
    )
    .style(
        Style::default()
            .fg(Color::DarkGray)
    );

    frame.render_widget(status, layout[6]);

    
    // CURSOR
    

    frame.set_cursor_position((
        layout[4].x + app.input.len() as u16 + 2,
        layout[4].y,
    ));
}