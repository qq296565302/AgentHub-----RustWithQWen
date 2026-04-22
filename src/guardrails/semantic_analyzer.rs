pub struct SemanticAnalyzerGuard;

impl SemanticAnalyzerGuard {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, _input: &str) -> Vec<String> {
        Vec::new()
    }
}
