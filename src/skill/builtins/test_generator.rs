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

    fn resolve_file_path(&self, file_path: &str, workspace_dir: &std::path::Path) -> Result<Vec<String>> {
        let path = std::path::Path::new(file_path);
        if path.exists() {
            return Ok(vec![file_path.to_string()]);
        }

        let file_name = path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().to_string();
        let mut search_dirs = vec![workspace_dir.to_path_buf()];

        for entry in &["src", "lib", "app", "core", "common"] {
            let dir = workspace_dir.join(entry);
            if dir.is_dir() {
                search_dirs.push(dir);
            }
        }

        let mut matches = Vec::new();
        for dir in &search_dirs {
            self.collect_matches(&dir, &file_name, &mut matches);
        }

        if matches.is_empty() {
            Err(crate::error::AgentHubError::FileNotFound { path: file_path.to_string() })
        } else {
            Ok(matches)
        }
    }

    fn collect_matches(&self, dir: &std::path::Path, file_name: &str, matches: &mut Vec<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    if let Some(name) = entry_path.file_name() {
                        if name.to_string_lossy() == file_name {
                            matches.push(entry_path.to_string_lossy().to_string());
                        }
                    }
                } else if entry_path.is_dir() {
                    self.collect_matches(&entry_path, file_name, matches);
                }
            }
        }
    }
}

#[async_trait]
impl SkillExecutor for TestGeneratorSkill {
    async fn execute(
        &self,
        params: serde_json::Value,
        context: &ExecutionContext,
    ) -> Result<SkillResult> {
        let input: TestGeneratorInput = serde_json::from_value(params)
            .map_err(|e| crate::error::SkillError::InvalidParameters(e.to_string()))?;

        let matches = self.resolve_file_path(&input.file_path, &context.workspace_dir)?;

        if matches.len() > 1 {
            return Err(crate::error::AgentHubError::AmbiguousFile {
                name: input.file_path.clone(),
                paths: matches,
            }.into());
        }

        let resolved_path = matches.into_iter().next()
            .ok_or_else(|| crate::error::AgentHubError::FileNotFound { path: input.file_path.clone() })?;

        let code = std::fs::read_to_string(&resolved_path)
            .map_err(|_| crate::error::AgentHubError::FileNotFound { path: resolved_path.clone() })?;

        let prompt = format!(
            "请为以下{}代码中的函数 '{}' 生成完整的单元测试：\n\n```{}\n{}\n```\n\n请使用该语言合适的测试框架，包含边界条件和异常情况测试。",
            input.language.as_deref().unwrap_or(""),
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
                file_path: resolved_path,
                function_name: input.function_name,
                test_code,
            })?,
            files_created: Vec::new(),
            warnings: Vec::new(),
        })
    }
}
