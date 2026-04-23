use crate::api::cli::commands::Command;
use crate::error::Result;
use crate::llm::LLMClient;
use crate::skill::SkillRegistry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedCommand {
    pub is_command: bool,
    pub command_type: String,
    pub params: serde_json::Value,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub command_type: String,
    pub description: String,
    pub usage: String,
    pub params_schema: serde_json::Value,
}

pub struct NaturalLanguageCommandParser {
    llm_client: Arc<dyn LLMClient>,
    tools: Vec<ToolDefinition>,
}

impl NaturalLanguageCommandParser {
    pub fn new(llm_client: Arc<dyn LLMClient>, skill_registry: &SkillRegistry) -> Self {
        let tools = Self::build_tools_list(skill_registry);
        Self { llm_client, tools }
    }

    fn build_tools_list(skill_registry: &SkillRegistry) -> Vec<ToolDefinition> {
        let mut tools = vec![
            ToolDefinition {
                name: "解释代码".to_string(),
                command_type: "explain".to_string(),
                description: "解释指定代码文件的功能、逻辑和潜在问题".to_string(),
                usage: "解释 mod.rs、帮我看看 config/mod.rs、分析一下 main.rs 的第42行".to_string(),
                params_schema: serde_json::json!({
                    "file_path": "文件路径",
                    "function_name": "可选：函数名",
                    "line": "可选：行号"
                }),
            },
            ToolDefinition {
                name: "生成测试".to_string(),
                command_type: "test".to_string(),
                description: "为指定函数生成单元测试".to_string(),
                usage: "为 main.rs 的 main 函数生成测试、给 utils.rs 的 parse 函数写个测试".to_string(),
                params_schema: serde_json::json!({
                    "file_path": "文件路径",
                    "function_name": "函数名"
                }),
            },
            ToolDefinition {
                name: "列出Skills".to_string(),
                command_type: "skills".to_string(),
                description: "列出所有可用的 Skills".to_string(),
                usage: "skills、有哪些技能、列出可用工具".to_string(),
                params_schema: serde_json::json!({}),
            },
            ToolDefinition {
                name: "查看配置".to_string(),
                command_type: "config".to_string(),
                description: "显示当前系统配置".to_string(),
                usage: "config、查看配置、当前设置是什么".to_string(),
                params_schema: serde_json::json!({}),
            },
            ToolDefinition {
                name: "切换LLM提供商".to_string(),
                command_type: "provider".to_string(),
                description: "显示或切换 LLM 提供商".to_string(),
                usage: "provider、切换到 openai、当前用的是哪个模型".to_string(),
                params_schema: serde_json::json!({
                    "name": "可选：提供商名称"
                }),
            },
            ToolDefinition {
                name: "列出LLM提供商".to_string(),
                command_type: "providers".to_string(),
                description: "列出所有可用的 LLM 提供商".to_string(),
                usage: "providers、有哪些模型可以用".to_string(),
                params_schema: serde_json::json!({}),
            },
            ToolDefinition {
                name: "创建新对话".to_string(),
                command_type: "conv_new".to_string(),
                description: "创建新的对话会话".to_string(),
                usage: "新对话、开始新话题、conv new".to_string(),
                params_schema: serde_json::json!({
                    "id": "可选：对话ID",
                    "system_prompt": "可选：系统提示"
                }),
            },
            ToolDefinition {
                name: "列出对话".to_string(),
                command_type: "conv_list".to_string(),
                description: "列出所有对话".to_string(),
                usage: "对话列表、有哪些对话".to_string(),
                params_schema: serde_json::json!({}),
            },
            ToolDefinition {
                name: "切换对话".to_string(),
                command_type: "conv_switch".to_string(),
                description: "切换到指定对话".to_string(),
                usage: "切换到对话1、switch conv 2".to_string(),
                params_schema: serde_json::json!({
                    "id": "对话ID"
                }),
            },
            ToolDefinition {
                name: "删除对话".to_string(),
                command_type: "conv_delete".to_string(),
                description: "删除指定对话".to_string(),
                usage: "删除对话2、delete conv old-chat".to_string(),
                params_schema: serde_json::json!({
                    "id": "对话ID"
                }),
            },
            ToolDefinition {
                name: "清空对话".to_string(),
                command_type: "conv_clear".to_string(),
                description: "清空当前对话历史".to_string(),
                usage: "清空历史、clear conv、重新开始".to_string(),
                params_schema: serde_json::json!({}),
            },
            ToolDefinition {
                name: "查询审计日志".to_string(),
                command_type: "audit".to_string(),
                description: "查询操作审计日志".to_string(),
                usage: "audit、查看日志、谁做了什么操作".to_string(),
                params_schema: serde_json::json!({
                    "user": "可选：用户ID",
                    "skill": "可选：Skill名称"
                }),
            },
            ToolDefinition {
                name: "清空屏幕".to_string(),
                command_type: "clear".to_string(),
                description: "清空终端屏幕".to_string(),
                usage: "clear、清屏".to_string(),
                params_schema: serde_json::json!({}),
            },
            ToolDefinition {
                name: "帮助".to_string(),
                command_type: "help".to_string(),
                description: "显示帮助信息".to_string(),
                usage: "help、帮助、怎么用".to_string(),
                params_schema: serde_json::json!({}),
            },
            ToolDefinition {
                name: "退出".to_string(),
                command_type: "exit".to_string(),
                description: "退出 REPL".to_string(),
                usage: "exit、退出、再见".to_string(),
                params_schema: serde_json::json!({}),
            },
        ];

        for skill in skill_registry.list_skills() {
            tools.push(ToolDefinition {
                name: format!("Skill: {}", skill.name),
                command_type: format!("skill_{}", skill.name.replace('.', "_")),
                description: skill.description.clone(),
                usage: format!("调用 {} 技能", skill.name),
                params_schema: serde_json::json!({}),
            });
        }

        tools
    }

