use crate::error::{Result, SecurityError};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    enabled: bool,
    requests_per_minute: u32,
    max_burst: u32,
    requests: Mutex<VecDeque<Instant>>,
}

impl RateLimiter {
    pub fn new(enabled: bool, requests_per_minute: u32, max_burst: u32) -> Self {
        Self {
            enabled,
            requests_per_minute,
            max_burst,
            requests: Mutex::new(VecDeque::new()),
        }
    }

    pub fn check(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();
        let window_start = now - Duration::from_secs(60);

        while let Some(&timestamp) = requests.front() {
            if timestamp < window_start {
                requests.pop_front();
            } else {
                break;
            }
        }

        if requests.len() >= self.requests_per_minute as usize {
            return Err(SecurityError::RateLimited.into());
        }

        if requests.len() >= self.max_burst as usize {
            let oldest = requests.front().unwrap();
            let elapsed = now.duration_since(*oldest);
            if elapsed < Duration::from_secs(1) {
                return Err(SecurityError::RateLimited.into());
            }
        }

        requests.push_back(now);
        Ok(())
    }
}
