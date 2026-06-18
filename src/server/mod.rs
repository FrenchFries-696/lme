use std::sync::Arc;

use crate::config::Config;
use crate::decay::DecayEngine;
use crate::error::LmeError;
use crate::storage::Storage;

pub mod protocol;
pub mod transport;
pub mod tools;

use protocol::{JsonRpcRequest, JsonRpcResponse};

/// Application state shared across all tool handlers.
pub struct AppState {
    pub storage: Storage,
    pub config: Config,
    pub decay: DecayEngine,
    #[cfg(feature = "embedding")]
    pub embedder: std::sync::Mutex<crate::embedder::Embedder>,
}

/// Server entrypoint. Reads JSON-RPC from stdin, writes to stdout.
pub fn run(config: Config) -> Result<(), LmeError> {
    use std::io::{BufRead, BufReader, Write};

    let db_path = std::path::Path::new(&config.database.path);
    let storage = Storage::open(db_path, &config.database)?;
    let decay = DecayEngine::new(config.decay.clone());
    #[cfg(feature = "embedding")]
    let embedder = std::sync::Mutex::new(crate::embedder::Embedder::new(config.embedding.clone()));
    #[cfg(feature = "embedding")]
    let state = Arc::new(AppState { storage, config, decay, embedder });
    #[cfg(not(feature = "embedding"))]
    let state = Arc::new(AppState { storage, config, decay });

    tracing::info!("MCP server listening on stdio");

    let stdin = BufReader::new(std::io::stdin());
    let mut stdout = std::io::stdout();

    for line in stdin.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("stdin read error: {}", e);
                break;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let err = JsonRpcResponse::error(
                    None,
                    -32700,
                    format!("Parse error: {}", e),
                );
                let _ = writeln!(stdout, "{}", serde_json::to_string(&err).unwrap_or_default());
                let _ = stdout.flush();
                continue;
            }
        };

        let response = handle_message(&state, request);
        let json = serde_json::to_string(&response).unwrap_or_else(|_| {
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"internal serialization error"}}"#.to_string()
        });

        let safe_json = json.replace('\n', "\\n").replace('\r', "\\r");
        let _ = writeln!(stdout, "{}", safe_json);
        let _ = stdout.flush();
    }

    Ok(())
}

fn handle_message(state: &Arc<AppState>, request: JsonRpcRequest) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => tools::handle_initialize(&request),
        "tools/list" => tools::handle_list_tools(state, &request),
        "tools/call" => tools::handle_call_tool(state, &request),
        _ => JsonRpcResponse::error(
            request.id,
            -32601,
            format!("Method not found: {}", request.method),
        ),
    }
}
