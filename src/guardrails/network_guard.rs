use crate::error::{Result, SecurityError};
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref INTERNAL_IP_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"^10\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap(),
        Regex::new(r"^172\.(1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}$").unwrap(),
        Regex::new(r"^192\.168\.\d{1,3}\.\d{1,3}$").unwrap(),
        Regex::new(r"^127\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap(),
        Regex::new(r"^localhost$").unwrap(),
    ];
}

pub struct NetworkGuard {
    #[allow(dead_code)]
    enabled: bool,
    #[allow(dead_code)]
    allow_external: bool,
    #[allow(dead_code)]
    allowed_domains: Vec<String>,
    #[allow(dead_code)]
    block_internal_ips: bool,
}

impl NetworkGuard {
    pub fn new(enabled: bool, allow_external: bool, allowed_domains: &[String], block_internal_ips: bool) -> Self {
        Self {
            enabled,
            allow_external,
            allowed_domains: allowed_domains.to_vec(),
            block_internal_ips,
        }
    }

    #[allow(dead_code)]
    pub fn check_access(&self, domain: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if self.block_internal_ips && self.is_internal_ip(domain) {
            return Err(SecurityError::NetworkDenied {
                reason: format!("Internal IP/hostname blocked: {}", domain),
            }
            .into());
        }

        if !self.allow_external {
            return Err(SecurityError::NetworkDenied {
                reason: "External network access is disabled".to_string(),
            }
            .into());
        }

        if !self.allowed_domains.is_empty() {
            let is_allowed = self.allowed_domains.iter().any(|d| {
                domain == d || domain.ends_with(&format!(".{}", d))
            });
            if !is_allowed {
                return Err(SecurityError::NetworkDenied {
                    reason: format!("Domain not in whitelist: {}", domain),
                }
                .into());
            }
        }

        Ok(())
    }

    fn is_internal_ip(&self, host: &str) -> bool {
        for pattern in INTERNAL_IP_PATTERNS.iter() {
            if pattern.is_match(host) {
                return true;
            }
        }
        false
    }
}
