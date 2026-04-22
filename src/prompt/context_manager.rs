use tiktoken_rs::cl100k_base;

#[derive(Debug, Clone)]
pub struct ContextManager {
    max_tokens: usize,
}

impl ContextManager {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
        }
    }

    pub fn count_tokens(&self, text: &str) -> usize {
        let bpe = cl100k_base().unwrap();
        bpe.encode_ordinary(text).len()
    }

    pub fn truncate_to_context_window(&self, text: &str) -> String {
        let bpe = cl100k_base().unwrap();
        let tokens = bpe.encode_ordinary(text);
        if tokens.len() <= self.max_tokens {
            return text.to_string();
        }

        let truncated_tokens = &tokens[..self.max_tokens];
        bpe.decode(truncated_tokens.to_vec()).unwrap_or_default()
    }

    pub fn should_truncate(&self, text: &str) -> bool {
        self.count_tokens(text) > self.max_tokens
    }

    pub fn get_available_tokens(&self, used_tokens: usize) -> usize {
        self.max_tokens.saturating_sub(used_tokens)
    }
}
