pub mod builtins;
pub mod external;
pub mod manager;
pub mod skillhub;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::error::Result;

#[async_trait]
pub trait SkillExecutor: Send + Sync {
    async fn execute(
        &self,
        params: serde_json::Value,
        context: &ExecutionContext,
    ) -> Result<SkillResult>;
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    #[allow(dead_code)]
    pub user_id: String,
    pub workspace_dir: std::path::PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillResult {
    pub output: serde_json::Value,
    pub files_created: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub interfaces: Vec<String>,
    pub requires_write: bool,
}

pub struct SkillInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub executor: Box<dyn SkillExecutor>,
}

impl std::fmt::Debug for SkillInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillInfo")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("description", &self.description)
            .finish()
    }
}

pub struct SkillRegistry {
    skills: std::collections::HashMap<String, SkillInfo>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, version: String, description: String, executor: Box<dyn SkillExecutor>) {
        self.skills.insert(name.clone(), SkillInfo {
            name,
            version,
            description,
            executor,
        });
    }

    pub fn get(&self, name: &str) -> Option<&dyn SkillExecutor> {
        self.skills.get(name).map(|s| s.executor.as_ref())
    }

    pub fn list_skills(&self) -> Vec<&SkillInfo> {
        self.skills.values().collect()
    }
}
