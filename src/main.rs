use anyhow::Result;
use clap::Parser;
use std::env as std_env;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod config;
mod app;
mod session;
mod tui;
mod llm;
mod lsp;
mod mcp;
mod utils;
mod permission;
mod version;
mod shell;
mod db;
mod message;
mod history;
mod csync;
mod format;
mod ansiext;
mod env;
mod diff;
mod pubsub;
mod fsext;
mod log;

use cli::Cli;

#[tokio::main]
async fn main() {
    // Set up panic hook for graceful error recovery
    std::panic::set_hook(Box::new(|panic_info| {
        error!("Application panicked: {}", panic_info);
        std::process::exit(1);
    }));

    // Load environment variables from .env file
    if let Err(e) = dotenvy::dotenv() {
        // Don't error if .env file doesn't exist, just log it
        tracing::debug!("No .env file found or error loading it: {}", e);
    }

    // Initialize logging/tracing
    let result = init_logging();
    if let Err(e) = result {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }

    // Start profiling server if enabled
    if let Ok(profile_port) = std_env::var("GOOFY_PROFILE") {
        start_profiling_server(&profile_port).await;
    }

    // Execute CLI command
    if let Err(e) = execute().await {
        error!("Application error: {}", e);
        std::process::exit(1);
    }
}

fn init_logging() -> Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "goofy=info".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;

    Ok(())
}

async fn start_profiling_server(port: &str) {
    let addr = format!("127.0.0.1:{}", port);
    info!("Starting profiling server on http://{}", addr);
    
    tokio::spawn(async move {
        // In a real implementation, you would set up a profiling endpoint here
        // For now, we'll just log that profiling would be enabled
        info!("Profiling server would be running on {}", addr);
    });
}

async fn execute() -> Result<()> {
    let cli = Cli::parse();
    cli.execute().await
}