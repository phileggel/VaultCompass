/// External API and Tauri commands.
mod api;
/// Core business entities and repository traits.
mod domain;
/// Data persistence implementations.
mod repository;
/// Coordination layer for business operations.
mod service;

pub use api::*;
pub use domain::*;
pub use repository::*;
pub use service::*;
