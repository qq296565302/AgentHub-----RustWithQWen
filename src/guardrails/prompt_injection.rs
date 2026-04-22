use crate::error::{Result, SecurityError};
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref INJECTION_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)ignore\s+previous\s+instructions").unwrap(),
        Regex::new(r"(?i)you\s+are\s+now\s+acting\s+as").unwrap(),
        Regex::new(r"(?i)system\s*:\s*").unwrap(),
        Regex::new(r"(?i)<\s*/?s\s*>").unwrap(),
        Regex::new(r"(?i)\[INST\]").unwrap(),
        Regex::new(r"(?i)DAN\s+mode").unwrap(),
        Regex::new(r"(?i)jailbreak").unwrap(),
    ];
}

pub struct PromptInjectionGuard {
    enabled: bool,
    check_unicode: bool,
    check_zero_width: bool,
}

impl PromptInjectionGuard {
    pub fn new(enabled: bool, check_unicode: bool, check_zero_width: bool) -> Self {
        Self {
            enabled,
            check_unicode,
            check_zero_width,
        }
    }

    pub fn check(&self, input: &str) -> Result<Vec<String>> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        let mut warnings = Vec::new();

        for pattern in INJECTION_PATTERNS.iter() {
            if pattern.is_match(input) {
                tracing::warn!("Potential prompt injection detected: pattern matched '{}'", pattern.as_str());
                return Err(SecurityError::PromptInjectionDetected.into());
            }
        }

        if self.check_unicode {
            let suspicious_chars = vec![
                '\u{0430}', '\u{0435}', '\u{043E}', '\u{0440}', '\u{0441}', '\u{0443}', '\u{0445}',
            ];
            if input.chars().any(|c| suspicious_chars.contains(&c)) {
                warnings.push("Unicode homoglyph characters detected".to_string());
            }
        }

        if self.check_zero_width {
            let zero_width_chars = vec![
                '\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}', '\u{2060}',
            ];
            if input.chars().any(|c| zero_width_chars.contains(&c)) {
                warnings.push("Zero-width characters detected".to_string());
            }
        }

        Ok(warnings)
    }
}
