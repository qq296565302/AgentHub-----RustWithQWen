use crate::error::Result;
use crate::skill::{ExecutionContext, SkillExecutor, SkillResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CodeExplainerInput {
    pub file_path: String,
    pub function_name: Option<String>,
    pub language: Option<String>,
}

pub struct CodeExplainerSkill {
    llm_client: std::sync::Arc<dyn crate::llm::LLMClient>,
}

impl CodeExplainerSkill {
    pub fn new(llm_client: std::sync::Arc<dyn crate::llm::LLMClient>) -> Self {
        Self { llm_client }
    }
}

#[async_trait]
impl SkillExecutor for CodeExplainerSkill {
    async fn execute(
        &self,
        params: serde_json::Value,
        _context: &ExecutionContext,
    ) -> Result<SkillResult> {
        let input: CodeExplainerInput = serde_json::from_value(params)
            .map_err(|e| crate::error::SkillError::InvalidParameters(e.to_string()))?;

        let code = std::fs::read_to_string(&input.file_path)
            .map_err(|_| crate::error::AgentHubError::FileNotFound { path: input.file_path.clone() })?;

        let prompt = format!(
            "Please explain the following {} code:\n\n```{}\n{}\n```\n\nProvide a clear explanation of what this code does, its key components, and any potential issues or improvements.",
            input.language.as_deref().unwrap_or("code"),
            input.language.as_deref().unwrap_or("code"),
            code
        );

        let explanation = self.llm_client.generate(&prompt).await?;

        #[derive(Serialize)]
        struct ExplanationOutput {
            file_path: String,
            explanation: String,
        }

        Ok(SkillResult {
            output: serde_json::to_value(ExplanationOutput {
                file_path: input.file_path,
                explanation,
            })?,
            files_created: Vec::new(),
            warnings: Vec::new(),
        })
    }
}
