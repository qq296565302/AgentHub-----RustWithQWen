use crate::llm::ChatMessage;
use crate::prompt::context_manager::ContextManager;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub message: ChatMessage,
    pub timestamp: DateTime<Utc>,
    pub token_count: usize,
}

#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub id: String,
    pub messages: Vec<ConversationMessage>,
    pub system_prompt: Option<String>,
    pub max_tokens: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ConversationContext {
    pub fn new(id: String, max_tokens: usize) -> Self {
        Self {
            id,
            messages: Vec::new(),
            system_prompt: None,
            max_tokens,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = Some(prompt);
        self
    }

    pub fn add_message(&mut self, message: ChatMessage, context_manager: &ContextManager) {
        let token_count = context_manager.count_tokens(&message.content);
        let conv_message = ConversationMessage {
            message,
            timestamp: Utc::now(),
            token_count,
        };
        self.messages.push(conv_message);
        self.updated_at = Utc::now();
        self.trim_to_context_window(context_manager);
    }

    pub fn get_messages_for_llm(&self) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        if let Some(ref system_prompt) = self.system_prompt {
            messages.push(ChatMessage::system(system_prompt));
        }

        messages.extend(
            self.messages
                .iter()
                .map(|cm| cm.message.clone())
                .collect::<Vec<_>>(),
        );

        messages
    }

    pub fn total_token_count(&self) -> usize {
        self.messages.iter().map(|m| m.token_count).sum()
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    fn trim_to_context_window(&mut self, _context_manager: &ContextManager) {
        let mut total_tokens: usize = self.messages.iter().map(|m| m.token_count).sum();

        while total_tokens > self.max_tokens && !self.messages.is_empty() {
            let removed = self.messages.remove(0);
            total_tokens -= removed.token_count;
        }
    }

    pub fn get_summary(&self) -> String {
        format!(
            "Conversation '{}' - {} messages, {} tokens",
            self.id,
            self.message_count(),
            self.total_token_count()
        )
    }
}

#[derive(Debug)]
pub struct ConversationManager {
    conversations: std::collections::HashMap<String, ConversationContext>,
    active_conversation: Option<String>,
    max_tokens: usize,
    context_manager: ContextManager,
}

impl ConversationManager {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            conversations: std::collections::HashMap::new(),
            active_conversation: None,
            max_tokens,
            context_manager: ContextManager::new(max_tokens),
        }
    }

    pub fn create_conversation(&mut self, id: Option<String>, system_prompt: Option<String>) -> String {
        let conv_id = id.unwrap_or_else(|| format!("conv_{}", uuid::Uuid::new_v4()));
        let mut conv = ConversationContext::new(conv_id.clone(), self.max_tokens);
        if let Some(prompt) = system_prompt {
            conv = conv.with_system_prompt(prompt);
        }
        self.conversations.insert(conv_id.clone(), conv);
        self.active_conversation = Some(conv_id.clone());
        conv_id
    }

    pub fn get_active(&self) -> Option<&ConversationContext> {
        if let Some(ref id) = self.active_conversation {
            self.conversations.get(id)
        } else {
            None
        }
    }

    pub fn get_active_mut(&mut self) -> Option<&mut ConversationContext> {
        if let Some(ref id) = self.active_conversation {
            self.conversations.get_mut(id)
        } else {
            None
        }
    }

    pub fn switch_conversation(&mut self, id: &str) -> Result<(), String> {
        if self.conversations.contains_key(id) {
            self.active_conversation = Some(id.to_string());
            Ok(())
        } else {
            Err(format!("Conversation '{}' not found", id))
        }
    }

    pub fn delete_conversation(&mut self, id: &str) -> Result<(), String> {
        if self.conversations.remove(id).is_some() {
            if self.active_conversation.as_deref() == Some(id) {
                self.active_conversation = self.conversations.keys().next().cloned();
            }
            Ok(())
        } else {
            Err(format!("Conversation '{}' not found", id))
        }
    }

    pub fn list_conversations(&self) -> Vec<&ConversationContext> {
        self.conversations.values().collect()
    }

    pub fn add_message_to_active(&mut self, message: ChatMessage) {
        if let Some(ref id) = self.active_conversation {
            if let Some(conv) = self.conversations.get_mut(id) {
                let context_manager = &self.context_manager;
                conv.add_message(message, context_manager);
            }
        }
    }

    pub fn get_messages_for_llm(&self) -> Option<Vec<ChatMessage>> {
        self.get_active().map(|c| c.get_messages_for_llm())
    }

    pub fn clear_active(&mut self) {
        if let Some(conv) = self.get_active_mut() {
            conv.clear();
        }
    }
}
