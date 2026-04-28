/// Tauri command handler for asset archiving.
mod api;
/// Cross-BC orchestrator: checks active holdings before delegating to AssetService.
mod orchestrator;

pub use api::*;
pub use orchestrator::{ArchiveAssetError, ArchiveAssetUseCase};
