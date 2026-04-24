use crate::error::{Result, AgentHubError};
use crate::skill::external::wasm_executor::WasmSkillConfig;
use crate::skill::{ExecutionContext, SkillExecutor};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStatus {
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub wasm_path: PathBuf,
    pub config: WasmSkillConfig,
}

pub struct SkillManager {
    skills_dir: PathBuf,
}

impl SkillManager {
    pub fn new(skills_dir: PathBuf) -> Self {
        Self { skills_dir }
    }

    pub fn list_skills(&self) -> Result<Vec<SkillStatus>> {
        let mut skills = Vec::new();

        if !self.skills_dir.exists() {
            return Ok(skills);
        }

        for entry in fs::read_dir(&self.skills_dir)
            .map_err(|e| AgentHubError::Internal(format!("无法读取 skills 目录: {}", e)))? {
            let entry = entry.map_err(|e| AgentHubError::Internal(format!("读取目录条目失败: {}", e)))?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            if let Ok(status) = self.get_skill_status(&path) {
                skills.push(status);
            }
        }

        Ok(skills)
    }

    pub fn enable_skill(&self, skill_name: &str) -> Result<()> {
        let skill_dir = self.find_skill_dir(skill_name)?;
        let config_path = skill_dir.join("skill_config.yaml");

        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| AgentHubError::Internal(format!("读取配置文件失败: {}", e)))?;
            serde_yaml::from_str::<WasmSkillConfig>(&content)
                .map_err(|e| AgentHubError::Internal(format!("解析配置文件失败: {}", e)))?
        } else {
            WasmSkillConfig::default()
        };

        let mut yaml_content = serde_yaml::to_string(&config)
            .map_err(|e| AgentHubError::Internal(format!("序列化配置失败: {}", e)))?;

        yaml_content.push_str("enabled: true\n");

        fs::write(&config_path, yaml_content)
            .map_err(|e| AgentHubError::Internal(format!("写入配置文件失败: {}", e)))?;

        info!("Skill '{}' enabled", skill_name);
        Ok(())
    }

    pub fn disable_skill(&self, skill_name: &str) -> Result<()> {
        let skill_dir = self.find_skill_dir(skill_name)?;
        let config_path = skill_dir.join("skill_config.yaml");

        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| AgentHubError::Internal(format!("读取配置文件失败: {}", e)))?;
            serde_yaml::from_str::<WasmSkillConfig>(&content)
                .map_err(|e| AgentHubError::Internal(format!("解析配置文件失败: {}", e)))?
        } else {
            WasmSkillConfig::default()
        };

        let mut yaml_content = serde_yaml::to_string(&config)
            .map_err(|e| AgentHubError::Internal(format!("序列化配置失败: {}", e)))?;

        yaml_content.push_str("enabled: false\n");

        fs::write(&config_path, yaml_content)
            .map_err(|e| AgentHubError::Internal(format!("写入配置文件失败: {}", e)))?;

        info!("Skill '{}' disabled", skill_name);
        Ok(())
    }

    pub async fn run_skill(&self, skill_name: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let skill_dir = self.find_skill_dir(skill_name)?;
        
        if !self.is_skill_enabled(&skill_dir) {
            return Err(AgentHubError::Internal(
                format!("Skill '{}' 已禁用，请先运行: agenthub skill enable {}", skill_name, skill_name)
            ));
        }

        let wasm_path = self.find_wasm_file(&skill_dir)?;
        let config = self.load_skill_config(&skill_dir)?;
        
        let executor = crate::skill::external::wasm_executor::WasmSkillExecutor::new(wasm_path, config)?;

        let context = ExecutionContext {
            user_id: "cli-user".to_string(),
            workspace_dir: std::env::current_dir().unwrap_or_default(),
        };

        let execute_result = executor.execute(params, &context).await;
        
        let result = match execute_result {
            Ok(r) => r,
            Err(e) => return Err(AgentHubError::Internal(format!("执行失败: {}", e))),
        };

        Ok(result.output)
    }

    pub fn get_skill_info(&self, skill_name: &str) -> Result<SkillStatus> {
        let skill_dir = self.find_skill_dir(skill_name)?;
        self.get_skill_status(&skill_dir)
    }

    fn find_skill_dir(&self, skill_name: &str) -> Result<PathBuf> {
        if !self.skills_dir.exists() {
            return Err(AgentHubError::Internal(format!("Skills 目录不存在: {:?}", self.skills_dir)));
        }

        for entry in fs::read_dir(&self.skills_dir)
            .map_err(|e| AgentHubError::Internal(format!("读取 skills 目录失败: {}", e)))? {
            let entry = entry.map_err(|e| AgentHubError::Internal(format!("读取目录条目失败: {}", e)))?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("manifest.yaml");
            if manifest_path.exists() {
                if let Ok(content) = fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = serde_yaml::from_str::<crate::skill::SkillManifest>(&content) {
                        if manifest.name == skill_name {
                            return Ok(path);
                        }
                    }
                }
            }
        }

        Err(AgentHubError::Internal(format!("未找到 Skill: {}", skill_name)))
    }

    fn get_skill_status(&self, skill_dir: &Path) -> Result<SkillStatus> {
        let manifest_path = skill_dir.join("manifest.yaml");
        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| AgentHubError::Internal(format!("读取 manifest 失败: {}", e)))?;
        
        let manifest: crate::skill::SkillManifest = serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::Internal(format!("解析 manifest 失败: {}", e)))?;

        let wasm_path = self.find_wasm_file(skill_dir)?;
        let config = self.load_skill_config(skill_dir)?;
        let enabled = self.is_skill_enabled(skill_dir);

        Ok(SkillStatus {
            name: manifest.name,
            version: manifest.version,
            description: manifest.description,
            enabled,
            wasm_path,
            config,
        })
    }

    fn is_skill_enabled(&self, skill_dir: &Path) -> bool {
        let config_path = skill_dir.join("skill_config.yaml");
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                    return config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                }
            }
        }
        true
    }

    fn find_wasm_file(&self, skill_dir: &Path) -> Result<PathBuf> {
        let wasm_patterns = vec!["impl.wasm", "skill.wasm", "main.wasm"];

        for pattern in &wasm_patterns {
            let path = skill_dir.join(pattern);
            if path.exists() {
                return Ok(path);
            }
        }

        for entry in fs::read_dir(skill_dir)
            .map_err(|e| AgentHubError::Internal(format!("读取 skill 目录失败: {}", e)))? {
            let entry = entry.map_err(|e| AgentHubError::Internal(format!("读取目录条目失败: {}", e)))?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "wasm") {
                return Ok(path);
            }
        }

        Err(AgentHubError::Internal(format!(
            "在 skill 目录中未找到 WASM 文件: {:?}", skill_dir
        )))
    }

    fn load_skill_config(&self, skill_dir: &Path) -> Result<WasmSkillConfig> {
        let config_path = skill_dir.join("skill_config.yaml");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| AgentHubError::Internal(format!("读取配置文件失败: {}", e)))?;

            let config: WasmSkillConfig = serde_yaml::from_str(&content)
                .map_err(|e| AgentHubError::Internal(format!("解析配置文件失败: {}", e)))?;

            Ok(config)
        } else {
            Ok(WasmSkillConfig::default())
        }
    }
}
