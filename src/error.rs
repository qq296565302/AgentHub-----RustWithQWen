use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentHubError {
    #[error("Skill error: {0}")]
    Skill(#[from] SkillError),

    #[error("Skill not found: '{0}'")]
    SkillNotFound(String),

    #[error("Skill execution failed: {reason}")]
    SkillExecutionFailed { reason: String },

    #[error("Security violation: {0}")]
    Security(#[from] SecurityError),

    #[error("Input blocked by security policy: {violation_type}")]
    InputBlocked { violation_type: String },

    #[error("LLM error: {0}")]
    LLM(#[from] LlmError),

    #[error("LLM service unavailable")]
    LlmUnavailable,

    #[error("LLM request timeout after {timeout_secs}s")]
    LlmTimeout { timeout_secs: u64 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Failed to read file: {path}")]
    FileReadError { path: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Missing required configuration: {field}")]
    MissingConfig { field: String },

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Parse error: {message}")]
    ParseError { message: String },

    #[error("Tree-sitter language error: {0}")]
    LanguageError(#[from] tree_sitter::LanguageError),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Unknown command: {0}")]
    UnknownCommand(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Error, Debug)]
pub enum SkillError {
    #[error("Invalid skill manifest: {0}")]
    InvalidManifest(String),

    #[error("Skill initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Invalid input parameters: {0}")]
    InvalidParameters(String),
}

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Prompt injection detected")]
    PromptInjectionDetected,

    #[error("PII data detected: {pii_type}")]
    PiiDetected { pii_type: String },

    #[error("Input size exceeds limit: {size} > {max_size}")]
    InputTooLarge { size: usize, max_size: usize },

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Blocked file access attempt: {path}")]
    FileAccessBlocked { path: String },

    #[error("Security check failed: {details}")]
    CheckFailed { details: String },

    #[error("Network access denied: {reason}")]
    NetworkDenied { reason: String },

    #[error("Output contains sensitive data: {pii_type}")]
    OutputPiiDetected { pii_type: String },
}

#[derive(Debug, Clone)]
pub enum SecurityLevel {
    Pass,
    Warn(String),
    Block(String),
}

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("API error (status {status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Response parsing failed: {0}")]
    ParseError(String),

    #[error("No response from model")]
    NoResponse,

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Context length exceeded")]
    ContextLengthExceeded,

    #[error("Circuit breaker open")]
    CircuitOpen,

    #[error("No more mock responses available")]
    NoMoreResponses,

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Rate limited by LLM provider")]
    RateLimited,

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, AgentHubError>;
