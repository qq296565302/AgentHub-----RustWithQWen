pub mod templates;
pub mod context_manager;
pub mod model_adapter;
pub mod conversation;

pub use conversation::{ConversationManager, ConversationContext};
pub use context_manager::ContextManager;

use handlebars::Handlebars;

pub struct PromptEngine {
    handlebars: Handlebars<'static>,
}

impl PromptEngine {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);
        Self { handlebars }
    }

    pub fn register_template(&mut self, name: &str, template: &str) {
        self.handlebars
            .register_template_string(name, template)
            .expect("Failed to register template");
    }

    pub fn render(&self, template_name: &str, data: &serde_json::Value) -> Result<String, String> {
        self.handlebars
            .render(template_name, data)
            .map_err(|e| e.to_string())
    }
}
