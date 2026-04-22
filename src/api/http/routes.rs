use crate::api::http::handlers::{health_check, explain_code, generate_test, list_skills, AppState};
use crate::config::Settings;
use crate::error::Result;
use crate::guardrails::SecurityPipeline;
use crate::llm::mock::MockLLMClient;
use crate::llm::LLMClient;
use crate::skill::builtins::code_explainer::CodeExplainerSkill;
use crate::skill::builtins::test_generator::TestGeneratorSkill;
use crate::skill::SkillRegistry;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;

pub fn create_router(settings: Settings) -> Router {
    let llm_client: Arc<dyn LLMClient> = Arc::new(
        MockLLMClient::new(vec![
            "Mock LLM response".to_string(),
        ])
    );

    let mut skill_registry = SkillRegistry::new();
    skill_registry.register(
        "code.explainer".to_string(),
        "0.1.0".to_string(),
        "解释代码逻辑及潜在风险".to_string(),
        Box::new(CodeExplainerSkill::new(llm_client.clone())),
    );
    skill_registry.register(
        "code.test.generator".to_string(),
        "0.1.0".to_string(),
        "为函数生成单元测试".to_string(),
        Box::new(TestGeneratorSkill::new(llm_client.clone())),
    );

    let security_pipeline = Arc::new(SecurityPipeline::new(settings.security.clone()));

    let state = Arc::new(AppState {
        settings,
        llm_client,
        skill_registry,
        security_pipeline,
    });

    Router::new()
        .route("/health", get(health_check))
        .route("/api/explain", post(explain_code))
        .route("/api/test", post(generate_test))
        .route("/api/skills", get(list_skills))
        .with_state(state)
}

pub async fn start_server(settings: Settings) -> Result<()> {
    let app = create_router(settings.clone());
    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    
    tracing::info!("Starting HTTP server on {}", addr);
    
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| crate::error::AgentHubError::Internal(format!("Failed to bind to {}: {}", addr, e)))?;
    
    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::AgentHubError::Internal(format!("Server error: {}", e)))?;
    
    Ok(())
}
