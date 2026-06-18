//! Tokenizer wrapper using HuggingFace `tokenizers` crate.

use crate::error::LmeError;

pub struct TokenizedInput {
    pub input_ids: Vec<i64>,
    pub attention_mask: Vec<i64>,
}

/// Load tokenizer from JSON file.
pub fn load_tokenizer(path: &str) -> Result<tokenizers::Tokenizer, LmeError> {
    tokenizers::Tokenizer::from_file(path)
        .map_err(|e| LmeError::Embedding(format!("failed to load tokenizer: {}", e)))
}

/// Encode text to input tensors.
/// Truncates at 512 tokens (model max length).
pub fn encode(
    tokenizer: &tokenizers::Tokenizer,
    text: &str,
) -> Result<TokenizedInput, LmeError> {
    let encoding = tokenizer
        .encode(text, true)
        .map_err(|e| LmeError::Embedding(format!("tokenization failed: {}", e)))?;

    let max_len = 512usize;
    let ids: Vec<i64> = encoding.get_ids().iter().take(max_len).map(|&id| id as i64).collect();
    let mask: Vec<i64> = encoding.get_attention_mask().iter().take(max_len).map(|&m| m as i64).collect();

    Ok(TokenizedInput {
        input_ids: ids,
        attention_mask: mask,
    })
}
