use anyhow::Result;
use async_stream::stream;
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, pin::Pin};

use crate::{
    ModelProvider,
    types::{ProviderEvent, ProviderMessage, ToolDef},
};

pub struct OpenAIProvider {
    pub api_key: String,
    pub client: Client,
    pub model: String,
    pub base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
            model: "gpt-4o-mini".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
}

// ── Request types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ReqMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    tools: Vec<ReqTool<'a>>,
}

#[derive(Serialize)]
struct ReqMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    tool_calls: Vec<ReqToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize)]
struct ReqToolCall {
    id: String,
    r#type: &'static str,
    function: ReqToolCallFn,
}

#[derive(Serialize)]
struct ReqToolCallFn {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct ReqTool<'a> {
    r#type: &'static str,
    function: ReqToolDef<'a>,
}

#[derive(Serialize)]
struct ReqToolDef<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a Value,
}

// ── Streaming response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct Chunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct ToolCallDelta {
    index: usize,
    id: Option<String>,
    function: Option<FnDelta>,
}

#[derive(Debug, Deserialize)]
struct FnDelta {
    name: Option<String>,
    arguments: Option<String>,
}

// Accumulator for a single tool call built across streaming chunks
#[derive(Default)]
struct PendingCall {
    id: String,
    name: String,
    args: String,
}

// ── Provider impl ─────────────────────────────────────────────────────────────

#[async_trait]
impl ModelProvider for OpenAIProvider {
    async fn stream_chat(
        &self,
        messages: Vec<ProviderMessage>,
        tools: &[ToolDef],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ProviderEvent>> + Send>>> {
        let req_messages: Vec<ReqMessage> = messages.into_iter().map(|m| {
            let has_tool_calls = !m.tool_calls.is_empty();
            ReqMessage {
                role: m.role,
                // content must be null (None) for assistant messages that only have tool_calls
                content: if has_tool_calls && m.content.is_empty() {
                    None
                } else {
                    Some(m.content)
                },
                tool_calls: m.tool_calls.into_iter().map(|tc| ReqToolCall {
                    id: tc.id,
                    r#type: "function",
                    function: ReqToolCallFn { name: tc.name, arguments: tc.args_json },
                }).collect(),
                tool_call_id: m.tool_call_id,
            }
        }).collect();

        let req_tools: Vec<ReqTool> = tools.iter().map(|t| ReqTool {
            r#type: "function",
            function: ReqToolDef {
                name: t.name,
                description: t.description,
                parameters: &t.parameters,
            },
        }).collect();

        let body = ChatRequest {
            model: &self.model,
            messages: req_messages,
            stream: true,
            tools: req_tools,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let response = self.client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let event_stream = response.bytes_stream().eventsource();

        let s = stream! {
            let mut pending: HashMap<usize, PendingCall> = HashMap::new();
            tokio::pin!(event_stream);

            while let Some(ev) = event_stream.next().await {
                match ev {
                    Ok(ev) => {
                        let data = ev.data;
                        if data == "[DONE]" { break; }

                        let chunk: Chunk = match serde_json::from_str(&data) {
                            Ok(c) => c,
                            Err(_) => continue,
                        };

                        for choice in &chunk.choices {
                            // Text token
                            if let Some(content) = &choice.delta.content {
                                if !content.is_empty() {
                                    yield Ok(ProviderEvent::Token(content.clone()));
                                }
                            }

                            // Tool call deltas — accumulate by index
                            if let Some(tcs) = &choice.delta.tool_calls {
                                for tc in tcs {
                                    let entry = pending.entry(tc.index).or_default();
                                    if let Some(id) = &tc.id { entry.id = id.clone(); }
                                    if let Some(f) = &tc.function {
                                        if let Some(n) = &f.name { entry.name = n.clone(); }
                                        if let Some(a) = &f.arguments { entry.args.push_str(a); }
                                    }
                                }
                            }

                            // Emit completed tool calls when stream finishes
                            if choice.finish_reason.as_deref() == Some("tool_calls") {
                                let mut indices: Vec<usize> = pending.keys().cloned().collect();
                                indices.sort_unstable();
                                for idx in indices {
                                    if let Some(pc) = pending.remove(&idx) {
                                        yield Ok(ProviderEvent::ToolCall {
                                            call_id: pc.id,
                                            name: pc.name,
                                            args_json: pc.args,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => yield Err(anyhow::anyhow!("{}", e)),
                }
            }
        };

        Ok(Box::pin(s))
    }
}
