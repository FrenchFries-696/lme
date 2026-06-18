pub mod config;
pub mod context;
pub mod decay;
#[cfg(feature = "embedding")]
pub mod download;
#[cfg(feature = "embedding")]
pub mod embedder;
pub mod error;
pub mod guardrails;
pub mod server;
pub mod storage;

use tracing::info;

fn main() {
    // Handle subcommands before full init
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            #[cfg(feature = "embedding")]
            "download-models" => {
                let filter = args.get(2).map(|s| s.as_str());
                if let Err(e) = download::download_models(filter) {
                    eprintln!("FATAL: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "--version" | "-V" => {
                println!("lme v{}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {
                eprintln!("unknown command: {}. Use --help for usage.", args[1]);
                std::process::exit(1);
            }
        }
    }

    // Init tracing — logs go to stderr (MCP protocol on stdout)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Load config
    let config = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FATAL: {}", e);
            std::process::exit(1);
        }
    };

    // Startup banner
    info!("lme v{} starting", env!("CARGO_PKG_VERSION"));
    info!("owner_id: {}", config.user.owner_id);
    info!("database: {}", config.database.path);
    #[cfg(feature = "embedding")]
    info!("embedding model: {} (ONNX)", config.embedding.model);
    #[cfg(not(feature = "embedding"))]
    info!("embedding: disabled (build with --features embedding for ONNX support)");
    info!(
        "store triggers: {}",
        config.store.store_triggers.len()
    );
    info!(
        "context triggers: {}",
        config.store.context_triggers.len()
    );
    info!(
        "decay lambdas — conv:{:.4} know:{:.4} learn:{:.4} dec:{:.4} arch:{:.4}",
        config.decay.lambda_conversation,
        config.decay.lambda_knowledge,
        config.decay.lambda_learning,
        config.decay.lambda_decision,
        config.decay.lambda_architecture,
    );

    // Start MCP server on stdio
    if let Err(e) = server::run(config) {
        eprintln!("FATAL: {}", e);
        std::process::exit(1);
    }
}

fn print_help() {
    println!("lme — Local Memory Engine for AI Agents");
    println!();
    println!("USAGE:");
    println!("  lme                    Start MCP server on stdio");
    #[cfg(feature = "embedding")]
    println!("  lme download-models    Download ONNX embedding models");
    println!("  lme --version          Show version");
    println!("  lme --help             Show this help");
    println!();
    println!("CONFIG:");
    println!("  lme.toml in current directory, or set LME_CONFIG env var");
}

