pub mod types;
pub mod openai;

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use types::{ProviderEvent, ProviderMessage, ToolDef};

#[async_trait]
pub trait ModelProvider {
    async fn stream_chat(
        &self,
        messages: Vec<ProviderMessage>,
        tools: &[ToolDef],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ProviderEvent>> + Send>>>;
}
