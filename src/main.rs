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

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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
        println!("  agenthub --help          Show this help message");
    }

    Ok(())
}
