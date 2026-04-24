use async_trait::async_trait;
use crate::error::Result;
use serde::{Deserialize, Serialize};

pub mod mock;
pub mod ollama;
pub mod cache;
pub mod openai;
pub mod multi_client;

pub use mock::MockLLMClient;
pub use ollama::OllamaClient;
pub use openai::OpenAIClient;
pub use multi_client::MultiLLMClient;

#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn generate(&self, prompt: &str) -> Result<String>;

    async fn chat(&self, messages: &[ChatMessage]) -> Result<String>;

    #[allow(dead_code)]
    fn set_model(&mut self, model: &str);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }

    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }
}
