use crate::error::{LlmError, Result};
use crate::llm::{ChatMessage, LLMClient};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;

pub struct MockLLMClient {
    responses: Mutex<VecDeque<String>>,
    model: Mutex<String>,
}

impl MockLLMClient {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Mutex::new(VecDeque::from(responses)),
            model: Mutex::new("mock-model".to_string()),
        }
    }

    pub fn add_response(&self, response: &str) {
        self.responses.lock().unwrap().push_back(response.to_string());
    }
}

#[async_trait]
impl LLMClient for MockLLMClient {
    async fn generate(&self, _prompt: &str) -> Result<String> {
        let mut responses = self.responses.lock().unwrap();
        responses
            .pop_front()
            .ok_or_else(|| LlmError::NoMoreResponses.into())
    }

    async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        let prompt = messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        self.generate(&prompt).await
    }

    fn set_model(&mut self, model: &str) {
        *self.model.lock().unwrap() = model.to_string();
    }
}
