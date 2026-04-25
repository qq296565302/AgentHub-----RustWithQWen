// =============================================================================
// SkillHub GitHub API 客户端
// =============================================================================
// 负责与 GitHub Releases API 交互，实现 Skill 的搜索、详情获取和下载。
//
// GitHub Releases API 文档:
//   https://docs.github.com/en/rest/releases/releases
//
// 命名约定:
//   GitHub 仓库中，每个 Release 代表一个 Skill。
//   Release tag 格式: {skill-name}-v{version}
//   例如: example-hello-world-v0.1.0
//   附件: {skill-name}-{version}.zip（包含 manifest.yaml + impl.wasm）
// =============================================================================

use crate::error::{AgentHubError, Result};
use crate::skill::skillhub::models::{
    GitHubReleaseList, SkillDetail, SkillManifestInfo, SkillSearchResult, SkillSummary,
};
use reqwest::Client;
use tracing::{debug, info};

/// SkillHub 仓库配置
/// 
/// 默认使用 AgentHub 官方的 Skills 仓库。
/// 用户可通过配置修改为其他仓库（如企业内部仓库）。
#[derive(Debug, Clone)]
pub struct SkillHubConfig {
    /// GitHub 仓库所有者，如 "AgentHub"
    pub owner: String,
    /// GitHub 仓库名称，如 "skills"
    pub repo: String,
    /// GitHub API Token（可选，用于提高 API 限流配额）
    pub token: Option<String>,
    /// API 基础 URL（默认: https://api.github.com）
    pub api_base_url: String,
    /// 请求超时（秒）
    pub timeout_secs: u64,
}

impl Default for SkillHubConfig {
    fn default() -> Self {
        Self {
            owner: "AgentHub".to_string(),
            repo: "skills".to_string(),
            token: None,
            api_base_url: "https://api.github.com".to_string(),
            timeout_secs: 30,
        }
    }
}

/// GitHub API 客户端
/// 
/// 封装所有与 GitHub Releases 的 HTTP 交互。
#[derive(Clone)]
pub struct SkillHubClient {
    config: SkillHubConfig,
    http_client: Client,
}

impl SkillHubClient {
    /// 创建新的客户端实例
    pub fn new(config: SkillHubConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .user_agent("AgentHub-SkillHub/0.1.0")
            .build()
            .map_err(|e| AgentHubError::Internal(format!("创建 HTTP 客户端失败: {}", e)))?;

        Ok(Self { config, http_client })
    }

    /// 创建默认配置的客户端
    pub fn with_default_config() -> Result<Self> {
        Self::new(SkillHubConfig::default())
    }

    // =====================================================================
    // 公开 API
    // =====================================================================

    /// 搜索 Skill
    /// 
    /// 通过关键词搜索 GitHub Releases，匹配 Release 名称和描述。
    /// 
    /// # 参数
    /// - `query`: 搜索关键词（如 "hello", "code formatter"）
    /// 
    /// # 返回
    /// 匹配的 Skill 摘要列表
    pub async fn search(&self, query: &str) -> Result<SkillSearchResult> {
        info!("Searching skills with query: '{}'", query);

        let releases = self.fetch_all_releases().await?;

        let skills = releases
            .into_iter()
            .filter(|r| self.matches_query(r, query))
            .map(|r| self.release_to_summary(&r))
            .collect();

        Ok(SkillSearchResult {
            skills,
            query: query.to_string(),
        })
    }

    /// 获取 Skill 详情
    /// 
    /// # 参数
    /// - `skill_id`: Skill 唯一标识（如 "example.hello.world"）
    /// 
    /// # 返回
    /// Skill 详细信息，包括所有可用版本和下载链接
    pub async fn get_skill_detail(&self, skill_id: &str) -> Result<SkillDetail> {
        info!("Fetching skill detail for: '{}'", skill_id);

        let releases = self.fetch_all_releases().await?;

        let matching_releases: Vec<_> = releases
            .into_iter()
            .filter(|r| self.release_matches_skill(r, skill_id))
            .collect();

        if matching_releases.is_empty() {
            return Err(AgentHubError::Internal(format!(
                "未找到 Skill: {}", skill_id
            )));
        }

        let latest = &matching_releases[0];
        let versions: Vec<String> = matching_releases
            .iter()
            .filter_map(|r| self.extract_version(r))
            .collect();

        let latest_asset = latest.assets.first().ok_or_else(|| {
            AgentHubError::Internal(format!("Skill '{}' 没有可用的下载附件", skill_id))
        })?;

        let version = self.extract_version(latest).unwrap_or_else(|| "unknown".to_string());

        Ok(SkillDetail {
            id: skill_id.to_string(),
            manifest: SkillManifestInfo {
                name: skill_id.to_string(),
                version: version.clone(),
                description: latest.name.clone(),
                author: "Unknown".to_string(),
                requires_write: false,
            },
            changelog: latest.body.clone(),
            available_versions: versions,
            download_url: latest_asset.url.clone(),
            file_size: latest_asset.size,
            html_url: latest.html_url.clone(),
        })
    }

