use crate::api::cli::commands::{Command, ConversationAction, format_command_help};
use crate::api::cli::nl_command_parser::NaturalLanguageCommandParser;
use crate::audit::AuditLogger;
use crate::config::Settings;
use crate::error::Result;
use crate::guardrails::SecurityPipeline;
use crate::llm::{ChatMessage, LLMClient, MultiLLMClient};
use crate::prompt::ConversationManager;
use crate::skill::external::external_loader::{ExternalSkillLoader, ExternalSkillConfig};
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
    nl_command_parser: Arc<NaturalLanguageCommandParser>,
    rl: Option<DefaultEditor>,
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

        let external_loader = ExternalSkillLoader::new(ExternalSkillConfig::default());
        match external_loader.load_all_skills(&mut skill_registry) {
            Ok(count) => {
                if count > 0 {
                    println!("已加载 {} 个外部 Skill", count);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to load external skills: {}", e);
            }
        }

        let audit_logger = AuditLogger::new(&settings);
        let security_pipeline = SecurityPipeline::new(settings.security.clone());
        let nl_command_parser = Arc::new(NaturalLanguageCommandParser::new(llm_client.clone(), &skill_registry));

        Self {
            settings,
            skill_registry,
            audit_logger,
            llm_client,
            conversation_manager,
            security_pipeline,
            nl_command_parser,
            rl: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        self.rl = Some(DefaultEditor::new().map_err(|e| {
            crate::error::AgentHubError::Internal(format!("Failed to initialize REPL: {}", e))
        })?);

        println!("AgentHub REPL v{}", env!("CARGO_PKG_VERSION"));
        println!("安全、可编程、Skill 驱动的本地 AI 执行引擎");
        println!("直接输入文字与 AI 对话，输入 /help 查看可用命令");
        println!();

        loop {
            let readline = self.rl.as_mut().unwrap().readline("agenthub> ");
            match readline {
                Ok(line) => {
                    if line.trim().is_empty() {
                        continue;
                    }

                    let _ = self.rl.as_mut().unwrap().add_history_entry(line.as_str());

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

    async fn execute_command(&mut self, cmd: Command) -> Result<String> {
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
                let mut output = String::from("\n已注册的 Skills:\n");
                for skill in &skills {
                    output.push_str(&format!("  {} v{} - {}\n", skill.name, skill.version, skill.description));
                }
                output.push_str(&format!("\n调用方法: /run <skill_name> [params_json]\n"));
                output.push_str("示例: /run test.skill {\"message\": \"hello\", \"operation\": \"upper\"}\n");
                output.push_str(&format!("\n共 {} 个 Skill", count));
                println!("{}", output);
                Ok(output)
            }
            Command::Run { skill_name, params } => {
                self.execute_run(&skill_name, &params).await?;
                Ok("".to_string())
            }
            Command::Audit { user_id, skill_name } => {
                let events = self.audit_logger.query_events(
                    user_id.as_deref(),
                    skill_name.as_deref(),
                ).await;
                let mut output = format!("Audit events ({}):", events.len());
                for event in &events {
                    output.push_str(&format!(
                        "\n  [{}] {} - {} ({})",
                        event.timestamp,
                        event.user_id,
                        event.skill_name,
                        event.status
                    ));
                }
                println!("{}", output);
                Ok(output)
            }
            Command::Config => {
                let output = self.format_config_output();
                println!("{}", output);
                Ok(output)
            }
            Command::Provider { name } => {
                let output = self.format_provider_output(name.as_deref()).await;
                println!("{}", output);
                Ok(output)
            }
            Command::Providers => {
                let output = self.format_providers_output().await;
                println!("{}", output);
                Ok(output)
            }
            Command::Conversation { action } => {
                self.handle_conversation(action).await.map(|_| "对话操作完成".to_string())
            }
            Command::Chat { message } => {
                self.handle_chat(&message).await
            }
            Command::Clear => {
                self.conversation_manager.clear_active();
                let output = "对话历史已清除。".to_string();
                println!("{}", output);
                Ok(output)
            }
            Command::Help => {
                let output = format!("\n可用命令:\n{}", format_command_help());
                println!("{}", output);
                Ok(output)
            }
            Command::Exit => {
                println!("再见！");
                std::process::exit(0);
            }
        }
    }

    fn execute_command_boxed<'a>(&'a mut self, cmd: Command) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(self.execute_command(cmd))
    }

    fn format_config_output(&self) -> String {
        let provider_config = self.settings.llm.get_default_provider();
        let mut output = String::from("\n当前配置:");
        if let Some(config) = provider_config {
            let nickname = config.nickname.as_deref().unwrap_or(&config.name);
            output.push_str(&format!("\n  LLM 提供商: {} ({})", nickname, config.model));
            output.push_str(&format!("\n  端点: {}", config.api_endpoint));
        }
        output.push_str(&format!("\n  服务地址: {}:{}", self.settings.server.host, self.settings.server.port));
        output.push_str("\n  安全设置:");
        output.push_str(&format!("\n    - PII 检测: {}", if self.settings.security.pii_detection.enabled { "开启" } else { "关闭" }));
        output.push_str(&format!("\n    - 注入检测: {}", if self.settings.security.prompt_injection.enabled { "开启" } else { "关闭" }));
        output.push_str(&format!("\n    - 文件访问控制: {}", if self.settings.security.file_access.enabled { "开启" } else { "关闭" }));
        output.push_str(&format!("\n    - 网络访问控制: {}", if self.settings.security.network.enabled { "开启" } else { "关闭" }));
        output.push_str(&format!("\n    - 速率限制: {}", if self.settings.security.rate_limit.enabled { "开启" } else { "关闭" }));
        output.push_str(&format!("\n    - 输出消毒: {}", if self.settings.security.output_sanitizer.enabled { "开启" } else { "关闭" }));
        output.push_str(&format!("\n    - 最大输入长度: {} 字符", self.settings.security.max_input_length));
        output
    }

    async fn format_provider_output(&self, name: Option<&str>) -> String {
        match name {
            Some(provider_name) => {
                match self.llm_client.switch_provider(provider_name).await {
                    Ok(()) => {
                        if let Some(config) = self.settings.llm.get_provider(provider_name) {
                            let nickname = config.nickname.as_deref().unwrap_or(provider_name);
                            format!("已切换到提供商: {} ({})", nickname, config.model)
                        } else {
                            format!("已切换到提供商: {}", provider_name)
                        }
                    }
                    Err(e) => {
                        format!("切换失败: {}", e)
                    }
                }
            }
            None => {
                let current = self.llm_client.get_current_provider().await;
                if let Some(config) = self.settings.llm.get_provider(&current) {
                    let nickname = config.nickname.as_deref().unwrap_or(&current);
                    format!("当前提供商: {} ({})", nickname, config.model)
                } else {
                    format!("当前提供商: {}", current)
                }
            }
        }
    }

    async fn format_providers_output(&self) -> String {
        let providers = self.llm_client.list_providers().await;
        let current = self.llm_client.get_current_provider().await;
        let mut output = String::from("\n可用的 LLM 提供商:");
        for (i, provider) in providers.iter().enumerate() {
            let marker = if provider == &current { " [当前]" } else { "" };
            if let Some(config) = self.settings.llm.get_provider(provider) {
                let nickname = config.nickname.as_deref().unwrap_or(provider);
                output.push_str(&format!(
                    "\n  {}. {}{} - {} ({})", 
                    i + 1, provider, marker, nickname, config.model
                ));
                if let Some(desc) = &config.description {
                    output.push_str(&format!("\n     {}", desc));
                }
            } else {
                output.push_str(&format!("\n  {}. {}{}", i + 1, provider, marker));
            }
        }
        output.push_str("\n\n使用 /provider <名称> 切换提供商");
        output
    }

    async fn execute_explain(&mut self, file_path: &str, _function_name: Option<&str>, _line_range: Option<(usize, usize)>) -> Result<String> {
        let resolved_path = match self.resolve_skill_file_path("code.explainer", file_path).await {
            Ok(p) => p,
            Err(crate::error::AgentHubError::AmbiguousFile { name, paths }) => {
                let selected = self.prompt_file_selection(&name, &paths).await?;
                selected
            }
            Err(e) => {
                let error_msg = format!("Skill 执行失败: {}", e);
                println!("{}", error_msg);
                return Err(crate::error::AgentHubError::Internal(error_msg));
            }
        };

        self.execute_skill_with_output("code.explainer", "explanation", &resolved_path, None).await
    }

    async fn execute_test(&mut self, file_path: &str, function_name: &str) -> Result<String> {
        let resolved_path = match self.resolve_skill_file_path("code.test.generator", file_path).await {
            Ok(p) => p,
            Err(crate::error::AgentHubError::AmbiguousFile { name, paths }) => {
                let selected = self.prompt_file_selection(&name, &paths).await?;
                selected
            }
            Err(e) => {
                let error_msg = format!("Skill 执行失败: {}", e);
                println!("{}", error_msg);
                return Err(crate::error::AgentHubError::Internal(error_msg));
            }
        };

        self.execute_skill_with_output("code.test.generator", "test_code", &resolved_path, Some(function_name)).await
    }

    async fn resolve_skill_file_path(&self, _skill_name: &str, file_path: &str) -> Result<String> {
        let path = std::path::Path::new(file_path);
        if path.exists() {
            return Ok(file_path.to_string());
        }

        let file_name = path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().to_string();
        let workspace_dir = std::env::current_dir().unwrap_or_default();
        let mut search_dirs = vec![workspace_dir.clone()];

        for entry in &["src", "lib", "app", "core", "common"] {
            let dir = workspace_dir.join(entry);
            if dir.is_dir() {
                search_dirs.push(dir);
            }
        }

        let mut matches = Vec::new();
        for dir in &search_dirs {
            self.collect_matches(dir, &file_name, &mut matches);
        }

        if matches.is_empty() {
            Err(crate::error::AgentHubError::FileNotFound { path: file_path.to_string() })
        } else if matches.len() > 1 {
            Err(crate::error::AgentHubError::AmbiguousFile {
                name: file_path.to_string(),
                paths: matches,
            })
        } else {
            Ok(matches.into_iter().next().unwrap())
        }
    }

    fn collect_matches(&self, dir: &std::path::Path, file_name: &str, matches: &mut Vec<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    if let Some(name) = entry_path.file_name() {
                        if name.to_string_lossy() == file_name {
                            matches.push(entry_path.to_string_lossy().to_string());
                        }
                    }
                } else if entry_path.is_dir() {
                    self.collect_matches(&entry_path, file_name, matches);
                }
            }
        }
    }

    async fn execute_skill_with_output(
        &self,
        skill_name: &str,
        output_key: &str,
        file_path: &str,
        function_name: Option<&str>,
    ) -> Result<String> {
        if let Some(skill) = self.skill_registry.get(skill_name) {
            let mut params = serde_json::json!({
                "file_path": file_path,
                "language": "auto"
            });
            if let Some(fn_name) = function_name {
                params["function_name"] = serde_json::json!(fn_name);
            }

            let context = ExecutionContext {
                user_id: "cli-user".to_string(),
                workspace_dir: std::env::current_dir().unwrap_or_default(),
            };

            match skill.execute(params, &context).await {
                Ok(result) => {
                    let output = match &result.output {
                        serde_json::Value::Object(map) => {
                            map.get(output_key)
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| result.output.to_string())
                        }
                        serde_json::Value::String(s) => s.clone(),
                        _ => result.output.to_string(),
                    };
                    println!();
                    render_markdown(&output);
                    println!();
                    Ok(output)
                }
                Err(e) => {
                    let error_msg = format!("Skill 执行失败: {}", e);
                    println!("{}", error_msg);
                    Err(crate::error::AgentHubError::Internal(error_msg))
                }
            }
        } else {
            Err(crate::error::AgentHubError::Internal(format!("Skill '{}' 不存在", skill_name)))
        }
    }

    async fn execute_run(&mut self, skill_name: &str, params_str: &str) -> Result<()> {
        let params: serde_json::Value = serde_json::from_str(params_str)
            .map_err(|e| crate::error::AgentHubError::Internal(format!("参数 JSON 解析失败: {}", e)))?;

        if let Some(skill) = self.skill_registry.get(skill_name) {
            let context = ExecutionContext {
                user_id: "cli-user".to_string(),
                workspace_dir: std::env::current_dir().unwrap_or_default(),
            };

            println!("\n正在执行 Skill: {}...", skill_name);
            match skill.execute(params, &context).await {
                Ok(result) => {
                    let output = match &result.output {
                        serde_json::Value::Object(map) => {
                            if let Some(s) = map.get("result").and_then(|v| v.as_str()) {
                                s.to_string()
                            } else if let Some(s) = map.get("output").and_then(|v| v.as_str()) {
                                s.to_string()
                            } else {
                                serde_json::to_string_pretty(&result.output)
                                    .unwrap_or_else(|_| result.output.to_string())
                            }
                        }
                        serde_json::Value::String(s) => s.clone(),
                        _ => serde_json::to_string_pretty(&result.output)
                            .unwrap_or_else(|_| result.output.to_string()),
                    };
                    println!("\n执行结果:");
                    println!("{}", output);
                    if !result.warnings.is_empty() {
                        println!("\n警告:");
                        for w in &result.warnings {
                            println!("  - {}", w);
                        }
                    }
                    println!();
                }
                Err(e) => {
                    let error_msg = format!("Skill 执行失败: {}", e);
                    println!("{}", error_msg);
                    return Err(crate::error::AgentHubError::Internal(error_msg));
                }
            }
        } else {
            let available: Vec<_> = self.skill_registry.list_skills()
                .iter()
                .map(|s| s.name.as_str())
                .collect();
            return Err(crate::error::AgentHubError::Internal(
                format!("Skill '{}' 不存在。可用的 Skills: {:?}", skill_name, available)
            ));
        }
        Ok(())
    }

    async fn prompt_file_selection(&mut self, name: &str, paths: &[String]) -> Result<String> {
        println!();
        println!("找到 {} 个匹配 '{}' 的文件:", paths.len(), name);
        for (i, path) in paths.iter().enumerate() {
            println!("  {}. {}", i + 1, path);
        }
        println!();
        
        loop {
            let prompt = format!("请选择文件编号 (1-{}): ", paths.len());
            if let Some(rl) = self.rl.as_mut() {
                match rl.readline(&prompt) {
                    Ok(input) => {
                        if let Ok(index) = input.trim().parse::<usize>() {
                            if index >= 1 && index <= paths.len() {
                                return Ok(paths[index - 1].clone());
                            }
                        }
                        println!("无效的选择，请重新输入。");
                    }
                    Err(ReadlineError::Interrupted) => {
                        return Err(crate::error::AgentHubError::Internal("选择已取消".to_string()));
                    }
                    Err(e) => {
                        return Err(crate::error::AgentHubError::Internal(format!("读取输入失败: {}", e)));
                    }
                }
            } else {
                return Err(crate::error::AgentHubError::Internal("REPL 编辑器未初始化".to_string()));
            }
        }
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

    async fn handle_chat(&mut self, message: &str) -> Result<String> {
        if let Err(e) = self.security_pipeline.check_rate_limit() {
            println!("速率限制: {}", e);
            return Ok("".to_string());
        }

        let check_result = self.security_pipeline.check_input(message);
        if check_result.is_blocked() {
            if let crate::error::SecurityLevel::Block(msg) = check_result.level {
                println!("安全拦截: {}", msg);
                return Ok("".to_string());
            }
        }
        if check_result.has_warnings() {
            println!("安全警告: {}", check_result.warnings.join("; "));
        }

        match self.nl_command_parser.parse(message).await {
            Ok(Some(parsed)) => {
                match self.nl_command_parser.parsed_to_command(&parsed) {
                    Ok(cmd) => {
                        println!("执行命令: {}", parsed.command_type);
                        let result = self.execute_command_boxed(cmd).await;
                        if self.conversation_manager.get_active().is_none() {
                            self.conversation_manager.create_conversation(None, None);
                        }
                        let user_message = ChatMessage::user(message);
                        self.conversation_manager.add_message_to_active(user_message);
                        match &result {
                            Ok(output) => {
                                let assistant_message = ChatMessage::assistant(output);
                                self.conversation_manager.add_message_to_active(assistant_message);
                            }
                            Err(e) => {
                                let error_message = ChatMessage::assistant(&format!("命令执行失败: {}", e));
                                self.conversation_manager.add_message_to_active(error_message);
                            }
                        }
                        return result.map(|_| "".to_string());
                    }
                    Err(e) => {
                        tracing::warn!("Failed to convert parsed command: {}", e);
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!("Natural language command parsing failed: {}", e);
            }
        }

        self.do_chat(message).await
    }

    async fn do_chat(&mut self, message: &str) -> Result<String> {
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
                Ok(sanitized)
            }
            Err(e) => {
                let error_msg = format!("错误: {}", e);
                println!("{}", error_msg);
                Ok(error_msg)
            }
        }
    }
}

pub async fn run_repl(settings: Settings) -> Result<()> {
    let mut repl = Repl::new(settings);
    repl.run().await
}
