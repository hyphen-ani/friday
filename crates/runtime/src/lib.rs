use anyhow::Result;
use std::pin::Pin;

use providers::{
    ModelProvider,
    types::ProviderMessage,
};

pub struct Runtime {

    provider: Box<
        dyn ModelProvider + Send + Sync
    >,
}

impl Runtime {

    pub fn new(
        provider: Box<
            dyn ModelProvider + Send + Sync
        >
    ) -> Self {

        Self {
            provider,
        }
    }

    pub async fn chat(
        &self,
        input: String,
    ) -> Result<String> {

        let messages = vec![
            ProviderMessage {
                role: "user".to_string(),
                content: input,
            }
        ];

        let response = self
            .provider
            .chat(messages)
            .await?;

        Ok(response)
    }

    pub async fn stream_chat(
    &self,
    input: String,
) -> Result<
    Pin<
        Box<
            dyn futures::Stream<
                Item = Result<String>
            > + Send
        >
    >
> {

    let messages = vec![
        ProviderMessage {
            role: "user".to_string(),
            content: input,
        }
    ];

    self.provider
        .stream_chat(messages)
        .await
}
}