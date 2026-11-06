use futures::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

use providers::{
    ModelProvider,
    types::{ProviderEvent, ProviderMessage, ToolCallRecord},
};

pub use providers::types::ProviderMessage as Message;

/// Events the runtime sends back to the CLI over the channel.
#[derive(Debug, Clone)]
pub enum RuntimeEvent {
    Token(String),
    ToolCallStarted { name: String },
    ToolCallFinished { name: String, result: String },
    /// Sent when the assistant finishes a complete response (after all tool loops).
    Finished,
    /// Full provider-format history after all tool loops, for the CLI to store.
    HistorySnapshot(Vec<ProviderMessage>),
    Error(String),
}

pub struct Runtime {
    provider: Box<dyn ModelProvider + Send + Sync>,
}

impl Runtime {
    pub fn new(provider: Box<dyn ModelProvider + Send + Sync>) -> Self {
        Self { provider }
    }

    /// Run the full agentic loop:
    /// stream → collect tool calls → execute → continue until no more tool calls.
    pub async fn run(
        &self,
        mut messages: Vec<ProviderMessage>,
        tx: UnboundedSender<RuntimeEvent>,
    ) {
        let tools = tools::definitions();

        loop {
            let stream = match self.provider.stream_chat(messages.clone(), &tools).await {
                Ok(s) => s,
                Err(e) => {
                    tx.send(RuntimeEvent::Error(e.to_string())).ok();
                    return;
                }
            };

            tokio::pin!(stream);

            let mut assistant_text = String::new();
            let mut tool_calls: Vec<(String, String, String)> = vec![]; // (call_id, name, args)
            let mut had_error = false;

            while let Some(event) = stream.next().await {
                match event {
                    Ok(ProviderEvent::Token(t)) => {
                        assistant_text.push_str(&t);
                        tx.send(RuntimeEvent::Token(t)).ok();
                    }
                    Ok(ProviderEvent::ToolCall { call_id, name, args_json }) => {
                        tool_calls.push((call_id, name, args_json));
                    }
                    Err(e) => {
                        tx.send(RuntimeEvent::Error(e.to_string())).ok();
                        had_error = true;
                        break;
                    }
                }
            }

            if had_error {
                return;
            }

            if tool_calls.is_empty() {
                // Normal text response — done
                messages.push(ProviderMessage::assistant(&assistant_text));
                tx.send(RuntimeEvent::Finished).ok();
                tx.send(RuntimeEvent::HistorySnapshot(messages)).ok();
                return;
            }

            // Commit any partial text the model emitted before tool calls
            if !assistant_text.is_empty() {
                tx.send(RuntimeEvent::Finished).ok();
            }

            // Record the assistant's decision to call tools
            messages.push(ProviderMessage {
                role: "assistant".into(),
                content: assistant_text,
                tool_calls: tool_calls.iter().map(|(id, name, args)| ToolCallRecord {
                    id: id.clone(),
                    name: name.clone(),
                    args_json: args.clone(),
                }).collect(),
                tool_call_id: None,
            });

            // Execute each tool, add result to history
            for (call_id, name, args_json) in &tool_calls {
                tx.send(RuntimeEvent::ToolCallStarted { name: name.clone() }).ok();

                let result = tools::execute(name, args_json).await
                    .unwrap_or_else(|e| format!("error: {}", e));

                tx.send(RuntimeEvent::ToolCallFinished {
                    name: name.clone(),
                    result: result.clone(),
                }).ok();

                messages.push(ProviderMessage::tool_result(call_id, result));
            }

            // Loop: stream the assistant's follow-up response
        }
    }
}
