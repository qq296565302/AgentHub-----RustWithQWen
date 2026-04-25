// =============================================================================
// SkillHub 数据模型
// =============================================================================
// 定义与 GitHub Releases 交互所需的数据结构。
// GitHub Releases API 返回的 JSON 结构映射为 Rust 类型。
// =============================================================================

use serde::{Deserialize, Serialize};

// =========================================================================
// GitHub Releases API 响应模型
// =========================================================================

/// GitHub Release 列表响应
/// 
/// 对应 GitHub API: GET /repos/{owner}/{repo}/releases
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubReleaseList {
    /// Release 标签名，如 "v0.2.0"
    pub tag_name: String,
    /// Release 名称，如 "skill-hello-world v0.2.0"
    pub name: String,
    /// 发布说明（changelog）
    pub body: String,
    /// 下载附件列表
    pub assets: Vec<GitHubAsset>,
    /// HTML 链接
    pub html_url: String,
}

/// GitHub Release 附件（即 Skill 包文件）
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    /// 下载 URL
    pub url: String,
    /// 文件大小（字节）
    pub size: u64,
    /// SHA256 校验和
    pub digest: Option<String>,
}

// =========================================================================
// SkillHub 业务模型
// =========================================================================

/// Skill 摘要（用于搜索列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSummary {
    /// Skill 唯一标识，如 "example.hello.world"
    pub id: String,
    /// Skill 名称
    pub name: String,
    /// 当前版本号
    pub version: String,
    /// 简短描述
    pub description: String,
    /// 作者
    pub author: String,
    /// 下载次数
    pub downloads: u64,
    /// 最新发布说明
    pub changelog_preview: String,
    /// GitHub Release 页面 URL
    pub html_url: String,
}

/// Skill 详情（用于安装前查看）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetail {
    /// Skill 唯一标识
    pub id: String,
    /// 完整 manifest 信息
    pub manifest: SkillManifestInfo,
    /// 完整 changelog
    pub changelog: String,
    /// 所有可用版本
    pub available_versions: Vec<String>,
    /// 下载 URL
    pub download_url: String,
    /// 文件大小
    pub file_size: u64,
    /// GitHub Release 页面 URL
    pub html_url: String,
}

/// Manifest 信息（从 Release 名称/标签解析或从包内读取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifestInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub requires_write: bool,
}

/// 版本更新信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpdateInfo {
    /// Skill 名称
    pub name: String,
    /// 当前已安装版本
    pub current_version: String,
    /// 最新可用版本
    pub latest_version: String,
    /// 更新说明
    pub changelog: String,
    /// 是否兼容（主版本号相同视为兼容）
    pub is_compatible: bool,
}

/// SkillHub 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSearchResult {
    /// 匹配的 Skill 列表
    pub skills: Vec<SkillSummary>,
    /// 搜索关键词
    pub query: String,
}
