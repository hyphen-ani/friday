use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use providers::types::ProviderMessage;
use crate::runtime_events::RuntimeEvent;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Chat,
    Command,
    Agent,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Chat    => "chat",
            Mode::Command => "command",
            Mode::Agent   => "agent",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Mode::Chat    => Mode::Command,
            Mode::Command => Mode::Agent,
            Mode::Agent   => Mode::Chat,
        }
    }

    pub fn placeholder(self) -> &'static str {
        match self {
            Mode::Chat    => "Ask anything…",
            Mode::Command => "Type a / command…",
            Mode::Agent   => "Agent mode — coming soon…",
        }
    }
}

pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
}

pub const COMMANDS: &[Command] = &[
    Command { name: "/help",    description: "Show available commands and shortcuts" },
    Command { name: "/clear",   description: "Clear the conversation history" },
    Command { name: "/model",   description: "Switch the active AI model" },
    Command { name: "/keys",    description: "Manage API keys for each provider" },
    Command { name: "/tools",   description: "List available tools" },
    Command { name: "/session", description: "Show current session info" },
    Command { name: "/theme",   description: "Change the color theme" },
    Command { name: "/exit",    description: "Quit Friday" },
];

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
    pub confirm_quit: bool,
    pub mode: Mode,
    pub welcome_dismissed: bool,
    pub scroll_offset: usize,
    pub provider_history: Vec<ProviderMessage>,
    pub filtered_commands: Vec<usize>,
    pub selected_command: usize,

    // Shared live state — same Arcs held by the provider
    pub current_model:    Arc<RwLock<String>>,
    pub current_api_key:  Arc<RwLock<String>>,
    pub current_base_url: Arc<RwLock<String>>,

    // Model picker
    pub model_picker_open:     bool,
    pub model_picker_selected: usize,

    // Inline API-key prompt (shown when switching to a provider with no stored key)
    pub key_input_open:          bool,
    pub key_input_provider:      String,
    pub key_input_value:         String,
    pub key_input_env_hint:      String,  // e.g. "ANTHROPIC_API_KEY"
    pub pending_model_switch:    Option<usize>,  // MODELS index waiting for key

    // /keys management panel
    pub keys_panel_open:     bool,
    pub keys_panel_selected: usize,

    // Local Ollama models discovered at runtime
    pub local_models: Vec<String>,

    // Theme
    pub theme_idx:            usize,
    pub theme_picker_open:    bool,
    pub theme_picker_selected: usize,

    pub event_tx: UnboundedSender<RuntimeEvent>,
    pub event_rx: UnboundedReceiver<RuntimeEvent>,
}

impl App {
    pub fn new(
        current_model:    Arc<RwLock<String>>,
        current_api_key:  Arc<RwLock<String>>,
        current_base_url: Arc<RwLock<String>>,
    ) -> Self {
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            input: String::new(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::System,
                    content: "Friday runtime initialized".to_string(),
                }
            ],
            streaming_message: None,
            should_quit: false,
            confirm_quit: false,
            mode: Mode::Chat,
            welcome_dismissed: false,
            scroll_offset: 0,
            provider_history: vec![],
            filtered_commands: vec![],
            selected_command: 0,
            current_model,
            current_api_key,
            current_base_url,
            model_picker_open: false,
            model_picker_selected: 0,
            key_input_open: false,
            key_input_provider: String::new(),
            key_input_value: String::new(),
            key_input_env_hint: String::new(),
            pending_model_switch: None,
            keys_panel_open: false,
            keys_panel_selected: 0,
            local_models: vec![],
            theme_idx: 0,
            theme_picker_open: false,
            theme_picker_selected: 0,
            event_tx,
            event_rx,
        }
    }

    #[allow(dead_code)]
    pub fn provider_messages(&self) -> Vec<ProviderMessage> {
        if !self.provider_history.is_empty() {
            return self.provider_history.clone();
        }
        self.messages.iter().filter_map(|m| match m.role {
            MessageRole::User => Some(ProviderMessage {
                role: "user".to_string(),
                content: m.content.clone(),
                tool_calls: vec![],
                tool_call_id: None,
            }),
            MessageRole::Assistant => Some(ProviderMessage {
                role: "assistant".to_string(),
                content: m.content.clone(),
                tool_calls: vec![],
                tool_call_id: None,
            }),
            MessageRole::System => None,
        }).collect()
    }

    pub fn update_command_filter(&mut self) {
        if self.input.starts_with('/') {
            self.filtered_commands = COMMANDS
                .iter()
                .enumerate()
                .filter(|(_, cmd)| cmd.name.starts_with(self.input.as_str()))
                .map(|(i, _)| i)
                .collect();
        } else {
            self.filtered_commands.clear();
        }
        self.selected_command = 0;
    }

    pub fn process_runtime_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                RuntimeEvent::Token(token) => {
                    if self.streaming_message.is_none() {
                        self.streaming_message = Some(String::new());
                    }
                    if let Some(msg) = &mut self.streaming_message {
                        msg.push_str(&token);
                    }
                }

                RuntimeEvent::Finished => {
                    if let Some(content) = self.streaming_message.take() {
                        self.messages.push(ChatMessage {
                            role: MessageRole::Assistant,
                            content,
                        });
                    }
                }

                RuntimeEvent::ToolCallStarted { name } => {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("⚙  running {}…", name),
                    });
                }

                RuntimeEvent::ToolCallFinished { name, result } => {
                    let preview: String = result.lines().take(4).collect::<Vec<_>>().join("\n");
                    if let Some(last) = self.messages.last_mut() {
                        if last.content == format!("⚙  running {}…", name) {
                            last.content = format!("⚙  {} → {}", name, preview);
                            continue;
                        }
                    }
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("⚙  {} → {}", name, preview),
                    });
                }

                RuntimeEvent::HistorySnapshot(history) => {
                    self.provider_history = history;
                }

                RuntimeEvent::Error(err) => {
                    self.streaming_message = None;
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Error: {}", err),
                    });
                }

                RuntimeEvent::LocalModels(models) => {
                    self.local_models = models;
                }
            }
        }
    }
}
