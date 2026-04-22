use crate::error::{LlmError, Result};
use crate::llm::{ChatMessage, LLMClient};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[derive(Debug)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
    connect_timeout: Duration,
    read_timeout: Duration,
}

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: ModelOptions,
}

#[derive(Serialize)]
struct ModelOptions {
    temperature: f32,
    num_predict: usize,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[async_trait]
impl LLMClient for OllamaClient {
    async fn generate(&self, prompt: &str) -> Result<String> {
        let request = GenerateRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: ModelOptions {
                temperature: 0.7,
                num_predict: 4096,
            },
        };

        let response = timeout(
            self.read_timeout,
            self.client
                .post(&format!("{}/api/generate", self.base_url))
                .json(&request)
                .send(),
        )
        .await
        .map_err(|_| LlmError::Timeout(self.read_timeout))??;

        if !response.status().is_success() {
            return Err(LlmError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            }
            .into());
        }

        let body: GenerateResponse = response.json().await.map_err(|e| LlmError::ParseError(e.to_string()))?;
        Ok(body.response)
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
        self.model = model.to_string();
    }
}

impl OllamaClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.to_string(),
            model: model.to_string(),
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(120),
        }
    }
}
