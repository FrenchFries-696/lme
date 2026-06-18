//! ONNX session management using `ort` crate v2.0.0-rc.

use ort::session::Session;
use crate::error::LmeError;

/// Run ONNX inference and return last_hidden_state as flat f32 vector.
pub fn run_inference(
    session: &mut Session,
    input_ids: &[i64],
    attention_mask: &[i64],
) -> Result<(Vec<f32>, usize), LmeError> {
    use ort::value::Tensor;

    let seq_len = input_ids.len();

    // Create input tensors using (shape, data) tuple format
    // (avoids ndarray version conflicts with ort crate)
    let input_ids_tensor = Tensor::from_array((
        vec![1i64, seq_len as i64],
        input_ids.to_vec(),
    ))
    .map_err(|e| LmeError::Embedding(format!("input_ids tensor: {}", e)))?;

    let attention_mask_tensor = Tensor::from_array((
        vec![1i64, seq_len as i64],
        attention_mask.to_vec(),
    ))
    .map_err(|e| LmeError::Embedding(format!("attention_mask tensor: {}", e)))?;

    let outputs = session
        .run(ort::inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
        ])
        .map_err(|e| LmeError::Embedding(format!("inference failed: {}", e)))?;

    // Extract last_hidden_state — ort 2.0 returns (&Shape, &[T]) tuple
    let (shape, data) = outputs["last_hidden_state"]
        .try_extract_tensor::<f32>()
        .map_err(|e| LmeError::Embedding(format!("output extraction: {}", e)))?;

    let hidden_size = shape[2] as usize;
    let flat: Vec<f32> = data.to_vec();

    Ok((flat, hidden_size))
}

/// Load ONNX session from file.
pub fn load_session(model_path: &str) -> Result<Session, LmeError> {
    Session::builder()
        .map_err(|e| LmeError::Embedding(format!("session builder: {}", e)))?
        .commit_from_file(model_path)
        .map_err(|e| LmeError::Embedding(format!(
            "failed to load model '{}': {}. Is ONNX Runtime installed?",
            model_path, e
        )))
}
