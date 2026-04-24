#[allow(dead_code)]
pub struct SemanticAnalyzerGuard;

#[allow(dead_code)]
impl SemanticAnalyzerGuard {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, _input: &str) -> Vec<String> {
        Vec::new()
    }
}
