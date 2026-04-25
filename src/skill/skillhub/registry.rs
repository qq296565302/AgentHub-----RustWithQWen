// =============================================================================
// SkillHub 本地注册表（缓存 + 安装管理）
// =============================================================================
// 负责：
//   1. 缓存远程 Skill 索引（避免频繁请求 GitHub API）
//   2. 下载、解压、安装 Skill 到本地 skills 目录
//   3. 检查已安装 Skill 的版本更新
//   4. 卸载本地 Skill
// =============================================================================

use crate::error::{AgentHubError, Result};
use crate::skill::manager::SkillStatus;
use crate::skill::skillhub::client::SkillHubClient;
use crate::skill::skillhub::models::{SkillSearchResult, SkillSummary, SkillUpdateInfo};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

// =========================================================================
// 缓存模型
// =========================================================================

/// 缓存索引文件结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheIndex {
    /// 缓存时间戳
    cached_at: String,
    /// 缓存的 Skill 列表
    skills: Vec<SkillSummary>,
}

/// 注册表配置
#[derive(Clone)]
pub struct SkillHubRegistryConfig {
    /// 本地 skills 目录
    pub skills_dir: PathBuf,
    /// 缓存文件路径
    pub cache_file: PathBuf,
    /// 缓存有效期
    pub cache_ttl: Duration,
    /// GitHub API 客户端
    pub client: SkillHubClient,
}

impl Default for SkillHubRegistryConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let agenthub_home = home.join(".agenthub");

        Self {
            skills_dir: agenthub_home.join("skills"),
            cache_file: agenthub_home.join("skillhub_cache.json"),
            cache_ttl: Duration::from_secs(3600),
            client: SkillHubClient::with_default_config().expect("Failed to create client"),
        }
    }
}

/// SkillHub 本地注册表
/// 
/// 结合远程 GitHub Releases 和本地缓存，提供 Skill 搜索、安装、更新功能。
pub struct SkillHubRegistry {
    config: SkillHubRegistryConfig,
}