    pub async fn parse(&self, input: &str) -> Result<Option<ParsedCommand>> {
        let prompt = self.build_parsing_prompt(input);
        
        let response = self.llm_client.chat(&[
            crate::llm::ChatMessage::system(&self.get_system_prompt()),
            crate::llm::ChatMessage::user(&prompt),
        ]).await?;

        self.parse_response(&response)
    }

    fn get_system_prompt(&self) -> String {
        let tools_json = serde_json::to_string_pretty(&self.tools).unwrap_or_default();
        
        format!(
            r#"你是一个智能命令解析器。你的任务是理解用户的自然语言输入，判断他们是否想要调用某个可用工具。

## 可用工具列表

{}

## 判断原则

1. **意图优先**：理解用户真正想做什么，而不是字面意思
2. **模糊匹配**：用户可能用各种方式表达同一个意图，比如"解释代码"、"看看这个文件"、"分析main.rs"都对应 explain 工具
3. **文件路径识别**：如果用户提到了文件名或路径（如 mod.rs、src/main.rs），通常是要操作代码的工具
4. **置信度评估**：
   - 0.9-1.0：用户明确表达了工具调用意图
   - 0.7-0.9：用户很可能想调用工具，但表达比较模糊
   - 0.5-0.7：不确定，可能是工具调用也可能是普通对话
   - <0.5：很可能是普通对话，不是工具调用

5. **普通对话**：如果用户只是问问题、聊天、表达想法，没有明确的工具调用意图，返回 is_command: false

## 返回格式

只返回 JSON，不要有其他内容：
{{
    "is_command": true 或 false,
    "command_type": "工具对应的 command_type 字段",
    "params": {{从用户输入中提取的参数对象}},
    "confidence": 0.0 到 1.0 之间的数字
}}"#,
            tools_json
        )
    }

    fn build_parsing_prompt(&self, input: &str) -> String {
        format!("用户输入：{}\n\n请分析用户意图，返回 JSON。", input)
    }

