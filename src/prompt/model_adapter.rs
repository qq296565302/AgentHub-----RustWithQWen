use crate::error::Result;
use crate::llm::ChatMessage;

#[derive(Debug, Clone)]
pub enum ModelType {
    Qwen,
    Llama,
    GPT,
    Claude,
    Other(String),
}

impl ModelType {
    pub fn from_name(name: &str) -> Self {
        let name_lower = name.to_lowercase();
        if name_lower.contains("qwen") {
            ModelType::Qwen
        } else if name_lower.contains("llama") {
            ModelType::Llama
        } else if name_lower.contains("gpt") {
            ModelType::GPT
        } else if name_lower.contains("claude") {
            ModelType::Claude
        } else {
            ModelType::Other(name.to_string())
        }
    }

    pub fn system_prompt_prefix(&self) -> Option<&str> {
        match self {
            ModelType::Qwen => Some("You are Qwen, a helpful AI assistant."),
            ModelType::Llama => Some("You are a helpful AI assistant."),
            ModelType::GPT => Some("You are a helpful AI assistant."),
            ModelType::Claude => Some("You are Claude, a helpful AI assistant."),
            ModelType::Other(_) => None,
        }
    }
}

pub struct ModelAdapter {
    model_type: ModelType,
}

impl ModelAdapter {
    pub fn new(model_name: &str) -> Self {
        Self {
            model_type: ModelType::from_name(model_name),
        }
    }

    pub fn adapt_messages(&self, mut messages: Vec<ChatMessage>) -> Result<Vec<ChatMessage>> {
        if let Some(prefix) = self.model_type.system_prompt_prefix() {
            if messages.is_empty() || messages[0].role != "system" {
                messages.insert(0, ChatMessage::system(prefix));
            }
        }
        Ok(messages)
    }

    pub fn get_model_type(&self) -> &ModelType {
        &self.model_type
    }
}
