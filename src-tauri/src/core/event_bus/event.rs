//! Domain events published across bounded contexts.

use serde::Serialize;

/// All possible side-effect events that can be published across the application.
/// Each variant represents a specific business event that features may need to react to.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, specta::Type, tauri_specta::Event)]
#[serde(tag = "type")]
pub enum Event {
    /// Health check event for testing/monitoring
    Health,
    /// An asset was created, updated, or deleted
    AssetUpdated,
    /// An account was created, updated, or deleted
    AccountUpdated,
    /// A category was created, updated, or deleted
    CategoryUpdated,
}
