use crate::error::{Result, AgentHubError};
use crate::skill::{SkillRegistry, SkillManifest};
use crate::skill::external::wasm_executor::{WasmSkillExecutor, WasmSkillConfig};
use std::path::{Path, PathBuf};
use std::fs;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct ExternalSkillConfig {
    pub skills_dir: PathBuf,
    pub default_wasm_config: WasmSkillConfig,
    #[allow(dead_code)]
    pub auto_load: bool,
}

impl Default for ExternalSkillConfig {
    fn default() -> Self {
        let skills_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("skills");

        Self {
            skills_dir,
            default_wasm_config: WasmSkillConfig::default(),
            auto_load: true,
        }
    }
}

pub struct ExternalSkillLoader {
    config: ExternalSkillConfig,
}

impl ExternalSkillLoader {
    pub fn new(config: ExternalSkillConfig) -> Self {
        Self { config }
    }

    pub fn load_all_skills(&self, registry: &mut SkillRegistry) -> Result<usize> {
        if !self.config.skills_dir.exists() {
            info!("External skills directory does not exist: {:?}", self.config.skills_dir);
            return Ok(0);
        }

        let mut loaded_count = 0;

        let entries = fs::read_dir(&self.config.skills_dir)
            .map_err(|e| AgentHubError::Internal(format!("Failed to read skills directory: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| AgentHubError::Internal(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            match self.load_single_skill(&path, registry) {
                Ok(skill_name) => {
                    info!("Loaded external skill: {}", skill_name);
                    loaded_count += 1;
                }
                Err(e) => {
                    warn!("Failed to load skill from {:?}: {}", path, e);
                }
            }
        }

        info!("Loaded {} external skills from {:?}", loaded_count, self.config.skills_dir);
        Ok(loaded_count)
    }

    fn load_single_skill(&self, skill_dir: &Path, registry: &mut SkillRegistry) -> Result<String> {
        let manifest_path = skill_dir.join("manifest.yaml");
        if !manifest_path.exists() {
            return Err(AgentHubError::FileNotFound { 
                path: manifest_path.to_string_lossy().to_string() 
            });
        }

        let manifest_content = fs::read_to_string(&manifest_path)
            .map_err(|_e| AgentHubError::FileReadError { 
                path: manifest_path.to_string_lossy().to_string() 
            })?;

        let manifest: SkillManifest = serde_yaml::from_str(&manifest_content)
            .map_err(|e| AgentHubError::ParseError { 
                message: format!("Invalid manifest: {}", e) 
            })?;

        if !self.is_skill_enabled(skill_dir) {
            info!("Skipping disabled skill: {}", manifest.name);
            return Ok(manifest.name);
        }

        let wasm_path = self.find_wasm_file(skill_dir)?;
        
        let wasm_config = self.load_skill_config(skill_dir)?;
        
        let executor = WasmSkillExecutor::new(wasm_path, wasm_config)?;

        registry.register(
            manifest.name.clone(),
            manifest.version.clone(),
            manifest.description.clone(),
            Box::new(executor),
        );

        Ok(manifest.name)
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
        let wasm_patterns = vec![
            "impl.wasm",
            "skill.wasm",
            "main.wasm",
        ];

        for pattern in &wasm_patterns {
            let path = skill_dir.join(pattern);
            if path.exists() {
                return Ok(path);
            }
        }

        for entry in fs::read_dir(skill_dir)
            .map_err(|e| AgentHubError::Internal(format!("Failed to read skill directory: {}", e)))? {
            let entry = entry.map_err(|e| AgentHubError::Internal(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "wasm") {
                return Ok(path);
            }
        }

        Err(AgentHubError::Internal(format!(
            "No WASM file found in skill directory: {:?}", skill_dir
        )))
    }

    fn load_skill_config(&self, skill_dir: &Path) -> Result<WasmSkillConfig> {
        let config_path = skill_dir.join("skill_config.yaml");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|_e| AgentHubError::FileReadError { 
                    path: config_path.to_string_lossy().to_string() 
                })?;

            let config: WasmSkillConfig = serde_yaml::from_str(&content)
                .map_err(|e| AgentHubError::ParseError { 
                    message: format!("Invalid skill config: {}", e) 
                })?;

            Ok(config)
        } else {
            Ok(self.config.default_wasm_config.clone())
        }
    }

    #[allow(dead_code)]
    pub fn list_available_skills(&self) -> Result<Vec<SkillManifest>> {
        let mut skills = Vec::new();

        if !self.config.skills_dir.exists() {
            return Ok(skills);
        }

        for entry in fs::read_dir(&self.config.skills_dir)
            .map_err(|e| AgentHubError::Internal(format!("Failed to read skills directory: {}", e)))? {
            let entry = entry.map_err(|e| AgentHubError::Internal(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("manifest.yaml");
            if manifest_path.exists() {
                if let Ok(content) = fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = serde_yaml::from_str::<SkillManifest>(&content) {
                        skills.push(manifest);
                    }
                }
            }
        }

        Ok(skills)
    }
}
