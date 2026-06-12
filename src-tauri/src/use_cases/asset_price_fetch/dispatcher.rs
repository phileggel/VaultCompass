use crate::context::asset::{
    Asset, AssetPrice, AssetPriceRepository, AssetPriceSource, PriceProvider,
};
use crate::context::currency::{CurrencyPair, CurrencyService};
use crate::core::event_bus::Event;
use crate::core::logger::BACKEND;
use crate::core::SideEffectEventBus;
use chrono::NaiveDate;
use std::sync::Arc;
use std::time::Duration;

use super::guard::FetchGuardLease;

/// Injectable source of "today" so tests can fix the date deterministically.
pub type Clock = Arc<dyn Fn() -> NaiveDate + Send + Sync>;

/// Politeness delay inserted between successive provider requests in a fetch task.
/// Stooq's proof-of-work gate fingerprints non-browser bursts (L-005/L-006), so
/// spacing requests out keeps a launch fetch of a large portfolio from looking
/// like an abusive burst. It shapes request *cadence* only — it does not raise the
/// provider's per-key daily quota.
const INTER_FETCH_DELAY: Duration = Duration::from_millis(250);

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
        use_api_key: bool,
        stooq_key: Option<String>,
    ) {
        tokio::spawn(async move {
            let _lease = lease;
            let today = (self.clock)();
            // MKT-119 — tally the task outcome so the frontend can summarize it.
            let mut ok: u32 = 0;
            let mut skipped: u32 = 0;
            // KEY-044 — in KEYED mode with no stored Stooq key: skip the entire scope
            // without any per-asset provider call; every asset is reported as skipped.
            // KEY-053 — in KEYLESS mode this short-circuit is suppressed; the fetch
            // proceeds anonymously (`stooq_key` is None and stays None below).
            if use_api_key && stooq_key.is_none() {
                skipped = scope.len() as u32;
                tracing::warn!(
                    target: BACKEND,
                    skipped,
                    "asset_price_fetch: no Stooq key configured; skipping all assets (KEY-044)"
                );
                self.event_bus
                    .publish(Event::AssetPriceFetchCompleted { ok, skipped });
                if let Err(e) = self.currency_service.refresh_all_rates(fx_pairs).await {
                    tracing::warn!(
                        target: BACKEND,
                        err = ?e,
                        "asset_price_fetch: FX rate refresh failed (no Stooq key path)"
                    );
                }
                return;
            }
            // `Some(key)` in keyed mode; `None` in keyless mode (anonymous, KEY-053).
            for (index, (asset, symbol)) in scope.into_iter().enumerate() {
                // Space out requests after the first to avoid a burst (see
                // INTER_FETCH_DELAY); the provider is hit at most once per asset.
                if index > 0 {
                    tokio::time::sleep(INTER_FETCH_DELAY).await;
                }
                match self.provider.fetch_price(&symbol, stooq_key.clone()).await {
                    Ok(Some(quote)) => {
                        let record = AssetPrice::restore(
                            asset.id.clone(),
                            resolve_observation_date(quote.date.as_deref(), today),
                            quote.price,
                            AssetPriceSource::Stooq,
                        );
                        if let Err(e) = self.price_repo.upsert(record).await {
                            skipped += 1;
                            tracing::warn!(
                                target: BACKEND,
                                asset_id = %asset.id,
                                symbol = %symbol,
                                err = ?e,
                                "asset_price_fetch: upsert failed; skipping (MKT-114)"
                            );
                            continue;
                        }
                        ok += 1;
                        self.event_bus.publish(Event::AssetPriceUpdated);
                    }
                    Ok(None) => {
                        skipped += 1;
                        tracing::debug!(
                            target: BACKEND,
                            asset_id = %asset.id,
                            symbol = %symbol,
                            "asset_price_fetch: provider reports no data for symbol; skipping (MKT-114)"
                        );
                    }
                    Err(e) => {
                        skipped += 1;
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

            // MKT-119 — task-completion signal carrying the outcome counts. The
            // frontend surfaces a snackbar when `skipped > 0` (MKT-145).
            self.event_bus
                .publish(Event::AssetPriceFetchCompleted { ok, skipped });

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

/// Resolves the date a fetched price is stored under (MKT-118): the provider's
/// observation date when it is a well-formed ISO `yyyy-mm-dd` not after `today`,
/// otherwise `today`. Always returns a valid, non-future ISO date.
fn resolve_observation_date(provider_date: Option<&str>, today: NaiveDate) -> String {
    provider_date
        .and_then(|raw| NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok())
        .filter(|parsed| *parsed <= today)
        .unwrap_or(today)
        .format("%Y-%m-%d")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::resolve_observation_date;
    use chrono::NaiveDate;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 6, 7).expect("valid date")
    }

    #[test]
    fn uses_a_valid_past_observation_date() {
        assert_eq!(
            resolve_observation_date(Some("2026-06-05"), today()),
            "2026-06-05"
        );
    }

    #[test]
    fn uses_an_observation_date_equal_to_today() {
        assert_eq!(
            resolve_observation_date(Some("2026-06-07"), today()),
            "2026-06-07"
        );
    }

    #[test]
    fn falls_back_to_today_when_date_absent() {
        assert_eq!(resolve_observation_date(None, today()), "2026-06-07");
    }

    #[test]
    fn falls_back_to_today_when_date_malformed() {
        assert_eq!(
            resolve_observation_date(Some("not-a-date"), today()),
            "2026-06-07"
        );
        assert_eq!(
            resolve_observation_date(Some("2026-13-40"), today()),
            "2026-06-07"
        );
    }

    #[test]
    fn falls_back_to_today_when_date_in_future() {
        assert_eq!(
            resolve_observation_date(Some("2026-06-08"), today()),
            "2026-06-07"
        );
    }
}