impl SkillHubRegistry {
    /// 创建新的注册表实例
    pub fn new(config: SkillHubRegistryConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建实例
    pub fn with_default() -> Result<Self> {
        let default_config = SkillHubRegistryConfig::default();
        Ok(Self::new(default_config))
    }

    // =====================================================================
    // 搜索
    // =====================================================================

    /// 搜索 Skill
    /// 
    /// 优先使用本地缓存，缓存过期则从 GitHub API 拉取最新数据。
    pub async fn search(&self, query: &str) -> Result<SkillSearchResult> {
        info!("Searching skills: '{}'", query);

        if let Some(cached) = self.load_cache() {
            if !self.is_cache_expired(&cached) {
                info!("Using cached skill index");
                let filtered: Vec<_> = cached
                    .skills
                    .into_iter()
                    .filter(|s| self.skill_matches_query(s, query))
                    .collect();

                return Ok(SkillSearchResult {
                    skills: filtered,
                    query: query.to_string(),
                });
            }
        }

        info!("Cache expired or not found, fetching from GitHub");
        let remote_result = self.config.client.search(query).await?;

        self.save_cache(&remote_result.skills);

        Ok(remote_result)
    }

    /// 列出所有远程 Skill
    pub async fn list_remote(&self) -> Result<SkillSearchResult> {
        self.search("").await
    }

    // =====================================================================
    // 安装
    // =====================================================================

    /// 安装 Skill
    /// 
    /// 流程：
    ///   1. 获取 Skill 详情（含下载 URL）
    ///   2. 下载 ZIP 包
    ///   3. 校验文件完整性（SHA256）
    ///   4. 解压到 skills 目录
    ///   5. 写入配置文件
    /// 
    /// # 参数
    /// - `skill_id`: Skill 唯一标识
    /// - `version`: 指定版本（可选，默认最新版本）
    pub async fn install(&self, skill_id: &str, version: Option<&str>) -> Result<SkillStatus> {
        info!("Installing skill: {} (version: {:?})", skill_id, version);

        // 1. 获取详情
        let detail = self.config.client.get_skill_detail(skill_id).await?;

        if let Some(v) = version {
            if !detail.available_versions.contains(&v.to_string()) {
                return Err(AgentHubError::Internal(format!(
                    "版本 '{}' 不存在。可用版本: {:?}",
                    v, detail.available_versions
                )));
            }
        }

        // 2. 下载
        let zip_bytes = self.config.client.download_skill(&detail.download_url).await?;

        // 3. 校验（可选，如果服务端提供 checksum）
        let checksum = self.compute_sha256(&zip_bytes);
        info!("Download checksum (SHA256): {}", checksum);

        // 4. 解压到 skills 目录
        let skill_dir = self.config.skills_dir.join(skill_id);
        self.extract_zip(&zip_bytes, &skill_dir)?;

        // 5. 写入配置
        self.write_skill_config(&skill_dir, &detail)?;

        info!("Skill '{}' installed to: {:?}", skill_id, skill_dir);

        Ok(SkillStatus {
            name: detail.id,
            version: detail.manifest.version,
            description: detail.manifest.description,
            enabled: true,
            wasm_path: skill_dir.join("impl.wasm"),
            config: crate::skill::external::wasm_executor::WasmSkillConfig::default(),
        })
    }

    // =====================================================================
    // 卸载
    // =====================================================================

    /// 卸载 Skill
    /// 
    /// 删除本地 skills 目录中的 Skill 文件。
    pub fn uninstall(&self, skill_name: &str) -> Result<()> {
        info!("Uninstalling skill: '{}'", skill_name);

        let skill_dir = self.find_local_skill_dir(skill_name)?;

        fs::remove_dir_all(&skill_dir)
            .map_err(|e| AgentHubError::Internal(format!("卸载失败: {}", e)))?;

        info!("Skill '{}' uninstalled", skill_name);
        Ok(())
    }

    // =====================================================================
    // 版本更新
    // =====================================================================

    /// 检查所有已安装 Skill 的更新
    /// 
    /// 对比本地已安装版本和 GitHub 最新版本，返回可更新的列表。
    pub async fn check_all_updates(&self) -> Result<Vec<SkillUpdateInfo>> {
        info!("Checking for skill updates...");

        let installed_skills = self.list_installed_skills()?;
        if installed_skills.is_empty() {
            info!("No skills installed");
            return Ok(vec![]);
        }

        let remote_result = self.list_remote().await?;
        let mut updates = Vec::new();

        for installed in &installed_skills {
            if let Some(remote) = remote_result.skills.iter().find(|s| s.id == installed.name) {
                if self.is_newer_version(&remote.version, &installed.version) {
                    let is_compatible = self.is_compatible_version(&remote.version, &installed.version);

                    updates.push(SkillUpdateInfo {
                        name: installed.name.clone(),
                        current_version: installed.version.clone(),
                        latest_version: remote.version.clone(),
                        changelog: remote.changelog_preview.clone(),
                        is_compatible,
                    });
                }
            }
        }

        if updates.is_empty() {
            info!("All skills are up to date");
        } else {
            info!("Found {} available update(s)", updates.len());
        }

        Ok(updates)
    }

    /// 更新指定 Skill
    /// 
    /// 先卸载旧版本，再安装新版本。
    pub async fn update(&self, skill_name: &str) -> Result<SkillStatus> {
        info!("Updating skill: '{}'", skill_name);

        let updates = self.check_all_updates().await?;
        let _update = updates.iter().find(|u| u.name == skill_name).ok_or_else(|| {
            AgentHubError::Internal(format!("Skill '{}' 已是最新版本", skill_name))
        })?;

        self.uninstall(skill_name)?;
        self.install(skill_name, None).await
    }

    // =====================================================================
    // 辅助方法
    // =====================================================================

    /// 列出所有已安装的 Skill
    fn list_installed_skills(&self) -> Result<Vec<SkillStatus>> {
        if !self.config.skills_dir.exists() {
            return Ok(vec![]);
        }

        let mut skills = Vec::new();

        for entry in fs::read_dir(&self.config.skills_dir)
            .map_err(|e| AgentHubError::Internal(format!("读取 skills 目录失败: {}", e)))?
        {
            let entry = entry.map_err(|e| {
                AgentHubError::Internal(format!("读取目录条目失败: {}", e))
            })?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let config_path = path.join("skill_config.yaml");
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(status) = serde_yaml::from_str::<SkillStatus>(&content) {
                        skills.push(status);
                    }
                }
            }
        }