    fn parse_response(&self, response: &str) -> Result<Option<ParsedCommand>> {
        let json_str = self.extract_json(response);
        
        if json_str.is_empty() {
            return Ok(None);
        }

        match serde_json::from_str::<ParsedCommand>(&json_str) {
            Ok(parsed) => {
                if parsed.is_command && parsed.confidence >= 0.7 {
                    Ok(Some(parsed))
                } else {
                    Ok(None)
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse command: {}", e);
                Ok(None)
            }
        }
    }

    fn extract_json(&self, response: &str) -> String {
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                return response[start..=end].to_string();
            }
        }
        String::new()
    }

    pub fn parsed_to_command(&self, parsed: &ParsedCommand) -> Result<Command> {
        match parsed.command_type.as_str() {
            "explain" => {
                let file_path = parsed.params["file_path"].as_str().unwrap_or("").to_string();
                let function_name = parsed.params.get("function_name").and_then(|v| v.as_str()).map(|s| s.to_string());
                let line = parsed.params.get("line").and_then(|v| v.as_u64()).map(|n| n as usize);
                
                let line_range = line.map(|l| (l, l));
                Ok(Command::Explain { file_path, function_name, line_range })
            }
            "test" => {
                let file_path = parsed.params["file_path"].as_str().unwrap_or("").to_string();
                let function_name = parsed.params["function_name"].as_str().unwrap_or("").to_string();
                Ok(Command::Test { file_path, function_name })
            }
            "skills" => Ok(Command::Skills),
            "config" => Ok(Command::Config),
            "provider" => {
                let name = parsed.params.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                Ok(Command::Provider { name })
            }
            "providers" => Ok(Command::Providers),
            "conv_new" => {
                let id = parsed.params.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let system_prompt = parsed.params.get("system_prompt").and_then(|v| v.as_str()).map(|s| s.to_string());
                Ok(Command::Conversation {
                    action: crate::api::cli::commands::ConversationAction::New { id, system_prompt },
                })
            }
            "conv_list" => {
                Ok(Command::Conversation {
                    action: crate::api::cli::commands::ConversationAction::List,
                })
            }
            "conv_switch" => {
                let id = parsed.params["id"].as_str().unwrap_or("").to_string();
                Ok(Command::Conversation {
                    action: crate::api::cli::commands::ConversationAction::Switch { id },
                })
            }
            "conv_delete" => {
                let id = parsed.params["id"].as_str().unwrap_or("").to_string();
                Ok(Command::Conversation {
                    action: crate::api::cli::commands::ConversationAction::Delete { id },
                })
            }
            "conv_clear" => {
                Ok(Command::Conversation {
                    action: crate::api::cli::commands::ConversationAction::Clear,
                })
            }
            "audit" => {
                let user_id = parsed.params.get("user").and_then(|v| v.as_str()).map(|s| s.to_string());
                let skill_name = parsed.params.get("skill").and_then(|v| v.as_str()).map(|s| s.to_string());
                Ok(Command::Audit { user_id, skill_name })
            }
            "clear" => Ok(Command::Clear),
            "help" => Ok(Command::Help),
            "exit" => Ok(Command::Exit),
            skill_cmd if skill_cmd.starts_with("skill_") => {
                let skill_name = skill_cmd[6..].replace('_', ".");
                let file_path = parsed.params["file_path"].as_str().unwrap_or("").to_string();
                let function_name = parsed.params.get("function_name").and_then(|v| v.as_str()).map(|s| s.to_string());
                
                if skill_name == "code.explainer" {
                    Ok(Command::Explain { file_path, function_name: None, line_range: None })
                } else if skill_name == "code.test.generator" {
                    Ok(Command::Test { file_path, function_name: function_name.unwrap_or_default() })
                } else {
                    Err(crate::error::AgentHubError::Internal(format!("Unknown skill: {}", skill_name)))
                }
            }
            _ => Err(crate::error::AgentHubError::Internal(format!("Unknown command type: {}", parsed.command_type))),
        }
    }
}
