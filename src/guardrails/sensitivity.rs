use crate::storage::models::Sensitivity;

/// Apply conservative sensitivity default (FR-GRD-06).
/// When agent doesn't specify, default to `Secret`.
pub fn default_sensitivity(explicit: Option<Sensitivity>) -> Sensitivity {
    explicit.unwrap_or(Sensitivity::Secret)
}

/// Gate: should this memory be visible in context/search?
/// Secret memories are hidden from aggregated views.
pub fn is_visible_in_context(sensitivity: Sensitivity) -> bool {
    sensitivity != Sensitivity::Secret
}
