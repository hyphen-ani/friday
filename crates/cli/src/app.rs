use std::{fmt::format, vec};

use tokio::sync::mpsc::{
    UnboundedReceiver,
    UnboundedSender,
};

use crate::{events, runtime_events::RuntimeEvent};


#[derive(Clone)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

pub struct App {
    pub input: String,
    pub messages: Vec<ChatMessage>,
    pub streaming_message: Option<String>,
    pub should_quit: bool,
    pub commands: Vec<String>,
    pub filtered_commands: Vec<String>,
    pub selected_command: usize,
    pub event_tx: UnboundedSender<RuntimeEvent>,
    pub event_rx: UnboundedReceiver<RuntimeEvent>,
}

impl App {
    pub fn new() -> Self {

        let commands = vec![
            "/help".to_string(),
            "/clear".to_string(),
            "/exit".to_string(),
            "/model".to_string(),
            "/tools".to_string(),
            "/session".to_string(),
            "/theme".to_string(),
        ];

        let (event_tx, event_rx) = 
        tokio::sync::mpsc::unbounded_channel();

        Self {
            input: String::new(),
            messages: vec![
                ChatMessage{
                    role: MessageRole::System,
                    content: "Friday runtime initialized".to_string(),
                }
            ],
            streaming_message: None,
            should_quit: false,
            commands,
            filtered_commands: vec![],
            selected_command: 0,
            event_tx,
            event_rx,
        }
    }

    pub fn update_command_filter(&mut self){
        if self.input.starts_with("/") {
            self.filtered_commands = self
                .commands
                .iter()
                .filter(|cmd|{
                    cmd.starts_with(&self.input)
                })
                .cloned()
                .collect();
        } else {
            self.filtered_commands.clear();
        }
    }

    pub fn process_runtime_events(&mut self){
        while let Ok(event) = self.event_rx.try_recv() {

            match event {
                RuntimeEvent::Token(token) => {
                    if self.streaming_message.is_none() {
                        self.streaming_message = Some(String::new());
                    }

                    if let Some(message) = &mut self.streaming_message {
                        message.push_str(&token);
                    }
                }

                RuntimeEvent::Finished => {
                    if let Some(message) = self.streaming_message.take() {
                        self.messages.push(
                            ChatMessage { role: MessageRole::Assistant, content: message }
                        );
                    }
                }

                RuntimeEvent::Error(error) => {
                    self.messages.push(
                        ChatMessage { role: MessageRole::System, 
                            content: format!(
                                "Error: {}", error
                            ),
                        }
                    );
                }
                
            }
            
        }
    }
}