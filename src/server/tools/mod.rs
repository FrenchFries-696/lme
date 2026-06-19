use std::sync::Arc;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::LmeError;

use crate::server::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::server::AppState;

mod context;
mod recall;
mod search;
mod status;
mod store;

/// Handle MCP "initialize" request — return server capabilities.
pub fn handle_initialize(request: &JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);
    JsonRpcResponse::success(
        id,
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "lme",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

/// Handle MCP "tools/list" — return all registered tools with their schemas.
pub fn handle_list_tools(state: &Arc<AppState>, request: &JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);
    let tools = build_tool_defs(&state.config);
    JsonRpcResponse::success(id, json!({ "tools": tools }))
}

/// Handle MCP "tools/call" — dispatch to the correct tool handler by name.
pub fn handle_call_tool(state: &Arc<AppState>, request: &JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);

    let params = match &request.params {
        Some(p) => p.clone(),
        None => {
            return JsonRpcResponse::error(
                Some(id),
                -32602,
                "Missing params".into(),
            );
        }
    };

    let tool_name = params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

    let result = match tool_name {
        "lme_store" => store::lme_store(state, arguments),
        "lme_context" => context::lme_context(state, arguments),
        "lme_search" => search::lme_search(state, arguments),
        "lme_recall" => recall::lme_recall(state, arguments),
        "lme_status" => status::lme_status(state, arguments),
        _ => Err(LmeError::Validation(format!(
            "Unknown tool: {}",
            tool_name
        ))),
    };

    match result {
        Ok(content) => JsonRpcResponse::success(
            id,
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string(&content).unwrap_or_default()
                    }
                ]
            }),
        ),
        Err(e) => JsonRpcResponse::success(
            id,
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("error: {}", e)
                    }
                ],
                "isError": true
            }),
        ),
    }
}

/// Build tool definitions with dynamic trigger injection from config (FR-MCP-03).
fn build_tool_defs(config: &Config) -> Vec<Value> {
    let store_triggers = config.store.store_triggers.join(", ");
    let context_triggers = config.store.context_triggers.join(", ");

    vec![
        json!({
            "name": "lme_store",
            "description": format!(
                "Store a new memory unit. Triggers: {}. Use when user asks to remember, save, or store information.",
                store_triggers
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": {"type": "string", "description": "Project namespace for this memory"},
                    "memory_type": {"type": "string", "enum": ["conversation","knowledge","learning","decision","architecture"]},
                    "essence": {"type": "string", "description": "One-line summary of the memory"},
                    "summary": {"type": "string", "description": "Detailed abstractive summary"},
                    "facts": {"type": "array", "items": {"type": "string"}, "description": "Extractive facts (exact values, no paraphrase)"},
                    "source_ref": {"type": "string", "description": "Source reference (file path, URL, conversation ID)"},
                    "importance": {"type": "integer", "minimum": 1, "maximum": 5, "default": 3},
                    "sensitivity": {"type": "string", "enum": ["public","internal","secret"], "default": "secret", "description": "Default: secret (conservative)"},
                    "tags": {"type": "array", "items": {"type": "string"}, "default": []}
                },
                "required": ["project", "memory_type", "essence", "source_ref"]
            }
        }),
        json!({
            "name": "lme_context",
            "description": format!(
                "PRIMARY project memory. Loads everything stored about a project, ranked by importance and recency. Use FIRST when user asks 'do you know about X', 'tell me about', 'what do you remember', 'catch me up', or starts a new session. Triggers: {}.",
                context_triggers
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": {"type": "string"},
                    "char_budget": {"type": "integer", "default": 50000}
                },
                "required": ["project"]
            }
        }),
        json!({
            "name": "lme_search",
            "description": "Search project memory by keyword or concept. Use when user asks about specific facts, past decisions, or looks for stored information. Try BEFORE relying on internal knowledge — this is the user's actual project notes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "project": {"type": "string"},
                    "limit": {"type": "integer", "default": 10},
                    "mode": {"type": "string", "enum": ["auto","semantic","keyword"], "default": "auto"}
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "lme_recall",
            "description": "Recall a specific memory by its hash. Falls back to prefix match within project if hash lookup fails (e.g. hash corrupted in transit).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hash": {"type": "string", "description": "SHA-256 hash of the memory unit"},
                    "project": {"type": "string", "description": "Required for fallback: project to search for prefix match when exact hash fails"}
                },
                "required": ["hash"]
            }
        }),
        json!({
            "name": "lme_status",
            "description": "Get memory engine status: total memories, by type, database health, embedding backend.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": {"type": "string", "description": "Optional: filter by project"}
                },
                "required": []
            }
        }),
    ]
}
