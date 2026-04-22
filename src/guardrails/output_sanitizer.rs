use crate::error::{Result, SecurityError};
use crate::guardrails::pii_detector::PIIRedactionGuard;

pub struct OutputSanitizer {
    enabled: bool,
    strip_control_chars: bool,
    redact_pii: bool,
    pii_guard: Option<PIIRedactionGuard>,
}

impl OutputSanitizer {
    pub fn new(enabled: bool, strip_control_chars: bool, redact_pii: bool) -> Self {
        let pii_guard = if redact_pii {
            Some(PIIRedactionGuard::new(true, true, true, true, true, true))
        } else {
            None
        };

        Self {
            enabled,
            strip_control_chars,
            redact_pii,
            pii_guard,
        }
    }

    pub fn sanitize(&self, output: &str) -> Result<String> {
        if !self.enabled {
            return Ok(output.to_string());
        }

        let mut result = output.to_string();

        if self.strip_control_chars {
            result = result
                .chars()
                .filter(|c| {
                    !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t'
                })
                .collect();
        }

        if self.redact_pii {
            if let Some(ref guard) = self.pii_guard {
                result = guard.inspect_output(&result)?;
            }
        }

        Ok(result)
    }
}
