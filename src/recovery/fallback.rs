use crate::error::Result;

pub struct FallbackResponse {
    pub message: String,
    pub source: String,
}

impl FallbackResponse {
    pub fn default_response() -> Self {
        Self {
            message: "LLM service is currently unavailable. Please try again later.".to_string(),
            source: "fallback".to_string(),
        }
    }

    pub fn cached_response(data: &str) -> Self {
        Self {
            message: format!("Cached response: {}", data),
            source: "cache".to_string(),
        }
    }
}

pub fn generate_fallback_response(skill_name: &str) -> Result<serde_json::Value> {
    let fallback = FallbackResponse::default_response();
    Ok(serde_json::json!({
        "skill": skill_name,
        "status": "fallback",
        "message": fallback.message,
        "source": fallback.source,
    }))
}
