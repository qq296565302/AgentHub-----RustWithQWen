use config::{Config as CfgConfig, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub llm: LLMConfig,
    pub security: SecurityConfig,
    pub cache: CacheConfig,
    pub audit: AuditConfig,
    pub retry: RetryConfig,
    pub timeouts: TimeoutConfig,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct LLMProviderConfig {
    pub name: String,
    pub nickname: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub api_key: Option<String>,
    pub model: String,
    #[serde(rename = "api_endpoint")]
    pub api_endpoint: String,
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_max_tokens() -> usize { 4096 }
fn default_timeout_secs() -> u64 { 120 }

impl Default for LLMProviderConfig {
    fn default() -> Self {
        Self {
            name: "ollama".to_string(),
            nickname: None,
            description: None,
            provider_type: "ollama".to_string(),
            api_key: None,
            model: "qwen:4b".to_string(),
            api_endpoint: "http://localhost:11434".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            timeout_secs: 120,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct LLMConfig {
    pub default_provider: String,
    pub providers: Vec<LLMProviderConfig>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            default_provider: "ollama".to_string(),
            providers: vec![
                LLMProviderConfig {
                    name: "ollama".to_string(),
                    nickname: Some("本地模型".to_string()),
                    provider_type: "ollama".to_string(),
                    model: "qwen:4b".to_string(),
                    api_endpoint: "http://localhost:11434".to_string(),
                    ..Default::default()
                },
                LLMProviderConfig {
                    name: "openai".to_string(),
                    nickname: Some("OpenAI".to_string()),
                    provider_type: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    api_endpoint: "https://api.openai.com/v1".to_string(),
                    api_key: None,
                    ..Default::default()
                },
            ],
        }
    }
}

impl LLMConfig {
    pub fn get_provider(&self, name: &str) -> Option<&LLMProviderConfig> {
        self.providers.iter().find(|p| p.name == name)
    }

    pub fn get_default_provider(&self) -> Option<&LLMProviderConfig> {
        self.providers.iter().find(|p| p.name == self.default_provider)
    }

    pub fn list_providers(&self) -> Vec<&str> {
        self.providers.iter().map(|p| p.name.as_str()).collect()
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SecurityConfig {
    #[serde(default = "default_max_input_length")]
    pub max_input_length: usize,
    
    #[serde(default)]
    pub pii_detection: PIIDetectionConfig,
    
    #[serde(default)]
    pub prompt_injection: PromptInjectionConfig,
    
    #[serde(default)]
    pub file_access: FileAccessConfig,
    
    #[serde(default)]
    pub network: NetworkConfig,
    
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    
    #[serde(default)]
    pub output_sanitizer: OutputSanitizerConfig,
}

fn default_max_input_length() -> usize { 50000 }

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct PIIDetectionConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub redact_emails: bool,
    #[serde(default = "default_true")]
    pub redact_phones: bool,
    #[serde(default = "default_true")]
    pub redact_id_cards: bool,
    #[serde(default)]
    pub redact_ips: bool,
    #[serde(default = "default_true")]
    pub redact_api_keys: bool,
}

impl Default for PIIDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            redact_emails: true,
            redact_phones: true,
            redact_id_cards: true,
            redact_ips: false,
            redact_api_keys: true,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct PromptInjectionConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub check_unicode: bool,
    #[serde(default = "default_true")]
    pub check_zero_width: bool,
}

impl Default for PromptInjectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_unicode: true,
            check_zero_width: true,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileAccessConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub workspace_dir: Option<String>,
    #[serde(default = "default_sensitive_files")]
    pub sensitive_files: Vec<String>,
    #[serde(default)]
    pub allow_symlinks: bool,
}

fn default_sensitive_files() -> Vec<String> {
    vec![
        ".env".to_string(),
        ".git/".to_string(),
        "id_rsa".to_string(),
        "id_dsa".to_string(),
        ".ssh/".to_string(),
        ".aws/".to_string(),
        ".bash_history".to_string(),
        ".zsh_history".to_string(),
    ]
}

impl Default for FileAccessConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            workspace_dir: None,
            sensitive_files: default_sensitive_files(),
            allow_symlinks: false,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct NetworkConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub allow_external: bool,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default = "default_true")]
    pub block_internal_ips: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_external: false,
            allowed_domains: Vec::new(),
            block_internal_ips: true,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,
    #[serde(default = "default_max_burst")]
    pub max_burst: u32,
}

