use crossterm::event::{
    self,
    Event,
    KeyCode,
};

use std::sync::Arc;
use runtime::Runtime;
use crate::runtime_events::RuntimeEvent;
use futures::StreamExt;
use std::time::Duration;
use crate::app::{
    App, ChatMessage, MessageRole,
};


pub async fn handle_events(app: &mut App, runtime: Arc<Runtime>) -> anyhow::Result<()> {
    if event::poll(
        Duration::from_millis(16)
    )? {
    if let Event::Key(key) = event::read()? {
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
                }
            }

            KeyCode::Up => {
                if !app.filtered_commands.is_empty() {
                    if app.selected_command == 0 {
                        app.selected_command = app.filtered_commands.len() - 1;
                    } else {
                        app.selected_command = app.selected_command - 1;
                    }
                }
            }

            KeyCode::Tab => {
                if let Some(command) = app.filtered_commands
                .get(app.selected_command) {
                    app.input = command.clone();
                }
            }

            KeyCode::Enter => {

            let input = app.input.clone();
            if !input.is_empty() {

            // USER MESSAGE

            app.messages.push(
                ChatMessage {
                    role: MessageRole::User,
                    content: input.clone(),
                }
            );

            let tx = app.event_tx.clone();
            let prompt = input.clone();
            let runtime = runtime.clone();

            tokio::spawn(async move {
                match runtime.stream_chat(prompt).await {

    Ok(mut stream) => {

        while let Some(chunk) =
            stream.next().await
        {

            match chunk {

                Ok(token) => {

                    let _ = tx.send(
                        RuntimeEvent::Token(token)
                    );
                }

                Err(error) => {

                    let _ = tx.send(
                        RuntimeEvent::Error(
                            error.to_string()
                        )
                    );

                    return;
                }
            }
        }

        let _ = tx.send(
            RuntimeEvent::Finished
        );
    }

    Err(error) => {

        let _ = tx.send(
            RuntimeEvent::Error(
                error.to_string()
            )
        );
    }
}
            });

            }

            app.input.clear();
            app.filtered_commands.clear();
            app.selected_command = 0;
    }

            KeyCode::Esc => {
                app.should_quit = false;
            }

            _ => {}
        }
    }
}

    Ok(())
}