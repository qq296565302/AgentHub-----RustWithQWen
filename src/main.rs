mod config;
mod error;
mod llm;
mod parser;
mod skill;
mod guardrails;
mod audit;
mod api;
mod recovery;
mod metrics;
mod utils;
mod prompt;

use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Subcommand, Debug, Clone)]
enum SkillCommand {
    /// 列出所有已安装的 Skills
    List,
    /// 启用指定的 Skill
    Enable {
        /// Skill 名称
        name: String,
    },
    /// 禁用指定的 Skill
    Disable {
        /// Skill 名称
        name: String,
    },
    /// 在沙箱中运行指定的 Skill
    Run {
        /// Skill 名称
        name: String,
        /// JSON 格式的参数（可选）
        #[arg(default_value = "{}")]
        params: String,
    },
    /// 查看 Skill 详细信息
    Info {
        /// Skill 名称
        name: String,
    },
}

#[derive(Parser, Debug)]
#[command(name = "agenthub", version, about = "安全、可编程、Skill 驱动的本地 AI 执行引擎")]
struct Args {
    #[arg(short = 'c', long, help = "配置文件路径")]
    config: Option<String>,

    #[arg(short = 's', long, help = "启动 HTTP API 服务")]
    http: bool,

    #[arg(short = 'r', long, help = "启动 REPL 交互模式")]
    repl: bool,

    #[arg(long, help = "日志级别 (trace, debug, info, warn, error)", default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Skill 管理命令
    Skill {
        #[command(subcommand)]
        subcommand: SkillCommand,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(&args.log_level)
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = config::load_settings(args.config.as_deref())
        .map_err(|e| {
            tracing::error!("Failed to load configuration: {}", e);
            e
        })?;

    tracing::info!("AgentHub starting");
    tracing::info!("Configuration loaded successfully");
    tracing::debug!("Settings: {:?}", settings);

    if let Some(Commands::Skill { subcommand }) = &args.command {
        handle_skill_command(subcommand.clone()).await?;
        return Ok(());
    }

    if args.http {
        tracing::info!("Starting HTTP API server on {}:{}", settings.server.host, settings.server.port);
        api::run_server(settings).await?;
    } else if args.repl {
        tracing::info!("Starting REPL mode");
        api::run_repl(settings).await?;
    } else {
        println!("AgentHub v{}", env!("CARGO_PKG_VERSION"));
        println!("安全、可编程、Skill 驱动的本地 AI 执行引擎");
        println!();
        println!("Usage:");
        println!("  agenthub --repl          Start REPL interactive mode");
        println!("  agenthub --http          Start HTTP API server");
        println!("  agenthub skill list      List all installed skills");
        println!("  agenthub skill run <name> [params]  Run a skill in sandbox");
        println!("  agenthub --help          Show this help message");
    }

    Ok(())
}

async fn handle_skill_command(cmd: SkillCommand) -> Result<(), Box<dyn std::error::Error>> {
    use skill::manager::SkillManager;
    
    let skills_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("skills");
    
    let manager = SkillManager::new(skills_dir);

    match cmd {
        SkillCommand::List => {
            let skills = manager.list_skills()
                .map_err(|e| format!("列出 Skills 失败: {}", e))?;
            
            if skills.is_empty() {
                println!("未安装任何 Skill");
                println!();
                println!("安装方法: 将 Skill 目录放入 skills/ 目录");
                return Ok(());
            }

            let enabled_count = skills.iter().filter(|s| s.enabled).count();
            let disabled_count = skills.len() - enabled_count;

            println!("已安装的 Skills:\n");
            for skill in &skills {
                let status = if skill.enabled { "✅" } else { "❌" };
                let status_text = if skill.enabled { "已启用" } else { "已禁用" };
                
                println!("  {} {} v{} - {} ({})", 
                    status, skill.name, skill.version, skill.description, status_text);
                println!("     路径: {}", skill.wasm_path.display());
                println!("     内存限制: {}MB | 超时: {}s | 网络: {}", 
                    skill.config.max_memory_mb,
                    skill.config.max_execution_time_secs,
                    if skill.config.allow_network { "允许" } else { "禁止" });
                println!();
            }

            println!("共 {} 个 Skill ({} 启用, {} 禁用)", 
                skills.len(), enabled_count, disabled_count);
        }
        SkillCommand::Enable { name } => {
            manager.enable_skill(&name)
                .map_err(|e| format!("启用 Skill 失败: {}", e))?;
            println!("Skill '{}' 已启用", name);
        }
        SkillCommand::Disable { name } => {
            manager.disable_skill(&name)
                .map_err(|e| format!("禁用 Skill 失败: {}", e))?;
            println!("Skill '{}' 已禁用", name);
        }
        SkillCommand::Run { name, params } => {
            let params_value: serde_json::Value = serde_json::from_str(&params)
                .map_err(|e| format!("参数 JSON 解析失败: {}", e))?;
            
            println!("正在沙箱中运行 Skill: {}...", name);
            let output = manager.run_skill(&name, params_value).await
                .map_err(|e| format!("运行 Skill 失败: {}", e))?;
            
            println!("\n执行结果:");
            println!("{}", serde_json::to_string_pretty(&output)
                .unwrap_or_else(|_| output.to_string()));
        }
        SkillCommand::Info { name } => {
            let info = manager.get_skill_info(&name)
                .map_err(|e| format!("获取 Skill 信息失败: {}", e))?;
            
            println!("Skill 信息:");
            println!("  名称: {} {}", info.name, info.version);
            println!("  描述: {}", info.description);
            println!("  状态: {}", if info.enabled { "✅ 已启用" } else { "❌ 已禁用" });
            println!("  WASM 路径: {}", info.wasm_path.display());
            println!();
            println!("安全配置:");
            println!("  内存限制: {}MB", info.config.max_memory_mb);
            println!("  执行超时: {}s", info.config.max_execution_time_secs);
            println!("  网络访问: {}", if info.config.allow_network { "允许" } else { "禁止" });
            if !info.config.allowed_dirs.is_empty() {
                println!("  允许访问的目录:");
                for dir in &info.config.allowed_dirs {
                    println!("    - {}", dir.display());
                }
            }
        }
    }

    Ok(())
}
