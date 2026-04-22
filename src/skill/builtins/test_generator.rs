use crate::error::Result;
use crate::skill::{ExecutionContext, SkillExecutor, SkillResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct TestGeneratorInput {
    pub file_path: String,
    pub function_name: String,
    pub language: Option<String>,
}

pub struct TestGeneratorSkill {
    llm_client: std::sync::Arc<dyn crate::llm::LLMClient>,
}

impl TestGeneratorSkill {
    pub fn new(llm_client: std::sync::Arc<dyn crate::llm::LLMClient>) -> Self {
        Self { llm_client }
    }
}

#[async_trait]
impl SkillExecutor for TestGeneratorSkill {
    async fn execute(
        &self,
        params: serde_json::Value,
        _context: &ExecutionContext,
    ) -> Result<SkillResult> {
        let input: TestGeneratorInput = serde_json::from_value(params)
            .map_err(|e| crate::error::SkillError::InvalidParameters(e.to_string()))?;

        let code = std::fs::read_to_string(&input.file_path)
            .map_err(|_| crate::error::AgentHubError::FileNotFound { path: input.file_path.clone() })?;

        let prompt = format!(
            "Generate comprehensive unit tests for the following {} function '{}':\n\n```{}\n{}\n```\n\nUse appropriate testing framework for the language.",
            input.language.as_deref().unwrap_or("code"),
            input.function_name,
            input.language.as_deref().unwrap_or("code"),
            code
        );

        let test_code = self.llm_client.generate(&prompt).await?;

        #[derive(Serialize)]
        struct TestOutput {
            file_path: String,
            function_name: String,
            test_code: String,
        }

        Ok(SkillResult {
            output: serde_json::to_value(TestOutput {
                file_path: input.file_path,
                function_name: input.function_name,
                test_code,
            })?,
            files_created: Vec::new(),
            warnings: Vec::new(),
        })
    }
}
