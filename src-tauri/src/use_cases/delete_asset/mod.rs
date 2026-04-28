/// Tauri command handler for asset hard-deletion.
mod api;
/// Cross-BC orchestrator: checks transaction history before delegating to AssetService.
mod orchestrator;

pub use api::*;
pub use orchestrator::{DeleteAssetError, DeleteAssetUseCase};
