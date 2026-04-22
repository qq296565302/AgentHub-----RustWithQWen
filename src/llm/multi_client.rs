use crate::config::{LLMConfig, LLMProviderConfig};
use crate::error::{LlmError, Result};
use crate::llm::{ChatMessage, LLMClient, MockLLMClient, OllamaClient, OpenAIClient};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MultiLLMClient {
    clients: RwLock<HashMap<String, Arc<dyn LLMClient>>>,
    current_provider: RwLock<String>,
}

impl MultiLLMClient {
    pub fn new(config: &LLMConfig) -> Self {
        let mut clients = HashMap::new();

        for provider_config in &config.providers {
            let client = Self::create_client(provider_config);
            clients.insert(provider_config.name.clone(), client);
        }

        Self {
            clients: RwLock::new(clients),
            current_provider: RwLock::new(config.default_provider.clone()),
        }
    }

    fn create_client(config: &LLMProviderConfig) -> Arc<dyn LLMClient> {
        match config.provider_type.to_lowercase().as_str() {
            "openai" => {
                let api_key = config.api_key.clone()
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .unwrap_or_default();
                Arc::new(OpenAIClient::new(&config.api_endpoint, &api_key, &config.model))
            }
            "ollama" => {
                Arc::new(OllamaClient::new(&config.api_endpoint, &config.model))
            }
            _ => {
                Arc::new(MockLLMClient::new(vec![
                    format!("Mock response from {}", config.name),
                ]))
            }
        }
    }

    pub async fn switch_provider(&self, name: &str) -> Result<()> {
        let clients = self.clients.read().await;
        if clients.contains_key(name) {
            let mut current = self.current_provider.write().await;
            *current = name.to_string();
            Ok(())
        } else {
            Err(LlmError::ModelNotFound(name.to_string()).into())
        }
    }

    pub async fn get_current_provider(&self) -> String {
        self.current_provider.read().await.clone()
    }

    pub async fn list_providers(&self) -> Vec<String> {
        self.clients.read().await.keys().cloned().collect()
    }

    pub async fn add_provider(&self, config: &LLMProviderConfig) {
        let client = Self::create_client(config);
        self.clients.write().await.insert(config.name.clone(), client);
    }

    async fn get_current_client(&self) -> Arc<dyn LLMClient> {
        let current = self.current_provider.read().await.clone();
        self.clients.read().await.get(&current).unwrap().clone()
    }
}

#[async_trait]
impl LLMClient for MultiLLMClient {
    async fn generate(&self, prompt: &str) -> Result<String> {
        let client = self.get_current_client().await;
        client.generate(prompt).await
    }

    async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        let client = self.get_current_client().await;
        client.chat(messages).await
    }

    fn set_model(&mut self, _model: &str) {
        // Not supported for multi-client
    }
}
