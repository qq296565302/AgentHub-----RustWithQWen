use crate::api::cli::commands::{Command, ConversationAction, format_command_help};
use crate::audit::AuditLogger;
use crate::config::Settings;
use crate::error::Result;
use crate::guardrails::SecurityPipeline;
use crate::llm::{ChatMessage, MockLLMClient, LLMClient, MultiLLMClient};
use crate::prompt::{ConversationManager, ContextManager};
use crate::skill::builtins::code_explainer::CodeExplainerSkill;
use crate::skill::builtins::test_generator::TestGeneratorSkill;
use crate::skill::{ExecutionContext, SkillRegistry};
use crate::utils::markdown::render_markdown;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::sync::Arc;

pub struct Repl {
    settings: Settings,
    skill_registry: SkillRegistry,
    audit_logger: AuditLogger,
    llm_client: Arc<MultiLLMClient>,
    conversation_manager: ConversationManager,
    security_pipeline: SecurityPipeline,
}

impl Repl {
    pub fn new(settings: Settings) -> Self {
        let llm_client = Arc::new(MultiLLMClient::new(&settings.llm));

        let max_tokens = settings.llm.get_default_provider()
            .map(|p| p.max_tokens)
            .unwrap_or(4096);
        let conversation_manager = ConversationManager::new(max_tokens / 2);

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

        let audit_logger = AuditLogger::new(&settings);
        let security_pipeline = SecurityPipeline::new(settings.security.clone());

        Self {
            settings,
            skill_registry,
            audit_logger,
            llm_client,
            conversation_manager,
            security_pipeline,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut rl = DefaultEditor::new().map_err(|e| {
            crate::error::AgentHubError::Internal(format!("Failed to initialize REPL: {}", e))
        })?;

        println!("AgentHub REPL v{}", env!("CARGO_PKG_VERSION"));
        println!("安全、可编程、Skill 驱动的本地 AI 执行引擎");
        println!("直接输入文字与 AI 对话，输入 /help 查看可用命令");
        println!();

        loop {
            let readline = rl.readline("agenthub> ");
            match readline {
                Ok(line) => {
                    if line.trim().is_empty() {
                        continue;
                    }

                    let _ = rl.add_history_entry(line.as_str());

                    match Command::parse(&line) {
                        Ok(cmd) => {
                            if let Err(e) = self.execute_command(cmd).await {
                                println!("Error: {}", e);
                            }
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Interrupted. Type 'exit' to quit.");
                }
                Err(ReadlineError::Eof) => {
                    println!("Exiting...");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn execute_command(&mut self, cmd: Command) -> Result<()> {
        match cmd {
            Command::Explain { file_path, function_name, line_range } => {
                self.execute_explain(&file_path, function_name.as_deref(), line_range).await
            }
            Command::Test { file_path, function_name } => {
                self.execute_test(&file_path, &function_name).await
            }
            Command::Skills => {
                let skills = self.skill_registry.list_skills();
                let count = skills.len();
                println!("\n已注册的 Skills:");
                for skill in skills {
                    println!("  {} v{} - {}", skill.name, skill.version, skill.description);
                }
                println!("\n共 {} 个 Skill", count);
                Ok(())
            }
            Command::Audit { user_id, skill_name } => {
                let events = self.audit_logger.query_events(
                    user_id.as_deref(),
                    skill_name.as_deref(),
                ).await;
                println!("Audit events ({}):", events.len());
                for event in events {
                    println!("  [{}] {} - {} ({})",
                        event.timestamp,
                        event.user_id,
                        event.skill_name,
                        event.status
                    );
                }
                Ok(())
            }
            Command::Config => {
                let current_provider = self.llm_client.get_current_provider().await;
                let provider_config = self.settings.llm.get_provider(&current_provider);
                println!("\n当前配置:");
                if let Some(config) = provider_config {
                    let nickname = config.nickname.as_deref().unwrap_or(&config.name);
                    println!("  LLM 提供商: {} ({})", nickname, config.model);
                    println!("  端点: {}", config.api_endpoint);
                }
                println!("  服务地址: {}:{}", self.settings.server.host, self.settings.server.port);
                println!("  安全设置:");
                println!("    - PII 检测: {}", if self.settings.security.pii_detection.enabled { "开启" } else { "关闭" });
                println!("    - 注入检测: {}", if self.settings.security.prompt_injection.enabled { "开启" } else { "关闭" });
                println!("    - 文件访问控制: {}", if self.settings.security.file_access.enabled { "开启" } else { "关闭" });
                println!("    - 网络访问控制: {}", if self.settings.security.network.enabled { "开启" } else { "关闭" });
                println!("    - 速率限制: {}", if self.settings.security.rate_limit.enabled { "开启" } else { "关闭" });
                println!("    - 输出消毒: {}", if self.settings.security.output_sanitizer.enabled { "开启" } else { "关闭" });
                println!("    - 最大输入长度: {} 字符", self.settings.security.max_input_length);
                Ok(())
            }
            Command::Provider { name } => {
                match name {
                    Some(provider_name) => {
                        match self.llm_client.switch_provider(&provider_name).await {
                            Ok(()) => {
                                if let Some(config) = self.settings.llm.get_provider(&provider_name) {
                                    let nickname = config.nickname.as_deref().unwrap_or(&provider_name);
                                    println!("已切换到提供商: {} ({})", nickname, config.model);
                                } else {
                                    println!("已切换到提供商: {}", provider_name);
                                }
                            }
                            Err(e) => {
                                println!("切换失败: {}", e);
                            }
                        }
                    }
                    None => {
                        let current = self.llm_client.get_current_provider().await;
                        if let Some(config) = self.settings.llm.get_provider(&current) {
                            let nickname = config.nickname.as_deref().unwrap_or(&current);
                            println!("当前提供商: {} ({})", nickname, config.model);
                        } else {
                            println!("当前提供商: {}", current);
                        }
                    }
                }
                Ok(())
            }
            Command::Providers => {
                let providers = self.llm_client.list_providers().await;
                let current = self.llm_client.get_current_provider().await;
                println!("\n可用的 LLM 提供商:");
                for (i, provider) in providers.iter().enumerate() {
                    let marker = if provider == &current { " [当前]" } else { "" };
                    if let Some(config) = self.settings.llm.get_provider(provider) {
                        let nickname = config.nickname.as_deref().unwrap_or(provider);
                        println!("  {}. {}{} - {} ({})", 
                            i + 1, provider, marker, nickname, config.model);
                        if let Some(desc) = &config.description {
                            println!("     {}", desc);
                        }
                    } else {
                        println!("  {}. {}{}", i + 1, provider, marker);
                    }
                }
                println!("\n使用 /provider <名称> 切换提供商");
                Ok(())
            }
            Command::Conversation { action } => {
                self.handle_conversation(action).await
            }
            Command::Chat { message } => {
                self.handle_chat(&message).await
            }
            Command::Clear => {
                self.conversation_manager.clear_active();
                println!("对话历史已清除。");
                Ok(())
            }
            Command::Help => {
                println!("\n可用命令:");
                println!("{}", format_command_help());
                Ok(())
            }
            Command::Exit => {
                println!("再见！");
                std::process::exit(0);
            }
        }
    }

    async fn execute_explain(&self, file_path: &str, _function_name: Option<&str>, _line_range: Option<(usize, usize)>) -> Result<()> {
        if let Some(skill) = self.skill_registry.get("code.explainer") {
            let params = serde_json::json!({
                "file_path": file_path,
                "language": "auto"
            });

            let context = ExecutionContext {
                user_id: "cli-user".to_string(),
                workspace_dir: std::env::current_dir().unwrap_or_default(),
            };

            match skill.execute(params, &context).await {
                Ok(result) => {
                    println!();
                    let output = match result.output {
                        serde_json::Value::String(s) => s,
                        _ => result.output.to_string(),
                    };
                    render_markdown(&output);
                    println!();
                }
                Err(e) => {
                    println!("Skill execution failed: {}", e);
                }
            }
        }
        Ok(())
    }

    async fn execute_test(&self, file_path: &str, function_name: &str) -> Result<()> {
        if let Some(skill) = self.skill_registry.get("code.test.generator") {
            let params = serde_json::json!({
                "file_path": file_path,
                "function_name": function_name,
                "language": "auto"
            });

            let context = ExecutionContext {
                user_id: "cli-user".to_string(),
                workspace_dir: std::env::current_dir().unwrap_or_default(),
            };

            match skill.execute(params, &context).await {
                Ok(result) => {
                    println!();
                    let output = match result.output {
                        serde_json::Value::String(s) => s,
                        _ => result.output.to_string(),
                    };
                    render_markdown(&output);
                    println!();
                }
                Err(e) => {
                    println!("Skill execution failed: {}", e);
                }
            }
        }
        Ok(())
    }

    async fn handle_conversation(&mut self, action: ConversationAction) -> Result<()> {
        match action {
            ConversationAction::New { id, system_prompt } => {
                let conv_id = self.conversation_manager.create_conversation(id.clone(), system_prompt.clone());
                println!("已创建新对话: {}", conv_id);
                if let Some(ref prompt) = system_prompt {
                    println!("  系统提示: {}", prompt);
                }
            }
            ConversationAction::List => {
                let conversations = self.conversation_manager.list_conversations();
                if conversations.is_empty() {
                    println!("暂无对话记录。");
                } else {
                    let active_id = self.conversation_manager.get_active().map(|c| &c.id);
                    println!("\n对话列表:");
                    for conv in conversations {
                        let marker = if Some(&conv.id) == active_id { " [当前]" } else { "" };
                        println!("  {}{} - {} 条消息, {} 个 Token{}",
                            conv.id,
                            marker,
                            conv.message_count(),
                            conv.total_token_count(),
                            if conv.system_prompt.is_some() { " [有系统提示]" } else { "" }
                        );
                    }
                    println!("\n使用 /conv switch <id> 切换对话");
                }
            }
            ConversationAction::Switch { id } => {
                match self.conversation_manager.switch_conversation(&id) {
                    Ok(()) => println!("已切换到对话: {}", id),
                    Err(e) => println!("错误: {}", e),
                }
            }
            ConversationAction::Delete { id } => {
                match self.conversation_manager.delete_conversation(&id) {
                    Ok(()) => println!("已删除对话: {}", id),
                    Err(e) => println!("错误: {}", e),
                }
            }
            ConversationAction::Clear => {
                self.conversation_manager.clear_active();
                println!("当前对话历史已清除。");
            }
            ConversationAction::Show => {
                if let Some(conv) = self.conversation_manager.get_active() {
                    println!("\n当前对话:");
                    println!("  ID: {}", conv.id);
                    println!("  消息数: {}", conv.message_count());
                    println!("  Token 数: {}", conv.total_token_count());
                    if let Some(ref prompt) = conv.system_prompt {
                        println!("  系统提示: {}", prompt);
                    }
                } else {
                    println!("当前无活跃对话。使用 /conv new 创建新对话。");
                }
            }
        }
        Ok(())
    }

    async fn handle_chat(&mut self, message: &str) -> Result<()> {
        if let Err(e) = self.security_pipeline.check_rate_limit() {
            println!("速率限制: {}", e);
            return Ok(());
        }

        let check_result = self.security_pipeline.check_input(message);
        if check_result.is_blocked() {
            if let crate::error::SecurityLevel::Block(msg) = check_result.level {
                println!("安全拦截: {}", msg);
                return Ok(());
            }
        }
        if check_result.has_warnings() {
            println!("安全警告: {}", check_result.warnings.join("; "));
        }

        if self.conversation_manager.get_active().is_none() {
            self.conversation_manager.create_conversation(None, None);
        }

        let user_message = ChatMessage::user(message);
        self.conversation_manager.add_message_to_active(user_message);

        let messages = self.conversation_manager.get_messages_for_llm()
            .unwrap_or_else(|| vec![ChatMessage::user(message)]);

        match self.llm_client.chat(&messages).await {
            Ok(response) => {
                let sanitized = self.security_pipeline.sanitize_output(&response).unwrap_or(response);
                println!();
                render_markdown(&sanitized);
                println!();
                let assistant_message = ChatMessage::assistant(&sanitized);
                self.conversation_manager.add_message_to_active(assistant_message);
            }
            Err(e) => {
                println!("错误: {}", e);
            }
        }

        Ok(())
    }
}

pub async fn run_repl(settings: Settings) -> Result<()> {
    let mut repl = Repl::new(settings);
    repl.run().await
}
