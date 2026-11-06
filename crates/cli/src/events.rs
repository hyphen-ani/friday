use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::sync::Arc;
use runtime::Runtime;
use std::time::Duration;
use providers::types::ProviderMessage;
use crate::app::{App, ChatMessage, MessageRole, COMMANDS};

pub async fn handle_events(app: &mut App, runtime: Arc<Runtime>) -> anyhow::Result<()> {
    if !event::poll(Duration::from_millis(16))? {
        return Ok(());
    }

    let Event::Key(key) = event::read()? else {
        return Ok(());
    };

    // When quit confirmation is showing, only y/Enter quit and n/Esc cancel
    if app.confirm_quit {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.should_quit = true,
            _ => app.confirm_quit = false,
        }
        return Ok(());
    }

    match key.code {
        KeyCode::Char(c) => {
            app.input.push(c);
            app.update_command_filter();
        }

        KeyCode::Backspace => {
            app.input.pop();
            app.update_command_filter();
        }

        KeyCode::Down => {
            if !app.filtered_commands.is_empty() {
                app.selected_command =
                    (app.selected_command + 1) % app.filtered_commands.len();
            } else {
                // Scroll down through conversation
                app.scroll_offset = app.scroll_offset.saturating_sub(3);
            }
        }

        KeyCode::Up => {
            if !app.filtered_commands.is_empty() {
                if app.selected_command == 0 {
                    app.selected_command = app.filtered_commands.len() - 1;
                } else {
                    app.selected_command -= 1;
                }
            } else {
                // Scroll up through conversation
                app.scroll_offset = app.scroll_offset.saturating_add(3);
            }
        }

        KeyCode::PageUp => {
            app.scroll_offset = app.scroll_offset.saturating_add(20);
        }

        KeyCode::PageDown => {
            app.scroll_offset = app.scroll_offset.saturating_sub(20);
        }

        // Tab: autocomplete or (with Shift) cycle mode
        // Some terminals send Shift+Tab as Tab+SHIFT instead of BackTab
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.mode = app.mode.cycle();
                app.filtered_commands.clear();
                app.selected_command = 0;
            } else if let Some(&idx) = app.filtered_commands.get(app.selected_command) {
                app.input = COMMANDS[idx].name.to_string();
                app.update_command_filter();
            }
        }

        // Shift+Tab: cycle mode (most terminals send this directly)
        KeyCode::BackTab => {
            app.mode = app.mode.cycle();
            app.filtered_commands.clear();
            app.selected_command = 0;
        }

        KeyCode::Enter => {
            // If the quit confirmation dialog is open, confirm quit
            if app.confirm_quit {
                app.should_quit = true;
                return Ok(());
            }

            // If the command palette is open, resolve the selected entry
            let input = if !app.filtered_commands.is_empty() {
                COMMANDS[app.filtered_commands[app.selected_command]].name.to_string()
            } else {
                app.input.trim().to_string()
            };
            app.input.clear();
            app.filtered_commands.clear();
            app.selected_command = 0;

            if input.is_empty() {
                return Ok(());
            }

            match input.as_str() {
                "/exit" | "/quit" => {
                    app.should_quit = true;
                }

                "/clear" => {
                    // Keep only the leading System boot messages
                    let boot_end = app.messages
                        .iter()
                        .take_while(|m| matches!(m.role, MessageRole::System))
                        .count();
                    app.messages.truncate(boot_end);
                }

                "/tools" => {
                    let defs = tools::definitions();
                    let mut lines = vec!["Available tools:".to_string()];
                    for t in &defs {
                        lines.push(format!("  {}  —  {}", t.name, t.description));
                    }
                    app.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: lines.join("\n"),
                    });
                }

                cmd if cmd.starts_with('/') => {
                    app.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("{} — not yet implemented", cmd),
                    });
                }

                _ => {
                    app.welcome_dismissed = true;
                    app.scroll_offset = 0;

                    // Add user message to both display and provider history
                    app.messages.push(ChatMessage {
                        role: MessageRole::User,
                        content: input.clone(),
                    });
                    app.provider_history.push(ProviderMessage {
                        role: "user".into(),
                        content: input,
                        tool_calls: vec![],
                        tool_call_id: None,
                    });

                    let tx = app.event_tx.clone();
                    let messages = app.provider_history.clone();
                    let runtime = runtime.clone();

                    tokio::spawn(async move {
                        runtime.run(messages, tx).await;
                    });
                }
            }
        }

        KeyCode::Esc => {
            if app.confirm_quit {
                // Cancel the quit confirmation
                app.confirm_quit = false;
            } else if !app.input.is_empty() || !app.filtered_commands.is_empty() {
                app.input.clear();
                app.filtered_commands.clear();
                app.selected_command = 0;
            } else {
                // Show confirmation dialog instead of quitting immediately
                app.confirm_quit = true;
            }
        }

        _ => {}
    }

    Ok(())
}