    /// 下载 Skill 包
    /// 
    /// 从 GitHub 下载 Skill 的 ZIP 包。
    /// 
    /// # 参数
    /// - `download_url`: 从 SkillDetail 中获取的下载 URL
    /// 
    /// # 返回
    /// ZIP 文件的字节数据
    pub async fn download_skill(&self, download_url: &str) -> Result<Vec<u8>> {
        info!("Downloading skill from: {}", download_url);

        let response = self.http_client
            .get(download_url)
            .send()
            .await
            .map_err(|e| AgentHubError::Internal(format!("下载失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(AgentHubError::Internal(format!(
                "下载失败，HTTP 状态码: {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AgentHubError::Internal(format!("读取下载数据失败: {}", e)))?;

        info!("Downloaded {} bytes", bytes.len());
        Ok(bytes.to_vec())
    }

    // =====================================================================
    // 内部实现
    // =====================================================================

    /// 获取所有 Releases
    /// 
    /// GitHub API 分页返回，需要循环获取所有页面。
    async fn fetch_all_releases(&self) -> Result<Vec<GitHubReleaseList>> {
        let mut all_releases = Vec::new();
        let mut page = 1;
        let per_page = 30;

        loop {
            let url = format!(
                "{}/repos/{}/{}/releases?per_page={}&page={}",
                self.config.api_base_url, self.config.owner, self.config.repo, per_page, page
            );

            debug!("Fetching releases from: {}", url);

            let mut request = self.http_client.get(&url);

            if let Some(token) = &self.config.token {
                request = request.header("Authorization", format!("token {}", token));
            }

            let response = request
                .send()
                .await
                .map_err(|e| AgentHubError::Internal(format!("请求 GitHub API 失败: {}", e)))?;

            if !response.status().is_success() {
                return Err(AgentHubError::Internal(format!(
                    "GitHub API 请求失败，状态码: {} (URL: {})",
                    response.status(), url
                )));
            }

            let releases: Vec<GitHubReleaseList> = response
                .json()
                .await
                .map_err(|e| AgentHubError::Internal(format!("解析 GitHub API 响应失败: {}", e)))?;

            if releases.is_empty() {
                break;
            }

            all_releases.extend(releases);
            page += 1;

            if page > 10 {
                break;
            }
        }

        debug!("Fetched {} releases total", all_releases.len());
        Ok(all_releases)
    }

    /// 判断 Release 是否匹配搜索关键词
    fn matches_query(&self, release: &GitHubReleaseList, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        release.name.to_lowercase().contains(&query_lower)
            || release.body.to_lowercase().contains(&query_lower)
            || release.tag_name.to_lowercase().contains(&query_lower)
    }

    /// 判断 Release 是否匹配指定 Skill ID
    fn release_matches_skill(&self, release: &GitHubReleaseList, skill_id: &str) -> bool {
        let tag_without_version = self.strip_version_from_tag(&release.tag_name);
        let skill_id_normalized = skill_id.replace('.', "-");

        tag_without_version.eq_ignore_ascii_case(&skill_id_normalized)
            || release.tag_name.contains(skill_id)
    }

    /// 将 GitHub Release 转换为 Skill 摘要
    fn release_to_summary(&self, release: &GitHubReleaseList) -> SkillSummary {
        let id = self.tag_to_skill_id(&release.tag_name);
        let version = self.extract_version(release).unwrap_or_else(|| "unknown".to_string());

        let downloads = release.assets.iter().map(|a| a.size).sum::<u64>() / 1024;

        let changelog_preview = release
            .body
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n");

        SkillSummary {
            id,
            name: release.name.clone(),
            version,
            description: release.name.clone(),
            author: "Unknown".to_string(),
            downloads,
            changelog_preview,
            html_url: release.html_url.clone(),
        }
    }

    /// 从 Release 标签提取版本号
    /// 
    /// 标签格式: {skill-name}-v{version}
    /// 例如: example-hello-world-v0.1.0 → 0.1.0
    fn extract_version(&self, release: &GitHubReleaseList) -> Option<String> {
        let tag = &release.tag_name;

        if let Some(pos) = tag.rfind("-v") {
            Some(tag[pos + 2..].to_string())
        } else if let Some(pos) = tag.rfind('v') {
            Some(tag[pos + 1..].to_string())
        } else {
            None
        }
    }

    /// 从 Release 标签提取 Skill ID（不含版本号）
    /// 
    /// 例如: example-hello-world-v0.1.0 → example.hello.world
    fn tag_to_skill_id(&self, tag: &str) -> String {
        let name_part = self.strip_version_from_tag(tag);
        name_part.replace('-', ".")
    }

    /// 从标签中移除版本号部分
    /// 
    /// 例如: example-hello-world-v0.1.0 → example-hello-world
    fn strip_version_from_tag(&self, tag: &str) -> String {
        if let Some(pos) = tag.rfind("-v") {
            tag[..pos].to_string()
        } else {
            tag.to_string()
        }
    }
}
