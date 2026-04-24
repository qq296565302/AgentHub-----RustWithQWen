#![allow(dead_code)]

use metrics::{counter, gauge, histogram};

pub fn record_llm_request(success: bool, duration: f64) {
    counter!("llm_requests_total").increment(1);
    if success {
        histogram!("llm_request_duration_seconds").record(duration);
    }
}

pub fn record_skill_execution(_skill_name: &str, duration: f64) {
    counter!("skill_executions_total").increment(1);
    histogram!("skill_execution_duration_seconds").record(duration);
}

pub fn record_security_violation(_violation_type: &str) {
    counter!("security_violations_total").increment(1);
}

pub fn update_active_connections(count: u64) {
    gauge!("active_connections").set(count as f64);
}

pub fn update_memory_usage(bytes: u64) {
    gauge!("memory_usage_bytes").set(bytes as f64);
}
