use crate::error::{LlmError, Result};
use crate::llm::{ChatMessage, LLMClient};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{timeout, Duration};

#[derive(Debug)]
pub struct OpenAIClient {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
    #[allow(dead_code)]
    connect_timeout: Duration,
    read_timeout: Duration,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
    max_tokens: usize,
}

#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[async_trait]
impl LLMClient for OpenAIClient {
    async fn generate(&self, prompt: &str) -> Result<String> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.7,
            max_tokens: 4096,
        };

        let response = timeout(
            self.read_timeout,
            self.client
                .post(&format!("{}/chat/completions", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
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

        let body: OpenAIResponse = response.json().await.map_err(|e| LlmError::ParseError(e.to_string()))?;
        Ok(body.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        let openai_messages: Vec<OpenAIMessage> = messages
            .iter()
            .map(|m| OpenAIMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: openai_messages,
            temperature: 0.7,
            max_tokens: 4096,
        };

        let response = timeout(
            self.read_timeout,
            self.client
                .post(&format!("{}/chat/completions", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
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

        let body: OpenAIResponse = response.json().await.map_err(|e| LlmError::ParseError(e.to_string()))?;
        Ok(body.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }
}

impl OpenAIClient {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(120),
        }
    }

    #[allow(dead_code)]
    pub fn from_env() -> Self {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY environment variable not set");
        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model = std::env::var("OPENAI_MODEL")
            .unwrap_or_else(|_| "gpt-4".to_string());

        Self::new(&base_url, &api_key, &model)
    }
}
