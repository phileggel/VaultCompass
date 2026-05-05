use crate::context::asset::AssetService;
use anyhow::Result;
use std::sync::Arc;

/// Ensures the system Cash Asset for `currency` exists in the asset catalog (CSH-010).
///
/// Stub during the holding-transaction consolidation refactor: returns `Ok(())`
/// without touching the catalog. The real implementation lands with the
/// cash-tracking spec (CSH).
#[allow(dead_code)]
pub async fn ensure_cash_asset(_asset_service: &Arc<AssetService>, _currency: &str) -> Result<()> {
    Ok(())
}
