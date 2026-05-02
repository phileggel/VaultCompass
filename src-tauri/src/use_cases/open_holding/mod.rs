/// Tauri command handler for the open_holding operation.
mod api;
/// Cross-BC orchestrator: validates asset status before delegating to AccountService (TRX-050, TRX-056).
mod orchestrator;

pub use api::*;
pub use orchestrator::OpenHoldingUseCase;
