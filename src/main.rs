use compiled_intent::telemetry;
use compiled_intent::config::EngineConfig;
use compiled_intent::engine::ChronicleEngine;
use compiled_intent::mcp::McpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_tracing();
    tracing::info!("Chronicle Engine: Booting System Domain...");
    
    let config = EngineConfig::load();
    let engine = ChronicleEngine::new(config);
    let server = McpServer::new(engine);

    server.run().await?;
    
    Ok(())
}
