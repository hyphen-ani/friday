pub mod types;
pub mod openai;
use std::marker::Send;
use anyhow::Result;

use async_trait::async_trait;

use futures::Stream;

use std::pin::Pin;

use types::ProviderMessage;

#[async_trait]
pub trait ModelProvider {

    async fn chat(
        &self,
        messages: Vec<ProviderMessage>,
    ) -> Result<String>;

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
    >;
}