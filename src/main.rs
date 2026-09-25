use clap::{Parser, Subcommand};
use compiled_intent::config::EngineConfig;
use compiled_intent::engine::CompitentEngine;
use compiled_intent::mcp::McpServer;
use compiled_intent::telemetry;

#[derive(Parser, Debug, PartialEq)]
#[command(name = "intent")]
#[command(about = "The Universal Semantic Filesystem CLI & MCP Server", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Optional path to config file
    #[arg(long, global = true)]
    pub config: Option<String>,

    /// Enable verbose logging output
    #[arg(short, long, global = true)]
    pub verbose: bool,

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
        /// Minimum normalized confidence score floor in [0.0, 1.0]
        #[arg(long)]
        min_score: Option<f32>,
        /// Relative score tolerance in [0.0, 1.0] (e.g. 0.15 = prune if score < (1.0 - 0.15) * `top_score`)
        #[arg(long)]
        score_tolerance: Option<f32>,
    },
    /// Ask a question about the indexed documentation
    Ask {
        /// The question to answer
        question: String,
        /// LLM model name (overrides config)
        #[arg(long)]
        model: Option<String>,
        /// LLM HTTP chat endpoint (overrides config)
        #[arg(long)]
        endpoint: Option<String>,
        /// Maximum candidate chunks to retrieve before collapsing
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Minimum normalized confidence score floor in [0.0, 1.0]
        #[arg(long, default_value_t = 0.0)]
        min_score: f32,
        /// Relative score tolerance in [0.0, 1.0]
        #[arg(long, default_value_t = 0.15)]
        score_tolerance: f32,
        /// Search mode (hybrid, fts, or vector)
        #[arg(long, default_value = "hybrid")]
        mode: String,
        /// Disable citation summary footer
        #[arg(long)]
        no_sources: bool,
    },
    /// Start the Model Context Protocol (MCP) server on stdio (default)
    Serve,
}

struct AskArgs<'a> {
    question: &'a str,
    model: Option<&'a str>,
    endpoint: Option<&'a str>,
    limit: usize,
    min_score: f32,
    score_tolerance: f32,
    mode: &'a str,
    no_sources: bool,
}