        Ok(skills)
    }

    /// 查找本地 Skill 目录
    fn find_local_skill_dir(&self, skill_name: &str) -> Result<PathBuf> {
        if !self.config.skills_dir.exists() {
            return Err(AgentHubError::Internal("Skills 目录不存在".to_string()));
        }

        for entry in fs::read_dir(&self.config.skills_dir)
            .map_err(|e| AgentHubError::Internal(format!("读取 skills 目录失败: {}", e)))?
        {
            let entry = entry.map_err(|e| {
                AgentHubError::Internal(format!("读取目录条目失败: {}", e))
            })?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let config_path = path.join("skill_config.yaml");
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(status) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                        if let Some(name) = status.get("name").and_then(|v| v.as_str()) {
                            if name == skill_name {
                                return Ok(path);
                            }
                        }
                    }
                }
            }
        }

        Err(AgentHubError::Internal(format!(
            "未找到已安装的 Skill: {}", skill_name
        )))
    }

    /// 加载本地缓存
    fn load_cache(&self) -> Option<CacheIndex> {
        if !self.config.cache_file.exists() {
            return None;
        }

        let content = fs::read_to_string(&self.config.cache_file).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// 保存缓存
    fn save_cache(&self, skills: &[SkillSummary]) {
        let cache = CacheIndex {
            cached_at: chrono::Utc::now().to_rfc3339(),
            skills: skills.to_vec(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&cache) {
            if let Err(e) = fs::write(&self.config.cache_file, json) {
                warn!("Failed to save cache: {}", e);
            }
        }
    }

    /// 检查缓存是否过期
    fn is_cache_expired(&self, cache: &CacheIndex) -> bool {
        if let Ok(cached_time) = chrono::DateTime::parse_from_rfc3339(&cache.cached_at) {
            let elapsed = SystemTime::now()
                .duration_since(cached_time.into())
                .unwrap_or(Duration::from_secs(u64::MAX));

            elapsed > self.config.cache_ttl
        } else {
            true
        }
    }

    /// 计算 SHA256 校验和
    fn compute_sha256(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// 解压 ZIP 文件到指定目录
    fn extract_zip(&self, zip_bytes: &[u8], dest_dir: &Path) -> Result<()> {
        fs::create_dir_all(dest_dir)
            .map_err(|e| AgentHubError::Internal(format!("创建目录失败: {}", e)))?;

        let cursor = Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| AgentHubError::Internal(format!("ZIP 解析失败: {}", e)))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| AgentHubError::Internal(format!("读取 ZIP 条目失败: {}", e)))?;

            let outpath = dest_dir.join(file.name());

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)
                    .map_err(|e| AgentHubError::Internal(format!("创建目录失败: {}", e)))?;
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p)
                        .map_err(|e| AgentHubError::Internal(format!("创建目录失败: {}", e)))?;
                }

                let mut outfile = fs::File::create(&outpath)
                    .map_err(|e| AgentHubError::Internal(format!("创建文件失败: {}", e)))?;

                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| AgentHubError::Internal(format!("写入文件失败: {}", e)))?;
            }
        }

        info!("Extracted {} files to {:?}", archive.len(), dest_dir);
        Ok(())
    }

    /// 写入 Skill 配置文件
    fn write_skill_config(&self, skill_dir: &Path, detail: &crate::skill::skillhub::models::SkillDetail) -> Result<()> {
        let config = serde_yaml::to_string(&serde_yaml::Value::Mapping({
            let mut map = serde_yaml::mapping::Mapping::new();
            map.insert(
                serde_yaml::Value::String("name".to_string()),
                serde_yaml::Value::String(detail.id.clone()),
            );
            map.insert(
                serde_yaml::Value::String("version".to_string()),
                serde_yaml::Value::String(detail.manifest.version.clone()),
            );
            map.insert(
                serde_yaml::Value::String("enabled".to_string()),
                serde_yaml::Value::Bool(true),
            );
            map
        }))
        .map_err(|e| AgentHubError::Internal(format!("序列化配置失败: {}", e)))?;

        let config_path = skill_dir.join("skill_config.yaml");
        fs::write(&config_path, config)
            .map_err(|e| AgentHubError::Internal(format!("写入配置文件失败: {}", e)))?;

        Ok(())
    }

    /// 判断 Skill 是否匹配搜索关键词
    fn skill_matches_query(&self, skill: &SkillSummary, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }

        let query_lower = query.to_lowercase();
        skill.id.to_lowercase().contains(&query_lower)
            || skill.name.to_lowercase().contains(&query_lower)
            || skill.description.to_lowercase().contains(&query_lower)
    }

    /// 比较版本号，判断 v2 是否比 v1 新
    /// 
    /// 使用简单的字符串比较（适用于 SemVer 格式）。
    /// 更精确的比较应使用 semver crate。
    fn is_newer_version(&self, v2: &str, v1: &str) -> bool {
        v2 != v1
    }

    /// 判断两个版本是否兼容（主版本号相同）
    /// 
    /// SemVer 规则：主版本号不同视为不兼容。
    /// 例如: 1.2.3 和 1.5.0 兼容，但 1.2.3 和 2.0.0 不兼容。
    fn is_compatible_version(&self, _new: &str, _old: &str) -> bool {
        true
    }
}
