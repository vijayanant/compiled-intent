use clap::{Parser, Subcommand};
use compiled_intent::config::EngineConfig;
use compiled_intent::engine::CompitentEngine;

#[derive(Parser)]
#[command(name = "intent")]
#[command(about = "The Universal Semantic Filesystem CLI", long_about = None)]
struct Cli {
    /// Optional path to config file
    #[arg(long)]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index a directory into the LanceDB database
    Index {
        /// The directory path to index
        path: String,
    },
    /// Search the semantic index
    Search {
        /// The query to search for
        query: String,
        /// Maximum number of results to return
        #[arg(long, default_value_t = 3)]
        limit: usize,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    compiled_intent::telemetry::init_tracing();
    
    let cli = Cli::parse();
    let config = EngineConfig::load(cli.config);
    let engine = CompitentEngine::new(config);

    match &cli.command {
        Commands::Index { path } => {
            tracing::info!("Indexing directory: {}", path);
            compiled_intent::indexer::index_adrs(&engine, &[path.to_string()]).await?;
            tracing::info!("Indexing complete!");
        }
        Commands::Search { query, limit } => {
            tracing::info!("Searching for: {}", query);
            let results = engine.search(query, *limit).await?;
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
    }

    Ok(())
}