fn print_sources(results: &[serde_json::Value]) {
    println!("\n---");
    println!("Sources consulted:");
    for item in results {
        let chunk_id = item
            .get("chunk_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let score = item.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        println!("- {} (relevance: {:.2})", chunk_id, score);
    }
}

async fn handle_search(
    engine: &CompitentEngine,
    query: &str,
    mode: &str,
    limit: usize,
    filters: compiled_intent::engine::SearchFilterOptions,
) -> anyhow::Result<()> {
    tracing::info!("Searching for: {} (Mode: {})", query, mode);
    let results = engine
        .search_with_options(query, limit, mode, filters)
        .await?;
    println!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}

async fn handle_ask(
    engine: &CompitentEngine,
    config: &EngineConfig,
    args: &AskArgs<'_>,
) -> anyhow::Result<()> {
    let filters = compiled_intent::engine::SearchFilterOptions {
        min_score: Some(args.min_score),
        score_tolerance: Some(args.score_tolerance),
    };
    let results = engine
        .search_with_options(args.question, args.limit, args.mode, filters)
        .await?;
    if results.is_empty() {
        println!(
            "The documentation does not currently contain information related to: \"{}\"",
            args.question
        );
        return Ok(());
    }

    let (sys_prompt, user_prompt) =
        compiled_intent::llm::build_grounding_prompt(args.question, &results);
    let target_endpoint = args.endpoint.unwrap_or(&config.llm.endpoint);
    let target_model = args.model.unwrap_or(&config.llm.model);

    let client = reqwest::Client::new();
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    compiled_intent::llm::stream_chat_completion(
        &client,
        target_endpoint,
        target_model,
        config.llm.temperature,
        &sys_prompt,
        &user_prompt,
        &mut handle,
    )
    .await?;

    println!();
    if !args.no_sources {
        print_sources(&results);
    }
    Ok(())
}

async fn dispatch_command(
    cli: &Cli,
    engine: CompitentEngine,
    config: &EngineConfig,
) -> anyhow::Result<()> {
    match &cli.command {
        Some(Commands::Index { path }) => {
            tracing::info!("Indexing directory: {}", path);
            compiled_intent::indexer::index_adrs(&engine, &[path]).await?;
            tracing::info!("Indexing complete!");
        }
        Some(Commands::Search {
            query,
            mode,
            limit,
            min_score,
            score_tolerance,
        }) => {
            let filters = compiled_intent::engine::SearchFilterOptions {
                min_score: *min_score,
                score_tolerance: *score_tolerance,
            };
            handle_search(&engine, query, mode, *limit, filters).await?;
        }
        Some(Commands::Ask {
            question,
            model,
            endpoint,
            limit,
            min_score,
            score_tolerance,
            mode,
            no_sources,
        }) => {
            let args = AskArgs {
                question,
                model: model.as_deref(),
                endpoint: endpoint.as_deref(),
                limit: *limit,
                min_score: *min_score,
                score_tolerance: *score_tolerance,
                mode,
                no_sources: *no_sources,
            };
            handle_ask(&engine, config, &args).await?;
        }
        Some(Commands::Serve) | None => {
            tracing::info!("Compitent Engine: Booting System Domain (MCP Stdio)...");
            let server = McpServer::new(engine);
            server.run().await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = EngineConfig::load(cli.config.clone());
    telemetry::init_tracing(cli.verbose, Some(&config.log_level));
    let engine = CompitentEngine::new(config.clone());
    dispatch_command(&cli, engine, &config).await
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
                min_score: None,
                score_tolerance: None,
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
            "--min-score",
            "0.5",
            "--score-tolerance",
            "0.15",
        ])
        .unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Search {
                query: "SessionLedger".to_string(),
                mode: "fts".to_string(),
                limit: 10,
                min_score: Some(0.5),
                score_tolerance: Some(0.15),
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
    fn test_cli_ask_defaults() {
        let cli = Cli::try_parse_from(["intent", "ask", "Why LanceDB?"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Ask {
                question: "Why LanceDB?".to_string(),
                model: None,
                endpoint: None,
                limit: 10,
                min_score: 0.0,
                score_tolerance: 0.15,
                mode: "hybrid".to_string(),
                no_sources: false,
            })
        );
    }

    #[test]
    fn test_cli_ask_custom_flags() {
        let cli = Cli::try_parse_from([
            "intent",
            "ask",
            "How does manifest sync work?",
            "--model",
            "qwen2.5-coder:7b",
            "--endpoint",
            "http://remote:11434/api/chat",
            "--limit",
            "5",
            "--min-score",
            "0.3",
            "--score-tolerance",
            "0.1",
            "--mode",
            "fts",
            "--no-sources",
        ])
        .unwrap();
        assert_eq!(
            cli.command,
            Some(Commands::Ask {
                question: "How does manifest sync work?".to_string(),
                model: Some("qwen2.5-coder:7b".to_string()),
                endpoint: Some("http://remote:11434/api/chat".to_string()),
                limit: 5,
                min_score: 0.3,
                score_tolerance: 0.1,
                mode: "fts".to_string(),
                no_sources: true,
            })
        );
    }

    #[test]
    fn test_cli_verbose_flag() {
        let cli_default = Cli::try_parse_from(["intent"]).unwrap();
        assert!(!cli_default.verbose);

        let cli_verbose = Cli::try_parse_from(["intent", "-v"]).unwrap();
        assert!(cli_verbose.verbose);

        let cli_verbose_long = Cli::try_parse_from(["intent", "--verbose", "serve"]).unwrap();
        assert!(cli_verbose_long.verbose);
    }

    #[test]
    fn test_cli_version_flag() {
        let err = Cli::try_parse_from(["intent", "--version"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }
}
