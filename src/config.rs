//! Config loading for Local Memory Engine.
//! Reads lme.toml from current directory or LME_CONFIG env var.

use serde::Deserialize;
use std::path::PathBuf;

use crate::error::LmeError;

/// Top-level config matching lme.toml structure.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub store: StoreConfig,

    #[serde(default)]
    pub decay: DecayConfig,

    #[serde(default)]
    pub embedding: EmbeddingConfig,

    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub limits: LimitsConfig,

    #[serde(default)]
    pub user: UserConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct StoreConfig {
    pub store_triggers: Vec<String>,
    pub context_triggers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DecayConfig {
    pub lambda_conversation: f64,
    pub lambda_knowledge: f64,
    pub lambda_learning: f64,
    pub lambda_decision: f64,
    pub lambda_architecture: f64,
    pub conf_inferred: f64,
    pub prune_threshold: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EmbeddingConfig {
    pub model: String,
    pub model_path: String,
    pub tokenizer_path: String,
    pub lazy_load: bool,
    pub idle_unload_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub path: String,
    pub wal_mode: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LimitsConfig {
    pub max_context_chars: usize,
    pub max_search_results: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UserConfig {
    pub owner_id: String,
}

// --- Default implementations ---

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            store_triggers: vec![
                "remember".into(),
                "save this".into(),
                "store memory".into(),
            ],
            context_triggers: vec![
                "recall context".into(),
                "what do we know".into(),
                "load context".into(),
            ],
        }
    }
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            lambda_conversation: 0.01,
            lambda_knowledge: 0.005,
            lambda_learning: 0.005,
            lambda_decision: 0.002,
            lambda_architecture: 0.001,
            conf_inferred: 0.6,
            prune_threshold: 0.01,
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: "bge-m3".into(),
            model_path: "./models/bge-m3-int8.onnx".into(),
            tokenizer_path: "./models/bge-m3-tokenizer.json".into(),
            lazy_load: true,
            idle_unload_secs: 300,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "./data/lme.db".into(),
            wal_mode: true,
        }
    }
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_context_chars: 50_000,
            max_search_results: 20,
        }
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            owner_id: "default-user".into(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            store: StoreConfig::default(),
            decay: DecayConfig::default(),
            embedding: EmbeddingConfig::default(),
            database: DatabaseConfig::default(),
            limits: LimitsConfig::default(),
            user: UserConfig::default(),
        }
    }
}

// --- Loader ---

impl Config {
    /// Load config from lme.toml in current dir, or LME_CONFIG env var path.
    pub fn load() -> Result<Self, LmeError> {
        let path = std::env::var("LME_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("lme.toml"));

        let content =
            std::fs::read_to_string(&path).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => {
                    LmeError::Config(format!(
                        "config file not found at '{}'. Create lme.toml or set LME_CONFIG env var",
                        path.display()
                    ))
                }
                _ => LmeError::Config(format!(
                    "failed to read config file '{}': {}",
                    path.display(),
                    e
                )),
            })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            LmeError::Config(format!(
                "invalid config at '{}': {}",
                path.display(),
                e
            ))
        })?;

        Ok(config)
    }
}
