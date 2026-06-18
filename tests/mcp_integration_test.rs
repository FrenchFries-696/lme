use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Helper: spawn lme binary with a test config
fn spawn_lme(config_path: &str) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_lme"))
        .env("LME_CONFIG", config_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn lme")
}

/// Send a JSON-RPC request and read the response.
fn rpc_call(stdin: &mut std::process::ChildStdin, stdout: &mut BufReader<std::process::ChildStdout>, request: &str) -> String {
    writeln!(stdin, "{}", request).unwrap();
    stdin.flush().unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    line.trim().to_string()
}

/// Create a temporary config file for testing.
fn write_test_config(dir: &tempfile::TempDir) -> String {
    let config_path = dir.path().join("lme.toml");
    let db_path = dir.path().join("test.db");
    let config = format!(
        r#"
[database]
path = "{}"
wal_mode = true

[user]
owner_id = "test-user"
"#,
        db_path.display()
    );
    std::fs::write(&config_path, config).unwrap();
    config_path.to_string_lossy().to_string()
}

#[test]
fn test_initialize_and_list_tools() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_test_config(&dir);

    let mut child = spawn_lme(&config_path);
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stdout_reader = BufReader::new(stdout);

    // Initialize
    let init_response = rpc_call(
        &mut stdin,
        &mut stdout_reader,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#,
    );
    assert!(init_response.contains("lme"), "init response: {}", init_response);
    assert!(init_response.contains("2.0"), "should have jsonrpc: {}", init_response);

    // List tools
    let list_response = rpc_call(
        &mut stdin,
        &mut stdout_reader,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    );
    assert!(list_response.contains("lme_store"), "tools/list: {}", list_response);
    assert!(list_response.contains("lme_context"), "tools/list: {}", list_response);
    assert!(list_response.contains("lme_search"), "tools/list: {}", list_response);
    assert!(list_response.contains("lme_recall"), "tools/list: {}", list_response);
    assert!(list_response.contains("lme_status"), "tools/list: {}", list_response);

    child.kill().ok();
}

#[test]
fn test_lme_store_and_recall() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_test_config(&dir);

    let mut child = spawn_lme(&config_path);
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stdout_reader = BufReader::new(stdout);

    // Initialize
    rpc_call(&mut stdin, &mut stdout_reader,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#);

    // Store a memory
    let store_response = rpc_call(&mut stdin, &mut stdout_reader, r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"lme_store","arguments":{"project":"test-proj","memory_type":"knowledge","essence":"test integration","source_ref":"integration/test.md"}}}"#);
    assert!(store_response.contains("hash"), "store: {}", store_response);
    assert!(store_response.contains("stored"), "store: {}", store_response);

    // Extract hash from response
    let store_json: serde_json::Value = serde_json::from_str(&store_response).unwrap();
    let text = store_json["result"]["content"][0]["text"].as_str().unwrap();
    let result: serde_json::Value = serde_json::from_str(text).unwrap();
    let hash = result["hash"].as_str().unwrap();

    // Recall the memory
    let recall_response = rpc_call(&mut stdin, &mut stdout_reader,
        &format!(r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"lme_recall","arguments":{{"hash":"{}"}}}}}}"#, hash));
    assert!(recall_response.contains("test integration"), "recall: {}", recall_response);

    child.kill().ok();
}

#[test]
fn test_lme_store_and_search() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_test_config(&dir);

    let mut child = spawn_lme(&config_path);
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stdout_reader = BufReader::new(stdout);

    rpc_call(&mut stdin, &mut stdout_reader,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#);

    // Store memories with Vietnamese text
    rpc_call(&mut stdin, &mut stdout_reader, r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"lme_store","arguments":{"project":"vn-proj","memory_type":"knowledge","essence":"quyết định kiến trúc","facts":["dùng Rust"],"source_ref":"vn/meeting.md"}}}"#);

    // Search
    let search_response = rpc_call(&mut stdin, &mut stdout_reader, r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"lme_search","arguments":{"query":"quyết định","project":"vn-proj"}}}"#);
    assert!(search_response.contains("quyết định kiến trúc"), "search vietnamese: {}", search_response);

    child.kill().ok();
}

#[test]
fn test_parse_error_handling() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_test_config(&dir);

    let mut child = spawn_lme(&config_path);
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stdout_reader = BufReader::new(stdout);

    // Send invalid JSON
    let response = rpc_call(&mut stdin, &mut stdout_reader, "not valid json");
    assert!(response.contains("-32700"), "parse error: {}", response);

    // Server should still be running after parse error
    let init_response = rpc_call(&mut stdin, &mut stdout_reader,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#);
    assert!(init_response.contains("lme"), "should recover: {}", init_response);

    child.kill().ok();
}

#[test]
fn test_unknown_method() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_test_config(&dir);

    let mut child = spawn_lme(&config_path);
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stdout_reader = BufReader::new(stdout);

    let response = rpc_call(&mut stdin, &mut stdout_reader,
        r#"{"jsonrpc":"2.0","id":99,"method":"nonexistent_method","params":{}}"#);
    assert!(response.contains("-32601"), "method not found: {}", response);

    child.kill().ok();
}
