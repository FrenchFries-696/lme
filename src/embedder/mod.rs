//! Text embedding via ONNX Runtime.
//! Only compiled when `embedding` feature is enabled.
//! Requires ONNX Runtime 1.18+ installed on the system.

use std::collections::HashMap;
use std::time::Instant;

use crate::config::EmbeddingConfig;
use crate::error::LmeError;

mod cosine;
mod onnx;
mod tokenize;

pub struct Embedder {
    config: EmbeddingConfig,
    session: Option<ort::session::Session>,
    tokenizer: Option<tokenizers::Tokenizer>,
    /// In-memory vector cache: project → [(hash, vector)]
    vector_cache: HashMap<String, Vec<(String, Vec<f32>)>>,
    last_used: Instant,
    model_info: ModelInfo,
}

#[derive(Clone)]
pub struct ModelInfo {
    pub name: String,
    pub dim: usize,
    pub loaded: bool,
}

impl Embedder {
    /// Create embedder. Does NOT load model yet (lazy).
    pub fn new(config: EmbeddingConfig) -> Self {
        let model_info = ModelInfo {
            name: config.model.clone(),
            dim: if config.model == "bge-m3" { 1024 } else { 384 },
            loaded: false,
        };
        Self {
            config,
            session: None,
            tokenizer: None,
            vector_cache: HashMap::new(),
            last_used: Instant::now(),
            model_info,
        }
    }

    /// Embed text. Loads model on first call.
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>, LmeError> {
        self.ensure_loaded()?;

        let tokenizer = self.tokenizer.as_ref().unwrap();

        // 1. Tokenize
        let encoding = tokenize::encode(tokenizer, text)?;

        // 2. Run ONNX inference
        let session = self.session.as_mut().unwrap();
        let (hidden, hidden_size) = onnx::run_inference(
            session,
            &encoding.input_ids,
            &encoding.attention_mask,
        )?;

        // 3. Mean pooling
        let pooled = Self::mean_pool(&hidden, &encoding.attention_mask, hidden_size);

        // 4. L2 normalize
        Ok(Self::l2_normalize(&pooled))
    }

    /// Cache vectors for a project (in-memory, avoids re-reading from DB).
    pub fn cache_vectors(&mut self, project: &str, vectors: Vec<(String, Vec<f32>)>) {
        self.vector_cache.insert(project.to_string(), vectors);
    }

    /// Invalidate cache for a project (called after new store).
    pub fn invalidate_cache(&mut self, project: &str) {
        self.vector_cache.remove(project);
    }

    /// Cosine search over cached vectors for a project.
    pub fn cosine_search(
        &self,
        query_vec: &[f32],
        project: &str,
        top_k: usize,
    ) -> Vec<(String, f32)> {
        match self.vector_cache.get(project) {
            Some(candidates) => cosine::cosine_search(query_vec, candidates, top_k),
            None => vec![],
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.model_info.loaded
    }

    pub fn info(&self) -> ModelInfo {
        self.model_info.clone()
    }

    /// Unload model to free RAM.
    pub fn unload(&mut self) {
        self.session = None;
        self.tokenizer = None;
        self.model_info.loaded = false;
        self.vector_cache.clear();
        tracing::info!("embedder: model unloaded");
    }

    /// Load or reload model if not already loaded.
    fn ensure_loaded(&mut self) -> Result<(), LmeError> {
        if self.session.is_some() {
            self.last_used = Instant::now();
            return Ok(());
        }

        let model_path = &self.config.model_path;
        let tokenizer_path = &self.config.tokenizer_path;

        // Verify files exist
        if !std::path::Path::new(model_path).exists() {
            return Err(LmeError::Embedding(format!(
                "model file not found: {}. Run 'lme download-models' first.",
                model_path
            )));
        }
        if !std::path::Path::new(tokenizer_path).exists() {
            return Err(LmeError::Embedding(format!(
                "tokenizer file not found: {}. Run 'lme download-models' first.",
                tokenizer_path
            )));
        }

        tracing::info!("loading embedding model: {}", self.config.model);

        self.session = Some(onnx::load_session(model_path)?);
        self.tokenizer = Some(tokenize::load_tokenizer(tokenizer_path)?);
        self.model_info.loaded = true;
        self.last_used = Instant::now();

        tracing::info!("embedder: model loaded ({})", self.config.model);
        Ok(())
    }

    /// Check if model should be unloaded (idle timeout).
    pub fn maybe_unload(&mut self) {
        if self.config.lazy_load
            && self.is_loaded()
            && self.last_used.elapsed().as_secs() > self.config.idle_unload_secs
        {
            self.unload();
        }
    }

    // --- Pooling helpers ---

    fn mean_pool(hidden: &[f32], attention_mask: &[i64], dim: usize) -> Vec<f32> {
        let seq_len = attention_mask.len();
        let hidden_size = dim;
        let mut pooled = vec![0.0f32; hidden_size];

        for i in 0..seq_len {
            let mask_val = attention_mask[i] as f32;
            if mask_val == 0.0 {
                continue;
            }
            for j in 0..hidden_size {
                pooled[j] += hidden[i * hidden_size + j] * mask_val;
            }
        }

        let mask_sum: f32 = attention_mask.iter().map(|&m| m as f32).sum();
        if mask_sum > 0.0 {
            for v in &mut pooled {
                *v /= mask_sum;
            }
        }

        pooled
    }

    fn l2_normalize(vec: &[f32]) -> Vec<f32> {
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter().map(|x| x / norm).collect()
        } else {
            vec.to_vec()
        }
    }
}
