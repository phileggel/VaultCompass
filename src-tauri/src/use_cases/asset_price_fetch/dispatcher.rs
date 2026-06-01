use crate::context::asset::{
    Asset, AssetPrice, AssetPriceRepository, AssetPriceSource, PriceProvider,
};
use crate::context::currency::{CurrencyPair, CurrencyService};
use crate::core::event_bus::Event;
use crate::core::logger::BACKEND;
use crate::core::SideEffectEventBus;
use chrono::NaiveDate;
use std::sync::Arc;

use super::guard::FetchGuardLease;

/// Injectable source of "today" so tests can fix the date deterministically.
pub type Clock = Arc<dyn Fn() -> NaiveDate + Send + Sync>;

/// Dispatches the background per-asset price-fetch task (MKT-114, MKT-102, MKT-112).
pub struct Dispatcher {
    provider: Arc<dyn PriceProvider>,
    price_repo: Arc<dyn AssetPriceRepository>,
    event_bus: Arc<SideEffectEventBus>,
    currency_service: Arc<CurrencyService>,
    clock: Clock,
}

impl Dispatcher {
    /// Creates a new Dispatcher.
    pub fn new(
        provider: Arc<dyn PriceProvider>,
        price_repo: Arc<dyn AssetPriceRepository>,
        event_bus: Arc<SideEffectEventBus>,
        currency_service: Arc<CurrencyService>,
        clock: Clock,
    ) -> Self {
        Self {
            provider,
            price_repo,
            event_bus,
            currency_service,
            clock,
        }
    }

    /// Spawns a Tokio background task that fetches prices for the pre-derived
    /// `(Asset, symbol)` scope, then refreshes FX rates for `fx_pairs` plus all
    /// persisted pairs (FXR-075/076 — same task, same in-flight lease). The `lease`
    /// is moved into the task; its `Drop` releases the in-flight guard at task end,
    /// panic included (MKT-113).
    pub fn spawn(
        self: Arc<Self>,
        scope: Vec<(Asset, String)>,
        fx_pairs: Vec<CurrencyPair>,
        lease: FetchGuardLease,
    ) {
        tokio::spawn(async move {
            let _lease = lease;
            let today = (self.clock)();
            let date_string = today.format("%Y-%m-%d").to_string();
            for (asset, symbol) in scope {
                match self.provider.fetch_price(&symbol).await {
                    Ok(Some(price_micros)) => {
                        let record = AssetPrice::restore(
                            asset.id.clone(),
                            date_string.clone(),
                            price_micros,
                            AssetPriceSource::Stooq,
                        );
                        if let Err(e) = self.price_repo.upsert(record).await {
                            tracing::warn!(
                                target: BACKEND,
                                asset_id = %asset.id,
                                symbol = %symbol,
                                err = ?e,
                                "asset_price_fetch: upsert failed; skipping (MKT-114)"
                            );
                            continue;
                        }
                        self.event_bus.publish(Event::AssetPriceUpdated);
                    }
                    Ok(None) => {
                        tracing::debug!(
                            target: BACKEND,
                            asset_id = %asset.id,
                            symbol = %symbol,
                            "asset_price_fetch: provider reports no data for symbol; skipping (MKT-114)"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: BACKEND,
                            asset_id = %asset.id,
                            symbol = %symbol,
                            err = ?e,
                            "asset_price_fetch: provider fetch failed; skipping (MKT-114)"
                        );
                    }
                }
            }

            // FXR-075/076 — piggyback FX rate refresh on the same task and lease.
            // refresh_all_rates degrades internally (per-pair skips, provider
            // failure → no-op); a returned error is logged without aborting.
            if let Err(e) = self.currency_service.refresh_all_rates(fx_pairs).await {
                tracing::warn!(
                    target: BACKEND,
                    err = ?e,
                    "asset_price_fetch: FX rate refresh failed; prices already fetched"
                );
            }
        });
    }
}
