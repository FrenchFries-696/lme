use crate::error::LmeError;
use crate::storage::Storage;

/// Check for existing memory with same hash (FR-GRD-04).
pub fn check_dedup(storage: &Storage, hash: &str) -> Result<Option<String>, LmeError> {
    match storage.get_by_hash(hash) {
        Ok(memory) => Ok(Some(memory.hash)),
        Err(LmeError::NotFound(_)) => Ok(None),
        Err(e) => Err(e),
    }
}
