use anyhow::Result;
use async_trait::async_trait;

use futures::{
    Stream, stream, StreamExt
};

use eventsource_stream::Eventsource;

use bytes::Bytes;

use serde_json::Value;

use tokio::time::{
    sleep,
    Duration,
};

use serde::{Deserialize, Serialize};

use reqwest::Client;
use std::pin::Pin;

use crate::{
    ModelProvider,
    types::ProviderMessage,
};

pub struct OpenAIProvider {
    pub api_key: String,
    pub client: Client,
    pub model: String,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: String,
}

impl OpenAIProvider {

    pub fn new(api_key: String) -> Self {
        Self { 
            api_key, 
            client: Client::new(), 
            model: "gpt-4o-mini".to_string(),
        }
    }
}

#[async_trait]
impl ModelProvider for OpenAIProvider {
    
    async fn chat(
    &self,
    messages: Vec<ProviderMessage>,
) -> Result<String> {

    let body = ChatRequest {
        model: self.model.clone(),

        messages: messages
            .into_iter()
            .map(|m| OpenAIMessage {
                role: m.role,
                content: m.content,
            })
            .collect(),
        
        stream: false,
    };

    let response = self
        .client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(&self.api_key)
        .json(&body)
        .send()
        .await?;

    let response: ChatResponse =
        response.json().await?;

    Ok(
        response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default()
    )
}

    async fn stream_chat(
    &self,
    messages: Vec<ProviderMessage>,
) -> Result<
    Pin<
        Box<
            dyn Stream<
                Item = Result<String>
            > + Send
        >
    >
> {

    let body = ChatRequest {

        model: self.model.clone(),

        stream: true,

        messages: messages
            .into_iter()
            .map(|m| OpenAIMessage {
                role: m.role,
                content: m.content,
            })
            .collect(),
    };

    let response = self
        .client
        .post(
            "https://api.openai.com/v1/chat/completions"
        )
        .bearer_auth(&self.api_key)
        .json(&body)
        .send()
        .await?;

    let event_stream =
        response.bytes_stream().eventsource();

    let stream = event_stream.filter_map(
    |event| async move {

        match event {

            Ok(ev) => {

                let json = ev.data;

                if json == "[DONE]" {
                    return None;
                }

                if let Ok(value) =
                    serde_json::from_str::<Value>(&json)
                {

                    if let Some(token) = value
                        .pointer("/choices/0/delta/content")
                        .and_then(|v| v.as_str())
                    {
                        return Some(
                            Ok(token.to_string())
                        );
                    }
                }

                None
            }

            Err(err) => Some(
                Err(anyhow::anyhow!(err))
            ),
        }
    }
);

    Ok(Box::pin(stream))
}
}