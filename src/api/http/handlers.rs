use crate::config::Settings;
use crate::error::{AgentHubError, SecurityLevel};
use crate::guardrails::SecurityPipeline;
use crate::llm::LLMClient;
use crate::skill::SkillRegistry;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct AppState {
    #[allow(dead_code)]
    pub settings: Settings,
    #[allow(dead_code)]
    pub llm_client: Arc<dyn LLMClient>,
    pub skill_registry: SkillRegistry,
    pub security_pipeline: Arc<SecurityPipeline>,
}

#[derive(Deserialize)]
pub struct ExplainRequest {
    pub file_path: String,
    pub function_name: Option<String>,
    #[allow(dead_code)]
    pub line_range: Option<(usize, usize)>,
    #[allow(dead_code)]
    pub language: Option<String>,
}

#[derive(Serialize)]
pub struct ExplainResponse {
    pub explanation: String,
    pub file_path: String,
    pub function_name: Option<String>,
}

#[derive(Deserialize)]
pub struct TestRequest {
    pub file_path: String,
    pub function_name: String,
    #[allow(dead_code)]
    pub language: Option<String>,
}

#[derive(Serialize)]
pub struct TestResponse {
    pub test_code: String,
    pub file_path: String,
    pub function_name: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

impl From<AgentHubError> for (StatusCode, Json<ErrorResponse>) {
    fn from(err: AgentHubError) -> Self {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "InternalError".to_string(),
                message: err.to_string(),
            }),
        )
    }
}

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub async fn explain_code(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExplainRequest>,
) -> std::result::Result<Json<ExplainResponse>, (StatusCode, Json<ErrorResponse>)> {
    if let Err(e) = state.security_pipeline.check_rate_limit() {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: "RateLimited".to_string(),
                message: e.to_string(),
            }),
        ));
    }

    let input = format!("{} {}", request.file_path, request.function_name.as_deref().unwrap_or(""));
    let check_result = state.security_pipeline.check_input(&input);
    if check_result.is_blocked() {
        if let SecurityLevel::Block(msg) = check_result.level {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "SecurityBlocked".to_string(),
                    message: msg,
                }),
            ));
        }
    }

    let explanation = format!(
        "Mock explanation for {} in {}",
        request.function_name.as_deref().unwrap_or("entire file"),
        request.file_path
    );

    let sanitized = state.security_pipeline.sanitize_output(&explanation)
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "SanitizationError".to_string(),
                message: e.to_string(),
            }),
        ))?;

    Ok(Json(ExplainResponse {
        explanation: sanitized,
        file_path: request.file_path,
        function_name: request.function_name,
    }))
}

pub async fn generate_test(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TestRequest>,
) -> std::result::Result<Json<TestResponse>, (StatusCode, Json<ErrorResponse>)> {
    if let Err(e) = state.security_pipeline.check_rate_limit() {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: "RateLimited".to_string(),
                message: e.to_string(),
            }),
        ));
    }

    let input = format!("{} {}", request.file_path, request.function_name);
    let check_result = state.security_pipeline.check_input(&input);
    if check_result.is_blocked() {
        if let SecurityLevel::Block(msg) = check_result.level {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "SecurityBlocked".to_string(),
                    message: msg,
                }),
            ));
        }
    }

    let test_code = format!(
        "// Mock test for {} in {}\n#[test]\nfn test_{}() {{\n    // TODO: Implement test\n}}",
        request.function_name,
        request.file_path,
        request.function_name
    );

    let sanitized = state.security_pipeline.sanitize_output(&test_code)
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "SanitizationError".to_string(),
                message: e.to_string(),
            }),
        ))?;

    Ok(Json(TestResponse {
        test_code: sanitized,
        file_path: request.file_path,
        function_name: request.function_name,
    }))
}

pub async fn list_skills(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let skills = state.skill_registry.list_skills();
    let skill_list: Vec<serde_json::Value> = skills.iter().map(|s| {
        serde_json::json!({
            "name": s.name,
            "version": s.version,
            "description": s.description
        })
    }).collect();
    Json(serde_json::json!({
        "skills": skill_list
    }))
}
