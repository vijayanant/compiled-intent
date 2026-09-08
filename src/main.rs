use clap::Parser;
use compiled_intent::config::EngineConfig;
use compiled_intent::engine::CompitentEngine;
use compiled_intent::mcp::McpServer;
use compiled_intent::telemetry;

#[derive(Parser)]
#[command(name = "compiled-intent", about = "Compitent MCP Server")]
struct Args {
    /// Optional path to configuration file
    #[arg(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_tracing();
    tracing::info!("Compitent Engine: Booting System Domain...");

    let args = Args::parse();
    let config = EngineConfig::load(args.config);
    let engine = CompitentEngine::new(config);
    let server = McpServer::new(engine);

    server.run().await?;

    Ok(())
}
