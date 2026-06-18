use lme::config::Config;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.user.owner_id, "default-user");
    assert_eq!(config.database.path, "./data/lme.db");
    assert_eq!(config.embedding.model, "bge-m3");
    assert_eq!(config.store.store_triggers.len(), 3);
    assert_eq!(config.store.context_triggers.len(), 3);
    assert_eq!(config.limits.max_context_chars, 50_000);
    assert_eq!(config.limits.max_search_results, 20);
    assert!(config.database.wal_mode);
    assert!(config.embedding.lazy_load);
    assert_eq!(config.embedding.idle_unload_secs, 300);
    assert_eq!(config.decay.lambda_conversation, 0.01);
    assert_eq!(config.decay.lambda_architecture, 0.001);
    assert_eq!(config.decay.conf_inferred, 0.6);
    assert_eq!(config.decay.prune_threshold, 0.01);
}

#[test]
fn test_parse_valid_toml() {
    let toml_str = r#"
[store]
store_triggers = ["alpha", "beta"]
context_triggers = ["gamma"]

[user]
owner_id = "test-user"
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.user.owner_id, "test-user");
    assert_eq!(config.store.store_triggers, vec!["alpha", "beta"]);
    assert_eq!(config.store.context_triggers, vec!["gamma"]);
    // Unspecified fields get defaults
    assert_eq!(config.database.path, "./data/lme.db");
    assert_eq!(config.embedding.model, "bge-m3");
}

#[test]
fn test_invalid_toml_fails() {
    let result: Result<Config, _> = toml::from_str("this is not valid toml {{{");
    assert!(result.is_err());
}

#[test]
fn test_empty_toml_uses_defaults() {
    let config: Config = toml::from_str("").unwrap();
    assert_eq!(config.user.owner_id, "default-user");
    assert_eq!(config.database.path, "./data/lme.db");
}

#[test]
fn test_partial_config_merges_defaults() {
    let toml_str = r#"
[database]
path = "/custom/path/lme.db"
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.database.path, "/custom/path/lme.db");
    // Other fields remain default
    assert_eq!(config.user.owner_id, "default-user");
}
