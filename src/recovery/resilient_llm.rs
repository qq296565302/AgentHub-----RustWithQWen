use crate::error::Result;
use crate::llm::LLMClient;
use crate::recovery::circuit_breaker::CircuitBreaker;
use async_trait::async_trait;
use std::sync::Arc;

pub struct ResilientLLMClient {
    primary: Arc<dyn LLMClient>,
    fallback: Option<Arc<dyn LLMClient>>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl ResilientLLMClient {
    pub fn new(
        primary: Arc<dyn LLMClient>,
        fallback: Option<Arc<dyn LLMClient>>,
        circuit_breaker: Arc<CircuitBreaker>,
    ) -> Self {
        Self {
            primary,
            fallback,
            circuit_breaker,
        }
    }
}

#[async_trait]
impl LLMClient for ResilientLLMClient {
    async fn generate(&self, prompt: &str) -> Result<String> {
        if !self.circuit_breaker.is_available().await {
            if let Some(fallback) = &self.fallback {
                tracing::warn!("Circuit breaker open, using fallback");
                return fallback.generate(prompt).await;
            }
            return Err(crate::error::LlmError::CircuitOpen.into());
        }

        match self.primary.generate(prompt).await {
            Ok(response) => {
                self.circuit_breaker.record_success().await;
                Ok(response)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                if let Some(fallback) = &self.fallback {
                    tracing::warn!("Primary failed, using fallback: {}", e);
                    fallback.generate(prompt).await
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn chat(&self, messages: &[crate::llm::ChatMessage]) -> Result<String> {
        self.primary.chat(messages).await
    }

    fn set_model(&mut self, _model: &str) {
        // Cannot mutate Arc, this is a no-op for the resilient wrapper
        tracing::warn!("set_model is not supported for ResilientLLMClient");
    }
}