fn default_requests_per_minute() -> u32 { 60 }
fn default_max_burst() -> u32 { 10 }

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute: 60,
            max_burst: 10,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OutputSanitizerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub strip_control_chars: bool,
    #[serde(default = "default_true")]
    pub redact_pii: bool,
}

impl Default for OutputSanitizerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strip_control_chars: true,
            redact_pii: true,
        }
    }
}

fn default_true() -> bool { true }

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_input_length: 50000,
            pii_detection: PIIDetectionConfig::default(),
            prompt_injection: PromptInjectionConfig::default(),
            file_access: FileAccessConfig::default(),
            network: NetworkConfig::default(),
            rate_limit: RateLimitConfig::default(),
            output_sanitizer: OutputSanitizerConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size: usize,
    pub ttl_secs: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size: 1000,
            ttl_secs: 3600,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_dir: String,
    pub max_file_size_mb: usize,
    pub hmac_secret: Option<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_dir: "~/.agenthub/audit".to_string(),
            max_file_size_mb: 100,
            hmac_secret: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_secs: u64,
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_secs: 30,
            multiplier: 2.0,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TimeoutConfig {
    pub llm_request_secs: u64,
    pub skill_execution_secs: u64,
    pub http_request_secs: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            llm_request_secs: 120,
            skill_execution_secs: 300,
            http_request_secs: 30,
        }
    }
}

pub fn load_settings(config_path: Option<&str>) -> Result<Settings, config::ConfigError> {
    let mut builder = CfgConfig::builder();

    if let Some(path) = config_path {
        builder = builder.add_source(File::with_name(path).required(true));
    } else {
        let local_config = PathBuf::from("config").join("agenthub.yaml");
        if local_config.exists() {
            builder = builder.add_source(File::with_name(&local_config.to_string_lossy()));
        } else {
            let default_config_path = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("agenthub")
                .join("agenthub.yaml");

            if default_config_path.exists() {
                builder = builder.add_source(File::with_name(&default_config_path.to_string_lossy()));
            } else {
                builder = builder.add_source(config::File::from_str(
                    &serde_yaml::to_string(&Settings {
                        server: ServerConfig::default(),
                        llm: LLMConfig::default(),
                        security: SecurityConfig::default(),
                        cache: CacheConfig::default(),
                        audit: AuditConfig::default(),
                        retry: RetryConfig::default(),
                        timeouts: TimeoutConfig::default(),
                    })
                    .unwrap(),
                    config::FileFormat::Yaml,
                ));
            }
        }
    }

    builder = builder
        .add_source(Environment::with_prefix("AGENTHUB").separator("_"));

    builder.build()?.try_deserialize()
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with('~') {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(&path[2..])
    } else {
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~/.agenthub/config.yaml");
        assert!(expanded.is_absolute());
        assert!(!expanded.to_string_lossy().contains('~'));
    }

    #[test]
    fn test_default_settings() {
        let settings = Settings {
            server: ServerConfig::default(),
            llm: LLMConfig::default(),
            security: SecurityConfig::default(),
            cache: CacheConfig::default(),
            audit: AuditConfig::default(),
            retry: RetryConfig::default(),
            timeouts: TimeoutConfig::default(),
        };
        assert_eq!(settings.server.port, 8080);
        assert_eq!(settings.llm.default_provider, "ollama");
        assert_eq!(settings.llm.get_default_provider().unwrap().model, "qwen:4b");
        assert!(settings.security.pii_detection.enabled);
    }
}
