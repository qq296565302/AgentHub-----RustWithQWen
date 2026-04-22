use crate::error::{Result, SecurityError};

pub struct InputValidator {
    max_length: usize,
}

impl InputValidator {
    pub fn new(max_length: usize) -> Self {
        Self { max_length }
    }

    pub fn validate(&self, input: &str) -> Result<()> {
        if input.len() > self.max_length {
            return Err(SecurityError::InputTooLarge {
                size: input.len(),
                max_size: self.max_length,
            }
            .into());
        }

        if input.is_empty() {
            return Err(SecurityError::CheckFailed {
                details: "Empty input".to_string(),
            }
            .into());
        }

        Ok(())
    }
}
