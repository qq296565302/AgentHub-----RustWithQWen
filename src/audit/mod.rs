pub mod signed_log;

use crate::config::Settings;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub skill_name: String,
    pub input_hash: String,
    pub status: String,
    pub error_message: Option<String>,
}

pub struct AuditLogger {
    #[allow(dead_code)]
    log_dir: PathBuf,
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

#[allow(dead_code)]
impl AuditLogger {
    pub fn new(settings: &Settings) -> Self {
        let log_dir = crate::config::expand_tilde(&settings.audit.log_dir);
        std::fs::create_dir_all(&log_dir).expect("Failed to create audit log directory");

        Self {
            log_dir,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn log_event(&self, event: AuditEvent) {
        let mut events = self.events.lock().await;
        events.push(event);
    }

    pub async fn create_event(
        &self,
        user_id: &str,
        skill_name: &str,
        input: &str,
        status: &str,
        error_message: Option<String>,
    ) -> AuditEvent {
        let id = uuid::Uuid::new_v4().to_string();
        let input_hash = Self::hash_input(input);

        AuditEvent {
            id,
            timestamp: Utc::now(),
            user_id: user_id.to_string(),
            skill_name: skill_name.to_string(),
            input_hash,
            status: status.to_string(),
            error_message,
        }
    }

    pub async fn query_events(
        &self,
        user_id: Option<&str>,
        skill_name: Option<&str>,
    ) -> Vec<AuditEvent> {
        let events = self.events.lock().await;
        events
            .iter()
            .filter(|e| {
                let user_match = user_id.map_or(true, |uid| e.user_id == uid);
                let skill_match = skill_name.map_or(true, |sn| e.skill_name == sn);
                user_match && skill_match
            })
            .cloned()
            .collect()
    }

    fn hash_input(input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
