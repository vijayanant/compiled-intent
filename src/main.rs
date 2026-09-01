use compiled_intent::telemetry;
use compiled_intent::config::EngineConfig;
use compiled_intent::engine::CompitentEngine;
use compiled_intent::mcp::McpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_tracing();
    tracing::info!("Compitent Engine: Booting System Domain...");
    
    // Parse optional --config argument
    let mut args = std::env::args().skip(1);
    let mut config_path = None;
    while let Some(arg) = args.next() {
        if arg == "--config" {
            config_path = args.next();
        }
    }
    
    let config = EngineConfig::load(config_path);
    let engine = CompitentEngine::new(config);
    let server = McpServer::new(engine);

    server.run().await?;
    
    Ok(())
}
