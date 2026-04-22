use crate::error::{Result, SecurityError};
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref EMAIL_PATTERN: Regex = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
    static ref PHONE_PATTERN: Regex = Regex::new(r"\b(?:\+?86)?1[3-9]\d{9}\b").unwrap();
    static ref ID_CARD_PATTERN: Regex = Regex::new(r"\b\d{17}[\dXx]\b").unwrap();
    static ref IP_PATTERN: Regex = Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap();
    static ref API_KEY_PATTERN: Regex = Regex::new(r#"(?i)(?:api[_-]?key|token|secret)\s*[:=]\s*['"]?([a-zA-Z0-9]{20,})['"]?"#).unwrap();
}

pub struct PIIRedactionGuard {
    enabled: bool,
    redact_emails: bool,
    redact_phones: bool,
    redact_id_cards: bool,
    redact_ips: bool,
    redact_api_keys: bool,
}

impl PIIRedactionGuard {
    pub fn new(
        enabled: bool,
        redact_emails: bool,
        redact_phones: bool,
        redact_id_cards: bool,
        redact_ips: bool,
        redact_api_keys: bool,
    ) -> Self {
        Self {
            enabled,
            redact_emails,
            redact_phones,
            redact_id_cards,
            redact_ips,
            redact_api_keys,
        }
    }

    pub fn check_input(&self, input: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let mut pii_types = Vec::new();

        if self.redact_emails && EMAIL_PATTERN.is_match(input) {
            pii_types.push("email".to_string());
        }
        if self.redact_phones && PHONE_PATTERN.is_match(input) {
            pii_types.push("phone".to_string());
        }
        if self.redact_id_cards && ID_CARD_PATTERN.is_match(input) {
            pii_types.push("id_card".to_string());
        }
        if self.redact_ips && IP_PATTERN.is_match(input) {
            pii_types.push("ip".to_string());
        }
        if self.redact_api_keys && API_KEY_PATTERN.is_match(input) {
            pii_types.push("api_key".to_string());
        }

        if !pii_types.is_empty() {
            tracing::warn!("PII detected in input: {:?}", pii_types);
            return Err(SecurityError::PiiDetected {
                pii_type: pii_types.join(", "),
            }
            .into());
        }

        Ok(())
    }

    pub fn inspect_output(&self, output: &str) -> Result<String> {
        if !self.enabled {
            return Ok(output.to_string());
        }

        let mut result = output.to_string();

        if self.redact_emails {
            result = EMAIL_PATTERN.replace_all(&result, "[EMAIL_REDACTED]").to_string();
        }
        if self.redact_phones {
            result = PHONE_PATTERN.replace_all(&result, "[PHONE_REDACTED]").to_string();
        }
        if self.redact_id_cards {
            result = ID_CARD_PATTERN.replace_all(&result, "[ID_REDACTED]").to_string();
        }
        if self.redact_ips {
            result = IP_PATTERN.replace_all(&result, "[IP_REDACTED]").to_string();
        }
        if self.redact_api_keys {
            result = API_KEY_PATTERN.replace_all(&result, "[API_KEY_REDACTED]").to_string();
        }

        Ok(result)
    }
}
