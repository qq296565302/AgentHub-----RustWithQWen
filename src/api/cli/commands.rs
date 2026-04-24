
#[derive(Debug, Clone)]
pub enum Command {
    Explain {
        file_path: String,
        function_name: Option<String>,
        line_range: Option<(usize, usize)>,
    },
    Test {
        file_path: String,
        function_name: String,
    },
    Skills,
    Run {
        skill_name: String,
        params: String,
    },
    Audit {
        user_id: Option<String>,
        skill_name: Option<String>,
    },
    Config,
    Provider {
        name: Option<String>,
    },
    Providers,
    Conversation {
        action: ConversationAction,
    },
    Chat {
        message: String,
    },
    Clear,
    Help,
    Exit,
}

#[derive(Debug, Clone)]
pub enum ConversationAction {
    New { id: Option<String>, system_prompt: Option<String> },
    List,
    Switch { id: String },
    Delete { id: String },
    Clear,
    Show,
}

impl Command {
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        if input.is_empty() {
            return Err("Empty input".to_string());
        }

        if input.starts_with('/') {
            let trimmed = &input[1..];
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                return Err("Empty command".to_string());
            }

            match parts[0] {
                "explain" | "e" => Self::parse_explain(parts),
                "test" | "t" => Self::parse_test(parts),
                "skills" | "s" => Ok(Command::Skills),
                "run" | "r" => Self::parse_run(parts),
                "audit" | "a" => Self::parse_audit(parts),
                "config" | "c" => Ok(Command::Config),
                "provider" | "p" => Self::parse_provider(parts),
                "providers" | "ps" => Ok(Command::Providers),
                "conv" | "conversation" => Self::parse_conversation(parts),
                "clear" => Ok(Command::Clear),
                "help" | "h" | "?" => Ok(Command::Help),
                "exit" | "quit" | "q" => Ok(Command::Exit),
                _ => Err(format!("Unknown command: /{}. Type /help for available commands.", parts[0])),
            }
        } else {
            Ok(Command::Chat { message: input.to_string() })
        }
    }

    fn parse_provider(parts: Vec<&str>) -> Result<Self, String> {
        let name = parts.get(1).map(|s| s.to_string());
        Ok(Command::Provider { name })
    }

    fn parse_conversation(parts: Vec<&str>) -> Result<Self, String> {
        if parts.len() < 2 {
            return Ok(Command::Conversation {
                action: ConversationAction::Show,
            });
        }

        match parts[1] {
            "new" | "create" => {
                let id = parts.get(2).map(|s| s.to_string());
                let system_prompt = parts.get(3).map(|s| s.to_string());
                Ok(Command::Conversation {
                    action: ConversationAction::New { id, system_prompt },
                })
            }
            "list" | "ls" => Ok(Command::Conversation {
                action: ConversationAction::List,
            }),
            "switch" | "sw" | "use" => {
                let id = parts.get(2)
                    .ok_or("Usage: /conv switch <id>")?
                    .to_string();
                Ok(Command::Conversation {
                    action: ConversationAction::Switch { id },
                })
            }
            "delete" | "del" | "rm" => {
                let id = parts.get(2)
                    .ok_or("Usage: /conv delete <id>")?
                    .to_string();
                Ok(Command::Conversation {
                    action: ConversationAction::Delete { id },
                })
            }
            "clear" | "reset" => Ok(Command::Conversation {
                action: ConversationAction::Clear,
            }),
            _ => Ok(Command::Conversation {
                action: ConversationAction::Show,
            }),
        }
    }

    fn parse_explain(parts: Vec<&str>) -> Result<Self, String> {
        if parts.len() < 2 {
            return Err("Usage: /explain <file_path>[:line] or /explain <file_path>::<function>".to_string());
        }

        let target = parts[1];
        let (file_path, function_name, line_range) = if target.contains("::") {
            let parts: Vec<&str> = target.split("::").collect();
            (parts[0].to_string(), Some(parts[1].to_string()), None)
        } else if target.contains(':') {
            let parts: Vec<&str> = target.split(':').collect();
            let file_path = parts[0].to_string();
            if parts.len() == 3 {
                let start: usize = parts[1].parse().map_err(|_| "Invalid line number")?;
                let end: usize = parts[2].parse().map_err(|_| "Invalid line number")?;
                (file_path, None, Some((start, end)))
            } else {
                let line: usize = parts[1].parse().map_err(|_| "Invalid line number")?;
                (file_path, None, Some((line, line)))
            }
        } else {
            (target.to_string(), None, None)
        };

        Ok(Command::Explain {
            file_path,
            function_name,
            line_range,
        })
    }

    fn parse_test(parts: Vec<&str>) -> Result<Self, String> {
        if parts.len() < 3 {
            return Err("Usage: /test <file_path> <function_name>".to_string());
        }
        Ok(Command::Test {
            file_path: parts[1].to_string(),
            function_name: parts[2].to_string(),
        })
    }

    fn parse_run(parts: Vec<&str>) -> Result<Self, String> {
        if parts.len() < 2 {
            return Err("Usage: /run <skill_name> [params_json]".to_string());
        }
        let skill_name = parts[1].to_string();
        let params = if parts.len() > 2 {
            parts[2..].join(" ")
        } else {
            "{}".to_string()
        };
        Ok(Command::Run { skill_name, params })
    }

    fn parse_audit(parts: Vec<&str>) -> Result<Self, String> {
        let user_id = parts.get(1).map(|s| s.to_string());
        let skill_name = parts.get(2).map(|s| s.to_string());
        Ok(Command::Audit { user_id, skill_name })
    }
}

pub fn format_command_help() -> String {
    vec![
        ("/skills", "List available skills with descriptions"),
        ("/run <skill> [params]", "Execute a skill with optional JSON params"),
        ("/explain <file>[:line] | <file>::<fn>", "Explain code in a file"),
        ("/test <file> <function>", "Generate unit tests for a function"),
        ("/audit [user] [skill]", "Query audit logs"),
        ("/config", "Show current configuration"),
        ("/provider [name]", "Switch or show current LLM provider"),
        ("/providers", "List all configured LLM providers"),
        ("/conv [show]", "Show current conversation info"),
        ("/conv new [id] [prompt]", "Create a new conversation"),
        ("/conv list", "List all conversations"),
        ("/conv switch <id>", "Switch to a conversation"),
        ("/conv delete <id>", "Delete a conversation"),
        ("/conv clear", "Clear current conversation history"),
        ("/clear", "Clear conversation history"),
        ("/help", "Show this help message"),
        ("/exit", "Exit REPL"),
        ("<message>", "Chat with AI directly (with context memory)"),
    ]
    .iter()
    .map(|(cmd, desc)| format!("  {:45} {}", cmd, desc))
    .collect::<Vec<_>>()
    .join("\n")
}
