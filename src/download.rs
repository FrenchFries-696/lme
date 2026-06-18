//! Model download via `lme download-models` subcommand.
//! Only available when `embedding` feature is enabled.
//! Downloads ONNX models + tokenizers from HuggingFace onnx-community.

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

/// Model manifest entry.
#[allow(dead_code)]
struct ModelEntry {
    name: &'static str,
    url: &'static str,
    filename: &'static str,
    sha256: &'static str,
    description: &'static str,
}

/// Manifest of available models.
const MODELS: &[ModelEntry] = &[
    // bge-m3 (primary — 1024-dim, multilingual, int8 quantized)
    ModelEntry {
        name: "bge-m3",
        url: "https://huggingface.co/onnx-community/bge-m3-ONNX/resolve/main/onnx/model_quantized.onnx",
        filename: "bge-m3-int8.onnx",
        sha256: "",
        description: "BGE-M3 int8 quantized (1024-dim, multilingual, ~558MB self-contained). Primary model for Vietnamese-English.",
    },
    ModelEntry {
        name: "bge-m3",
        url: "https://huggingface.co/onnx-community/bge-m3-ONNX/resolve/main/tokenizer.json",
        filename: "bge-m3-tokenizer.json",
        sha256: "",
        description: "BGE-M3 tokenizer (required with bge-m3 ONNX model).",
    },
    // multilingual-e5-small (fallback — 384-dim, lightweight)
    ModelEntry {
        name: "multilingual-e5-small",
        url: "https://huggingface.co/intfloat/multilingual-e5-small/resolve/main/onnx/model.onnx",
        filename: "multilingual-e5-small.onnx",
        sha256: "",
        description: "Multilingual-E5-small (384-dim, ~450MB). Lightweight fallback model.",
    },
    ModelEntry {
        name: "multilingual-e5-small",
        url: "https://huggingface.co/intfloat/multilingual-e5-small/resolve/main/tokenizer.json",
        filename: "multilingual-e5-small-tokenizer.json",
        sha256: "",
        description: "Multilingual-E5-small tokenizer.",
    },
];

/// Run the download-models subcommand.
pub fn download_models(model_filter: Option<&str>) -> Result<(), crate::error::LmeError> {
    let models_dir = PathBuf::from("./models");
    fs::create_dir_all(&models_dir).map_err(|e| {
        crate::error::LmeError::Embedding(format!("cannot create models dir: {}", e))
    })?;

    let entries: Vec<&ModelEntry> = if let Some(filter) = model_filter {
        MODELS.iter().filter(|m| m.name == filter).collect()
    } else {
        MODELS.iter().collect()
    };

    if entries.is_empty() {
        println!("No models match filter '{}'. Available: bge-m3, multilingual-e5-small, all", model_filter.unwrap_or(""));
        return Ok(());
    }

    println!("Downloading {} model files to ./models/...\n", entries.len());

    for entry in &entries {
        let dest = models_dir.join(entry.filename);
        println!("  {} → {}", entry.url, dest.display());

        let data = download_file(entry.url)?;

        // Verify size
        if data.is_empty() {
            return Err(crate::error::LmeError::Embedding(format!(
                "downloaded empty file from {}",
                entry.url
            )));
        }

        // Write to disk
        fs::write(&dest, &data).map_err(|e| {
            crate::error::LmeError::Embedding(format!("cannot write {}: {}", dest.display(), e))
        })?;

        let size_mb = data.len() as f64 / 1_048_576.0;
        let hash = format!("{:x}", Sha256::digest(&data));
        println!("    ✓ {} ({:.1} MB, sha256: {}...)", entry.filename, size_mb, &hash[..16]);
    }

    println!("\nDone. Models installed to ./models/");
    println!("Update lme.toml to point to your preferred model:");
    println!("  [embedding]");
    println!("  model = \"bge-m3\"  # or \"multilingual-e5-small\"");
    println!("  model_path = \"./models/bge-m3-int8.onnx\"");
    println!("  tokenizer_path = \"./models/bge-m3-tokenizer.json\"");

    Ok(())
}

/// Download a file from URL to bytes.
fn download_file(url: &str) -> Result<Vec<u8>, crate::error::LmeError> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| crate::error::LmeError::Embedding(format!(
            "download failed for {}: {}\n  Tip: check your internet connection or try again later.",
            url, e
        )))?;

    let mut data = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut data)
        .map_err(|e| crate::error::LmeError::Embedding(format!(
            "read error for {}: {}",
            url, e
        )))?;

    Ok(data)
}
