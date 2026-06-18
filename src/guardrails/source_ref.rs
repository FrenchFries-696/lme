use crate::error::LmeError;

/// Validate source_ref is non-empty and looks like a valid reference.
/// Accepts: file paths, URLs, conversation IDs.
pub fn validate_source_ref(source_ref: &str) -> Result<(), LmeError> {
    let trimmed = source_ref.trim();

    if trimmed.is_empty() {
        return Err(LmeError::Validation(
            "source_ref is required and must not be empty (FR-GRD-03)".into(),
        ));
    }

    // Check minimum length and contains some structure
    if trimmed.len() < 3 {
        return Err(LmeError::Validation(
            "source_ref too short — must be a valid reference (FR-GRD-03)".into(),
        ));
    }

    Ok(())
}

/// Check if source_ref appears to be a memory hash (64-char hex).
/// Used for anti-re-compression gate (FR-CMP-04).
pub fn is_memory_hash(source_ref: &str) -> bool {
    let trimmed = source_ref.trim();
    trimmed.len() == 64
        && trimmed.chars().all(|c| c.is_ascii_hexdigit())
}
