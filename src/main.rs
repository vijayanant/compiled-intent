use clap::{Parser, Subcommand};
use compiled_intent::config::EngineConfig;
use compiled_intent::engine::CompitentEngine;
use compiled_intent::mcp::McpServer;
use compiled_intent::telemetry;

#[derive(Parser, Debug, PartialEq)]
#[command(name = "intent")]
#[command(version)]
#[command(about = "The Universal Semantic Filesystem CLI & MCP Server", long_about = None)]
pub struct Cli {
    /// Optional path to config file
    #[arg(long, global = true)]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Index a directory into the LanceDB database
    Index {
        /// The directory path to index
        path: String,
    },
    /// Search the semantic index
    Search {
        /// The query to search for
        query: String,
        /// Search mode (vector, fts, or hybrid)
        #[arg(long, default_value = "hybrid")]
        mode: String,
        /// Maximum number of results to return
        #[arg(long, default_value_t = 3)]
        limit: usize,
    },
    /// Start the Model Context Protocol (MCP) server on stdio (default)
    Serve,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_tracing();

    let cli = Cli::parse();
    let config = EngineConfig::load(cli.config);
    let engine = CompitentEngine::new(config);

    match &cli.command {
        Some(Commands::Index { path }) => {
            tracing::info!("Indexing directory: {}", path);
            compiled_intent::indexer::index_adrs(&engine, &[path]).await?;
            tracing::info!("Indexing complete!");
        }
        Some(Commands::Search { query, mode, limit }) => {
            tracing::info!("Searching for: {} (Mode: {})", query, mode);
            let results = engine.search(query, *limit, mode).await?;
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        Some(Commands::Serve) | None => {
            tracing::info!("Compitent Engine: Booting System Domain (MCP Stdio)...");
            let server = McpServer::new(engine);
            server.run().await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_zero_args_defaults_to_serve() {
        let cli = Cli::try_parse_from(["intent"]).expect("Zero args must parse successfully");
        assert_eq!(cli.command, None); // Will match `Some(Commands::Serve) | None`
        assert_eq!(cli.config, None);
    }

    #[test]
    fn test_cli_index_parsing() {
        let cli = Cli::try_parse_from(["intent", "index", "docs/adrs"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Index {
                path: "docs/adrs".to_string()
            })
        );
    }

    #[test]
    fn test_cli_search_defaults() {
        let cli = Cli::try_parse_from(["intent", "search", "architecture query"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Search {
                query: "architecture query".to_string(),
                mode: "hybrid".to_string(),
                limit: 3,
            })
        );
    }

    #[test]
    fn test_cli_search_custom_flags() {
        let cli = Cli::try_parse_from([
            "intent",
            "search",
            "SessionLedger",
            "--mode",
            "fts",
            "--limit",
            "10",
        ])
        .unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Search {
                query: "SessionLedger".to_string(),
                mode: "fts".to_string(),
                limit: 10,
            })
        );
    }

    #[test]
    fn test_cli_global_config_flag() {
        let cli = Cli::try_parse_from(["intent", "--config", "custom.toml", "serve"]).unwrap();
        assert_eq!(cli.config, Some("custom.toml".to_string()));
        assert_eq!(cli.command, Some(Commands::Serve));
    }

    #[test]
    fn test_cli_version_flag() {
        let err = Cli::try_parse_from(["intent", "--version"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }
}
