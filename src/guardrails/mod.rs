pub mod prompt_injection;
pub mod pii_detector;
pub mod unicode_security;
pub mod semantic_analyzer;
pub mod file_access;
pub mod network_guard;
pub mod output_sanitizer;
pub mod rate_limiter;
pub mod input_validator;

use crate::config::SecurityConfig;
use crate::error::{Result, SecurityLevel};

pub struct SecurityCheckResult {
    pub level: SecurityLevel,
    #[allow(dead_code)]
    pub sanitized_input: Option<String>,
    pub warnings: Vec<String>,
}

impl SecurityCheckResult {
    pub fn pass() -> Self {
        Self {
            level: SecurityLevel::Pass,
            sanitized_input: None,
            warnings: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn warn(message: String) -> Self {
        Self {
            level: SecurityLevel::Warn(message.clone()),
            sanitized_input: None,
            warnings: vec![message],
        }
    }

    pub fn block(message: String) -> Self {
        Self {
            level: SecurityLevel::Block(message.clone()),
            sanitized_input: None,
            warnings: vec![message],
        }
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self.level, SecurityLevel::Block(_))
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

pub struct SecurityPipeline {
    config: SecurityConfig,
    input_validator: input_validator::InputValidator,
    prompt_injection_guard: prompt_injection::PromptInjectionGuard,
    pii_guard: pii_detector::PIIRedactionGuard,
    #[allow(dead_code)]
    unicode_guard: unicode_security::UnicodeSecurityGuard,
    #[allow(dead_code)]
    file_access_guard: file_access::FileAccessGuard,
    #[allow(dead_code)]
    network_guard: network_guard::NetworkGuard,
    output_sanitizer: output_sanitizer::OutputSanitizer,
    rate_limiter: rate_limiter::RateLimiter,
}

impl SecurityPipeline {
    pub fn new(config: SecurityConfig) -> Self {
        let workspace_dir = config.file_access.workspace_dir.clone()
            .unwrap_or_else(|| ".".to_string());

        Self {
            config: config.clone(),
            input_validator: input_validator::InputValidator::new(config.max_input_length),
            prompt_injection_guard: prompt_injection::PromptInjectionGuard::new(
                config.prompt_injection.enabled,
                config.prompt_injection.check_unicode,
                config.prompt_injection.check_zero_width,
            ),
            pii_guard: pii_detector::PIIRedactionGuard::new(
                config.pii_detection.enabled,
                config.pii_detection.redact_emails,
                config.pii_detection.redact_phones,
                config.pii_detection.redact_id_cards,
                config.pii_detection.redact_ips,
                config.pii_detection.redact_api_keys,
            ),
            unicode_guard: unicode_security::UnicodeSecurityGuard::new(),
            file_access_guard: file_access::FileAccessGuard::new(
                config.file_access.enabled,
                &workspace_dir,
                &config.file_access.sensitive_files,
                config.file_access.allow_symlinks,
            ),
            network_guard: network_guard::NetworkGuard::new(
                config.network.enabled,
                config.network.allow_external,
                &config.network.allowed_domains,
                config.network.block_internal_ips,
            ),
            output_sanitizer: output_sanitizer::OutputSanitizer::new(
                config.output_sanitizer.enabled,
                config.output_sanitizer.strip_control_chars,
                config.output_sanitizer.redact_pii,
            ),
            rate_limiter: rate_limiter::RateLimiter::new(
                config.rate_limit.enabled,
                config.rate_limit.requests_per_minute,
                config.rate_limit.max_burst,
            ),
        }
    }

    pub fn check_input(&self, input: &str) -> SecurityCheckResult {
        if input.len() > self.config.max_input_length {
            return SecurityCheckResult::block(format!(
                "输入长度 {} 超过限制 {}",
                input.len(),
                self.config.max_input_length
            ));
        }

        if let Err(e) = self.input_validator.validate(input) {
            return SecurityCheckResult::block(e.to_string());
        }

        let mut warnings = Vec::new();

        if self.config.prompt_injection.enabled {
            match self.prompt_injection_guard.check(input) {
                Err(e) => return SecurityCheckResult::block(e.to_string()),
                Ok(warns) => warnings.extend(warns),
            }
        }

        if self.config.pii_detection.enabled {
            match self.pii_guard.check_input(input) {
                Err(e) => return SecurityCheckResult::block(e.to_string()),
                Ok(_) => {}
            }
        }

        if warnings.is_empty() {
            SecurityCheckResult::pass()
        } else {
            let msg = warnings.join("; ");
            SecurityCheckResult {
                level: SecurityLevel::Warn(msg.clone()),
                sanitized_input: None,
                warnings,
            }
        }
    }

    pub fn sanitize_output(&self, output: &str) -> Result<String> {
        self.output_sanitizer.sanitize(output)
    }

    #[allow(dead_code)]
    pub fn check_file_access(&self, path: &str) -> Result<()> {
        self.file_access_guard.check_access(path)
    }

    #[allow(dead_code)]
    pub fn check_network_access(&self, domain: &str) -> Result<()> {
        self.network_guard.check_access(domain)
    }

    pub fn check_rate_limit(&self) -> Result<()> {
        self.rate_limiter.check()
    }
}
