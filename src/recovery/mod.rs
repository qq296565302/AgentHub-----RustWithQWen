pub mod circuit_breaker;
pub mod resilient_llm;
pub mod fallback;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_secs: u64,
    pub multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_secs: 30,
            multiplier: 2.0,
        }
    }
}
