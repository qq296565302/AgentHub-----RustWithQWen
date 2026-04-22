use crate::error::{AgentHubError, Result};
use crate::parser::CodeParser;
use std::collections::HashMap;
use std::sync::Arc;

pub struct LanguageRegistry {
    parsers: HashMap<String, Arc<dyn CodeParser>>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
        }
    }

    pub fn register(&mut self, language: String, parser: Arc<dyn CodeParser>) {
        self.parsers.insert(language, parser);
    }

    pub fn get_parser(&self, language: &str) -> Result<Arc<dyn CodeParser>> {
        self.parsers
            .get(language)
            .cloned()
            .ok_or_else(|| AgentHubError::UnsupportedLanguage(language.to_string()))
    }

    pub fn supported_languages(&self) -> Vec<String> {
        self.parsers.keys().cloned().collect()
    }
}
