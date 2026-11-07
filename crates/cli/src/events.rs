use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::sync::Arc;
use runtime::Runtime;
use std::time::Duration;
use providers::types::ProviderMessage;
use crate::app::{App, ChatMessage, MessageRole, COMMANDS};
use crate::models::{MODELS, unique_providers};
use crate::ollama;
use crate::themes;

pub async fn handle_events(app: &mut App, runtime: Arc<Runtime>) -> anyhow::Result<()> {
    if !event::poll(Duration::from_millis(16))? {
        return Ok(());
    }

    let Event::Key(key) = event::read()? else {
        return Ok(());
    };

    // ── Quit confirmation ─────────────────────────────────────────────────────
    if app.confirm_quit {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.should_quit = true,
            _ => app.confirm_quit = false,
        }
        return Ok(());
    }

    // ── API key input overlay ─────────────────────────────────────────────────
    if app.key_input_open {
        match key.code {
            KeyCode::Esc => {
                app.key_input_open = false;
                app.key_input_value.clear();
                app.pending_model_switch = None;
            }
            KeyCode::Backspace => { app.key_input_value.pop(); }
            KeyCode::Char(c) => { app.key_input_value.push(c); }
            KeyCode::Enter => {
                let key_val = app.key_input_value.trim().to_string();
                if !key_val.is_empty() {
                    // Persist to config
                    let mut cfg = config::Config::load().unwrap_or_default();
                    cfg.set_key_for_provider(&app.key_input_provider, key_val.clone());
                    cfg.save().ok();

                    // Update live Arc so the provider uses it immediately
                    *app.current_api_key.write().unwrap() = key_val;

                    // Complete a pending model switch if this was triggered from the picker
                    if let Some(idx) = app.pending_model_switch.take() {
                        let m = &MODELS[idx];
                        *app.current_model.write().unwrap()    = m.id.to_string();
                        *app.current_base_url.write().unwrap() = m.base_url.to_string();
                        // Persist model choice too
                        if let Ok(mut cfg) = config::Config::load() {
                            cfg.model = m.id.to_string();
                            cfg.save().ok();
                        }
                        app.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!(
                                "Switched to {} — API key for {} saved",
                                m.id, m.provider
                            ),
                        });
                    } else {
                        app.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!(
                                "API key for {} saved",
                                app.key_input_provider
                            ),
                        });
                    }

                    app.key_input_open = false;
                    app.key_input_value.clear();
                }
            }
            _ => {}
        }
        return Ok(());
    }

    // ── Theme picker ──────────────────────────────────────────────────────────
    if app.theme_picker_open {
        match key.code {
            KeyCode::Esc => {
                // Revert live preview to the confirmed theme
                app.theme_picker_selected = app.theme_idx;
                app.theme_picker_open = false;
            }
            KeyCode::Up => {
                if app.theme_picker_selected > 0 {
                    app.theme_picker_selected -= 1;
                }
            }
            KeyCode::Down => {
                if app.theme_picker_selected + 1 < themes::THEMES.len() {
                    app.theme_picker_selected += 1;
                }
            }
            KeyCode::Enter => {
                app.theme_idx = app.theme_picker_selected;
                app.theme_picker_open = false;
                // Persist
                if let Ok(mut cfg) = config::Config::load() {
                    cfg.theme = Some(themes::THEMES[app.theme_idx].name.to_string());
                    cfg.save().ok();
                }
                app.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!(
                        "Theme set to {}  —  {}",
                        themes::THEMES[app.theme_idx].name,
                        themes::THEMES[app.theme_idx].description,
                    ),
                });
            }
            _ => {}
        }
        return Ok(());
    }

    // ── /keys provider panel ──────────────────────────────────────────────────
    if app.keys_panel_open {
        let providers = unique_providers();
        match key.code {
            KeyCode::Esc => { app.keys_panel_open = false; }
            KeyCode::Up => {
                if app.keys_panel_selected > 0 {
                    app.keys_panel_selected -= 1;
                }
            }
            KeyCode::Down => {
                if app.keys_panel_selected + 1 < providers.len() {
                    app.keys_panel_selected += 1;
                }
            }
            KeyCode::Enter => {
                let p = providers[app.keys_panel_selected];
                app.keys_panel_open = false;
                app.key_input_open = true;
                app.key_input_provider = p.provider.to_string();
                app.key_input_env_hint = p.env_var.to_string();
                app.key_input_value.clear();
                app.pending_model_switch = None;
            }
            _ => {}
        }
        return Ok(());
    }

    // ── Model picker ──────────────────────────────────────────────────────────
    if app.model_picker_open {
        let total = MODELS.len() + app.local_models.len();
        match key.code {
            KeyCode::Esc => { app.model_picker_open = false; }
            KeyCode::Up => {
                if app.model_picker_selected > 0 {
                    app.model_picker_selected -= 1;
                }
            }
            KeyCode::Down => {
                if app.model_picker_selected + 1 < total {
                    app.model_picker_selected += 1;
                }
            }
            KeyCode::Enter => {
                let idx = app.model_picker_selected;
                if idx < MODELS.len() {
                    // ── Cloud model ──────────────────────────────────────────
                    let m = &MODELS[idx];
                    let cfg = config::Config::load().unwrap_or_default();

                    if let Some(key_val) = cfg.key_for_provider(m.provider) {
                        *app.current_model.write().unwrap()    = m.id.to_string();
                        *app.current_api_key.write().unwrap()  = key_val;
                        *app.current_base_url.write().unwrap() = m.base_url.to_string();
                        if let Ok(mut cfg) = config::Config::load() {
                            cfg.model = m.id.to_string();
                            cfg.save().ok();
                        }
                        app.messages.push(ChatMessage {
                            role: MessageRole::System,
                            content: format!("Switched to {}", m.id),
                        });
                        app.model_picker_open = false;
                    } else {
                        app.model_picker_open = false;
                        app.key_input_open = true;
                        app.key_input_provider = m.provider.to_string();
                        app.key_input_env_hint = m.env_var.to_string();
                        app.key_input_value.clear();
                        app.pending_model_switch = Some(idx);
                    }
                } else {
                    // ── Local Ollama model ───────────────────────────────────
                    let local_id = app.local_models[idx - MODELS.len()].clone();
                    *app.current_model.write().unwrap()    = local_id.clone();
                    *app.current_api_key.write().unwrap()  = "ollama".to_string();
                    *app.current_base_url.write().unwrap() = ollama::OLLAMA_BASE_URL.to_string();
                    if let Ok(mut cfg) = config::Config::load() {
                        cfg.model = local_id.clone();
                        cfg.save().ok();
                    }
                    app.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Switched to local model {}", local_id),
                    });
                    app.model_picker_open = false;
                }
            }
            _ => {}
        }
        return Ok(());
    }

    // ── Normal input ──────────────────────────────────────────────────────────
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
                app.scroll_offset = app.scroll_offset.saturating_add(3);
            }
        }

        KeyCode::PageUp   => { app.scroll_offset = app.scroll_offset.saturating_add(20); }
        KeyCode::PageDown => { app.scroll_offset = app.scroll_offset.saturating_sub(20); }

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

        KeyCode::BackTab => {
            app.mode = app.mode.cycle();
            app.filtered_commands.clear();
            app.selected_command = 0;
        }

        KeyCode::Enter => {
            if app.confirm_quit {
                app.should_quit = true;
                return Ok(());
            }

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
                "/exit" | "/quit" => { app.should_quit = true; }

                "/clear" => {
                    let boot_end = app.messages
                        .iter()
                        .take_while(|m| matches!(m.role, MessageRole::System))
                        .count();
                    app.messages.truncate(boot_end);
                }

                "/model" => {
                    app.local_models = ollama::discover().await;
                    app.model_picker_open = true;
                    app.model_picker_selected = {
                        let current = app.current_model.read().unwrap().clone();
                        MODELS.iter().position(|m| m.id == current).unwrap_or(0)
                    };
                }

                "/keys" => {
                    app.keys_panel_open = true;
                    app.keys_panel_selected = 0;
                }

                "/theme" => {
                    app.theme_picker_open = true;
                    app.theme_picker_selected = app.theme_idx;
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

                    let tx       = app.event_tx.clone();
                    let messages = app.provider_history.clone();
                    let runtime  = runtime.clone();

                    tokio::spawn(async move {
                        runtime.run(messages, tx).await;
                    });
                }
            }
        }

        KeyCode::Esc => {
            if app.confirm_quit {
                app.confirm_quit = false;
            } else if !app.input.is_empty() || !app.filtered_commands.is_empty() {
                app.input.clear();
                app.filtered_commands.clear();
                app.selected_command = 0;
            } else {
                app.confirm_quit = true;
            }
        }

        _ => {}
    }

    Ok(())
}
