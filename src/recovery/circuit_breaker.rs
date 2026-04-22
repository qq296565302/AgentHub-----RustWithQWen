use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    last_failure_time: Arc<RwLock<Option<DateTime<Utc>>>>,
    failure_threshold: u32,
    recovery_timeout_secs: u64,
    half_open_max_calls: u32,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout_secs: u64) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            failure_threshold,
            recovery_timeout_secs,
            half_open_max_calls: 1,
        }
    }

    pub async fn is_available(&self) -> bool {
        let state = self.state.read().await;
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_failure = self.last_failure_time.read().await;
                if let Some(last_time) = *last_failure {
                    let elapsed = Utc::now().signed_duration_since(last_time);
                    elapsed.num_seconds() >= self.recovery_timeout_secs as i64
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                self.success_count.load(Ordering::SeqCst) < self.half_open_max_calls
            }
        }
    }

    pub async fn record_success(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.fetch_add(1, Ordering::SeqCst);
        let mut state = self.state.write().await;
        if *state == CircuitState::HalfOpen {
            *state = CircuitState::Closed;
        }
    }

    pub async fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::SeqCst);
        let mut last_failure_time = self.last_failure_time.write().await;
        *last_failure_time = Some(Utc::now());

        if self.failure_count.load(Ordering::SeqCst) >= self.failure_threshold {
            let mut state = self.state.write().await;
            *state = CircuitState::Open;
        }
    }

    pub async fn get_state(&self) -> CircuitState {
        self.state.read().await.clone()
    }
}
