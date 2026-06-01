use crate::context::currency::domain::{
    CurrencyPair, CurrencyPairRepository, CurrencyPairSummary, CurrencyRate,
    CurrencyRateRepository, CurrencyRateSource,
};
use crate::context::currency::error::CurrencyError;
use crate::core::{Event, SideEffectEventBus, BACKEND};
use std::result::Result as StdResult;
use std::sync::Arc;

/// Orchestrates the manual rate CRUD operations for the `currency` bounded context.
pub struct CurrencyService {
    pair_repo: Box<dyn CurrencyPairRepository>,
    rate_repo: Box<dyn CurrencyRateRepository>,
    event_bus: Option<Arc<SideEffectEventBus>>,
}

impl CurrencyService {
    /// Creates a new CurrencyService with the given repositories.
    pub fn new(
        pair_repo: Box<dyn CurrencyPairRepository>,
        rate_repo: Box<dyn CurrencyRateRepository>,
    ) -> Self {
        Self {
            pair_repo,
            rate_repo,
            event_bus: None,
        }
    }

    /// Attaches an event bus for side-effect notifications.
    pub fn with_event_bus(mut self, bus: Arc<SideEffectEventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Publishes `CurrencyRateUpdated` when an event bus is attached (FXR-026).
    fn notify_rate_updated(&self) {
        if let Some(bus) = &self.event_bus {
            bus.publish(Event::CurrencyRateUpdated);
        }
    }

    /// Idempotently declares a currency pair (FXR-054).
    /// Returns the existing pair if it is already present; creates it otherwise.
    pub async fn declare_currency_pair(
        &self,
        from_currency: String,
        to_currency: String,
    ) -> StdResult<CurrencyPair, CurrencyError> {
        let pair = CurrencyPair::new(from_currency, to_currency)?;
        self.pair_repo.upsert_pair(pair).await.map_err(|e| {
            tracing::error!(target: BACKEND, err = ?e, "declare_currency_pair: repository failure");
            CurrencyError::DatabaseError
        })
    }

    /// Records a rate for a pair, ensuring the pair exists first (FXR-013 ergonomics).
    /// Sets `source = Manual` (FXR-101). Upserts by `(from, to, date)` (FXR-025).
    /// Publishes `CurrencyRateUpdated` on success (FXR-026).
    pub async fn record_currency_rate(
        &self,
        from_currency: String,
        to_currency: String,
        date: String,
        rate_micros: i64,
    ) -> StdResult<CurrencyRate, CurrencyError> {
        let rate = CurrencyRate::new(
            from_currency,
            to_currency,
            date,
            rate_micros,
            CurrencyRateSource::Manual,
        )?;

        // The rate's currencies are already validated by `CurrencyRate::new`, so the
        // pair is reconstructed without re-validation (FXR-013 ensure-pair side-effect).
        let pair = CurrencyPair::from_storage(rate.from_currency.clone(), rate.to_currency.clone());
        self.pair_repo.upsert_pair(pair).await.map_err(|e| {
            tracing::error!(target: BACKEND, err = ?e, "record_currency_rate: pair upsert failure");
            CurrencyError::DatabaseError
        })?;

        let saved = self.rate_repo.upsert_rate(rate).await.map_err(|e| {
            tracing::error!(target: BACKEND, err = ?e, "record_currency_rate: rate upsert failure");
            CurrencyError::DatabaseError
        })?;

        self.notify_rate_updated();
        Ok(saved)
    }

    /// Updates an existing rate (FXR-052).
    ///
    /// - Same date: in-place overwrite of the rate value.
    /// - Different date: deletes the original record and upserts at the new date.
    ///
    /// Returns `RateNotFound` when the original `(from, to, original_date)` does not exist.
    /// Sets `source = Manual` (FXR-101). Publishes `CurrencyRateUpdated` on success (FXR-052).
    pub async fn update_currency_rate(
        &self,
        from_currency: String,
        to_currency: String,
        original_date: String,
        new_date: String,
        new_rate_micros: i64,
    ) -> StdResult<(), CurrencyError> {
        self.rate_repo
            .get_by_key(&from_currency, &to_currency, &original_date)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "update_currency_rate: get_by_key failure");
                CurrencyError::DatabaseError
            })?
            .ok_or_else(|| CurrencyError::RateNotFound {
                from_currency: from_currency.clone(),
                to_currency: to_currency.clone(),
                date: original_date.clone(),
            })?;

        let new_rate = CurrencyRate::new(
            from_currency.clone(),
            to_currency.clone(),
            new_date.clone(),
            new_rate_micros,
            CurrencyRateSource::Manual,
        )?;

        if original_date != new_date {
            self.rate_repo
                .delete_rate(&from_currency, &to_currency, &original_date)
                .await
                .map_err(|e| {
                    tracing::error!(target: BACKEND, err = ?e, "update_currency_rate: delete_rate failure");
                    CurrencyError::DatabaseError
                })?;
        }

        self.rate_repo.upsert_rate(new_rate).await.map_err(|e| {
            tracing::error!(target: BACKEND, err = ?e, "update_currency_rate: upsert_rate failure");
            CurrencyError::DatabaseError
        })?;

        self.notify_rate_updated();
        Ok(())
    }

    /// Deletes the rate at `(from_currency, to_currency, date)` (FXR-053).
    /// Returns `RateNotFound` when the record does not exist.
    /// Never removes the pair (FXR-014).
    /// Publishes `CurrencyRateUpdated` on success (FXR-053).
    pub async fn delete_currency_rate(
        &self,
        from_currency: String,
        to_currency: String,
        date: String,
    ) -> StdResult<(), CurrencyError> {
        self.rate_repo
            .get_by_key(&from_currency, &to_currency, &date)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "delete_currency_rate: get_by_key failure");
                CurrencyError::DatabaseError
            })?
            .ok_or_else(|| CurrencyError::RateNotFound {
                from_currency: from_currency.clone(),
                to_currency: to_currency.clone(),
                date: date.clone(),
            })?;

        self.rate_repo
            .delete_rate(&from_currency, &to_currency, &date)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "delete_currency_rate: delete_rate failure");
                CurrencyError::DatabaseError
            })?;

        self.notify_rate_updated();
        Ok(())
    }

    /// Returns all persisted pairs enriched with their most-recent rate (FXR-051).
    pub async fn list_currency_pairs(&self) -> StdResult<Vec<CurrencyPairSummary>, CurrencyError> {
        self.pair_repo
            .list_pairs_with_latest_rate()
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "list_currency_pairs: repository failure");
                CurrencyError::DatabaseError
            })
    }

    /// Returns all rates for the given pair ordered by date descending (FXR-050).
    /// Returns an empty list for an unknown pair — never `RateNotFound`.
    pub async fn list_currency_rates(
        &self,
        from_currency: String,
        to_currency: String,
    ) -> StdResult<Vec<CurrencyRate>, CurrencyError> {
        self.rate_repo
            .list_rates_for_pair(&from_currency, &to_currency)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "list_currency_rates: repository failure");
                CurrencyError::DatabaseError
            })
    }

    /// Resolves the conversion rate (in micros) for valuing a holding priced in
    /// `from_currency` into `to_currency`, as of `as_of` (FXR-035). Returns the
    /// most-recent rate on or before that date, or `None` when no usable rate
    /// exists (FXR-034). An identity pair (`from == to`) resolves to `1.0`
    /// (1_000_000 micros) without touching the repository. Read-only: never
    /// writes or publishes events.
    pub async fn resolve_rate_micros(
        &self,
        from_currency: &str,
        to_currency: &str,
        as_of: &str,
    ) -> StdResult<Option<i64>, CurrencyError> {
        if from_currency == to_currency {
            return Ok(Some(1_000_000));
        }
        let rate = self
            .rate_repo
            .latest_rate_on_or_before(from_currency, to_currency, as_of)
            .await
            .map_err(|e| {
                tracing::error!(target: BACKEND, err = ?e, "resolve_rate_micros: repository failure");
                CurrencyError::DatabaseError
            })?;
        Ok(rate.map(|r| r.rate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::currency::domain::{
        MockCurrencyPairRepository, MockCurrencyRateRepository,
    };

    fn make_service(
        pair_repo: MockCurrencyPairRepository,
        rate_repo: MockCurrencyRateRepository,
    ) -> CurrencyService {
        CurrencyService::new(Box::new(pair_repo), Box::new(rate_repo))
    }

    fn make_rate(
        from: &str,
        to: &str,
        date: &str,
        micros: i64,
        source: CurrencyRateSource,
    ) -> CurrencyRate {
        CurrencyRate::from_storage(
            from.to_string(),
            to.to_string(),
            date.to_string(),
            micros,
            source,
        )
    }

    fn make_pair(from: &str, to: &str) -> CurrencyPair {
        CurrencyPair::from_storage(from.to_string(), to.to_string())
    }

    // -------------------------------------------------------------------------
    // declare_currency_pair
    // -------------------------------------------------------------------------

    // FXR-054 — declare_currency_pair returns the pair on first declaration
    #[tokio::test]
    async fn declare_currency_pair_returns_pair_on_first_declaration() {
        let mut pair_repo = MockCurrencyPairRepository::new();
        pair_repo.expect_upsert_pair().returning(Ok);
        let rate_repo = MockCurrencyRateRepository::new();

        let svc = make_service(pair_repo, rate_repo);
        let result = svc
            .declare_currency_pair("USD".to_string(), "EUR".to_string())
            .await;

        let pair = result.expect("declare_currency_pair should succeed");
        assert_eq!(pair.from_currency, "USD");
        assert_eq!(pair.to_currency, "EUR");
    }

    // FXR-054 — declare_currency_pair is idempotent: calling again returns the existing pair
    #[tokio::test]
    async fn declare_currency_pair_is_idempotent() {
        let mut pair_repo = MockCurrencyPairRepository::new();
        // Simulates the upsert returning the existing pair on a second call.
        pair_repo
            .expect_upsert_pair()
            .times(2)
            .returning(|_| Ok(make_pair("USD", "EUR")));
        let rate_repo = MockCurrencyRateRepository::new();
        let svc = make_service(pair_repo, rate_repo);

        svc.declare_currency_pair("USD".to_string(), "EUR".to_string())
            .await
            .unwrap();
        let result = svc
            .declare_currency_pair("USD".to_string(), "EUR".to_string())
            .await;

        let pair = result.expect("second declare should also succeed");
        assert_eq!(pair.from_currency, "USD");
        assert_eq!(pair.to_currency, "EUR");
    }

    // FXR-023 — declare_currency_pair rejects an invalid currency code
    #[tokio::test]
    async fn declare_currency_pair_rejects_invalid_currency() {
        let pair_repo = MockCurrencyPairRepository::new();
        let rate_repo = MockCurrencyRateRepository::new();
        let svc = make_service(pair_repo, rate_repo);

        let err = svc
            .declare_currency_pair("XX".to_string(), "EUR".to_string())
            .await
            .unwrap_err();

        assert!(
            matches!(&err, CurrencyError::InvalidCurrency { currency } if currency == "XX"),
            "got: {err:?}"
        );
    }

    // FXR-011 — declare_currency_pair rejects an identity pair
    #[tokio::test]
    async fn declare_currency_pair_rejects_identity_pair() {
        let pair_repo = MockCurrencyPairRepository::new();
        let rate_repo = MockCurrencyRateRepository::new();
        let svc = make_service(pair_repo, rate_repo);

        let err = svc
            .declare_currency_pair("EUR".to_string(), "EUR".to_string())
            .await
            .unwrap_err();

        assert!(matches!(err, CurrencyError::IdentityPair), "got: {err:?}");
    }

    // DatabaseError — declare_currency_pair maps repo failure to DatabaseError
    #[tokio::test]
    async fn declare_currency_pair_maps_repo_failure_to_database_error() {
        let mut pair_repo = MockCurrencyPairRepository::new();
        pair_repo
            .expect_upsert_pair()
            .returning(|_| Err(anyhow::anyhow!("db exploded")));
        let rate_repo = MockCurrencyRateRepository::new();
        let svc = make_service(pair_repo, rate_repo);

        let err = svc
            .declare_currency_pair("USD".to_string(), "EUR".to_string())
            .await
            .unwrap_err();

        assert!(matches!(err, CurrencyError::DatabaseError), "got: {err:?}");
    }

    // -------------------------------------------------------------------------
    // record_currency_rate
    // -------------------------------------------------------------------------

    // FXR-025/101 — record_currency_rate upserts with source=Manual
    #[tokio::test]
    async fn record_currency_rate_upserts_with_source_manual() {
        let mut pair_repo = MockCurrencyPairRepository::new();
        pair_repo.expect_upsert_pair().returning(Ok);
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_upsert_rate().returning(Ok);

        let svc = make_service(pair_repo, rate_repo);
        let result = svc
            .record_currency_rate(
                "USD".to_string(),
                "EUR".to_string(),
                "2026-01-01".to_string(),
                920_000,
            )
            .await;

        let rate = result.expect("record_currency_rate should succeed");
        assert_eq!(rate.source, CurrencyRateSource::Manual);
        assert_eq!(rate.rate, 920_000);
    }

    // FXR-013 — record_currency_rate ensures the pair exists before writing the rate.
    // The mock's expect_upsert_pair() with times(1) assertion confirms the pair-ensure call occurs.
    #[tokio::test]
    async fn record_currency_rate_ensures_pair_exists_before_writing_rate() {
        let mut pair_repo = MockCurrencyPairRepository::new();
        pair_repo.expect_upsert_pair().times(1).returning(Ok);
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_upsert_rate().returning(Ok);

        let svc = make_service(pair_repo, rate_repo);
        svc.record_currency_rate(
            "USD".to_string(),
            "EUR".to_string(),
            "2026-01-01".to_string(),
            920_000,
        )
        .await
        .unwrap();
        // mockall validates the times(1) expectation on drop — test fails if
        // upsert_pair was not called exactly once.
    }

    // FXR-025 — record_currency_rate publishes CurrencyRateUpdated on success
    #[tokio::test]
    async fn record_currency_rate_publishes_currency_rate_updated_event() {
        let mut pair_repo = MockCurrencyPairRepository::new();
        pair_repo.expect_upsert_pair().returning(Ok);
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_upsert_rate().returning(Ok);

        let bus = Arc::new(SideEffectEventBus::new());
        let mut rx = bus.subscribe();
        let svc = make_service(pair_repo, rate_repo).with_event_bus(Arc::clone(&bus));

        svc.record_currency_rate(
            "USD".to_string(),
            "EUR".to_string(),
            "2026-01-01".to_string(),
            920_000,
        )
        .await
        .unwrap();

        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), Event::CurrencyRateUpdated);
    }

    // FXR-021 — record_currency_rate rejects a non-positive rate
    #[tokio::test]
    async fn record_currency_rate_rejects_non_positive_rate() {
        let pair_repo = MockCurrencyPairRepository::new();
        let rate_repo = MockCurrencyRateRepository::new();
        let svc = make_service(pair_repo, rate_repo);

        let err = svc
            .record_currency_rate(
                "USD".to_string(),
                "EUR".to_string(),
                "2026-01-01".to_string(),
                0,
            )
            .await
            .unwrap_err();

        assert!(matches!(err, CurrencyError::NotPositive), "got: {err:?}");
    }

    // FXR-022 — record_currency_rate rejects a future date
    #[tokio::test]
    async fn record_currency_rate_rejects_future_date() {
        let pair_repo = MockCurrencyPairRepository::new();
        let rate_repo = MockCurrencyRateRepository::new();
        let svc = make_service(pair_repo, rate_repo);

        let err = svc
            .record_currency_rate(
                "USD".to_string(),
                "EUR".to_string(),
                "2099-12-31".to_string(),
                920_000,
            )
            .await
            .unwrap_err();

        assert!(matches!(err, CurrencyError::DateInFuture), "got: {err:?}");
    }

    // FXR-022 — record_currency_rate rejects a malformed date
    #[tokio::test]
    async fn record_currency_rate_rejects_malformed_date() {
        let pair_repo = MockCurrencyPairRepository::new();
        let rate_repo = MockCurrencyRateRepository::new();
        let svc = make_service(pair_repo, rate_repo);

        let err = svc
            .record_currency_rate(
                "USD".to_string(),
                "EUR".to_string(),
                "not-a-date".to_string(),
                920_000,
            )
            .await
            .unwrap_err();

        assert!(
            matches!(&err, CurrencyError::InvalidDateFormat { date } if date == "not-a-date"),
            "got: {err:?}"
        );
    }

    // -------------------------------------------------------------------------
    // update_currency_rate
    // -------------------------------------------------------------------------

    // FXR-052 — update_currency_rate same-date: in-place overwrite succeeds
    #[tokio::test]
    async fn update_currency_rate_same_date_succeeds() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_get_by_key().returning(|_, _, _| {
            Ok(Some(make_rate(
                "USD",
                "EUR",
                "2026-01-01",
                920_000,
                CurrencyRateSource::Manual,
            )))
        });
        // delete_rate must NOT be called when the date is unchanged
        rate_repo.expect_upsert_rate().returning(Ok);

        let svc = make_service(pair_repo, rate_repo);
        let result = svc
            .update_currency_rate(
                "USD".to_string(),
                "EUR".to_string(),
                "2026-01-01".to_string(),
                "2026-01-01".to_string(),
                950_000,
            )
            .await;

        assert!(
            result.is_ok(),
            "same-date update should succeed: {result:?}"
        );
    }

    // FXR-052 — update_currency_rate changed-date: original record deleted, new one upserted
    #[tokio::test]
    async fn update_currency_rate_changed_date_deletes_original_and_upserts_new() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_get_by_key().returning(|_, _, _| {
            Ok(Some(make_rate(
                "USD",
                "EUR",
                "2026-01-01",
                920_000,
                CurrencyRateSource::Manual,
            )))
        });
        rate_repo
            .expect_delete_rate()
            .times(1)
            .returning(|_, _, _| Ok(()));
        rate_repo.expect_upsert_rate().times(1).returning(Ok);

        let svc = make_service(pair_repo, rate_repo);
        let result = svc
            .update_currency_rate(
                "USD".to_string(),
                "EUR".to_string(),
                "2026-01-01".to_string(),
                "2026-01-02".to_string(),
                950_000,
            )
            .await;

        assert!(
            result.is_ok(),
            "changed-date update should succeed: {result:?}"
        );
    }

    // FXR-052 — update_currency_rate returns RateNotFound when the original does not exist
    #[tokio::test]
    async fn update_currency_rate_returns_rate_not_found_for_missing_original() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_get_by_key().returning(|_, _, _| Ok(None));

        let svc = make_service(pair_repo, rate_repo);
        let err = svc
            .update_currency_rate(
                "USD".to_string(),
                "EUR".to_string(),
                "2026-01-01".to_string(),
                "2026-01-02".to_string(),
                950_000,
            )
            .await
            .unwrap_err();

        assert!(
            matches!(
                &err,
                CurrencyError::RateNotFound { from_currency, to_currency, date }
                    if from_currency == "USD" && to_currency == "EUR" && date == "2026-01-01"
            ),
            "got: {err:?}"
        );
    }

    // FXR-101 — update_currency_rate sets source=Manual regardless of original source
    #[tokio::test]
    async fn update_currency_rate_sets_source_to_manual() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_get_by_key().returning(|_, _, _| {
            Ok(Some(make_rate(
                "USD",
                "EUR",
                "2026-01-01",
                920_000,
                CurrencyRateSource::Frankfurter,
            )))
        });
        rate_repo.expect_upsert_rate().returning(|r| {
            // The upserted rate must carry source=Manual even though the original was Frankfurter.
            assert_eq!(r.source, CurrencyRateSource::Manual);
            Ok(r)
        });

        let svc = make_service(pair_repo, rate_repo);
        svc.update_currency_rate(
            "USD".to_string(),
            "EUR".to_string(),
            "2026-01-01".to_string(),
            "2026-01-01".to_string(),
            950_000,
        )
        .await
        .unwrap();
    }

    // FXR-052 — update_currency_rate publishes CurrencyRateUpdated on success
    #[tokio::test]
    async fn update_currency_rate_publishes_currency_rate_updated_event() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_get_by_key().returning(|_, _, _| {
            Ok(Some(make_rate(
                "USD",
                "EUR",
                "2026-01-01",
                920_000,
                CurrencyRateSource::Manual,
            )))
        });
        rate_repo.expect_upsert_rate().returning(Ok);

        let bus = Arc::new(SideEffectEventBus::new());
        let mut rx = bus.subscribe();
        let svc = make_service(pair_repo, rate_repo).with_event_bus(Arc::clone(&bus));

        svc.update_currency_rate(
            "USD".to_string(),
            "EUR".to_string(),
            "2026-01-01".to_string(),
            "2026-01-01".to_string(),
            950_000,
        )
        .await
        .unwrap();

        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), Event::CurrencyRateUpdated);
    }

    // -------------------------------------------------------------------------
    // delete_currency_rate
    // -------------------------------------------------------------------------

    // FXR-053 — delete_currency_rate succeeds when the rate exists
    #[tokio::test]
    async fn delete_currency_rate_succeeds_when_rate_exists() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_get_by_key().returning(|_, _, _| {
            Ok(Some(make_rate(
                "USD",
                "EUR",
                "2026-01-01",
                920_000,
                CurrencyRateSource::Manual,
            )))
        });
        rate_repo.expect_delete_rate().returning(|_, _, _| Ok(()));

        let svc = make_service(pair_repo, rate_repo);
        let result = svc
            .delete_currency_rate(
                "USD".to_string(),
                "EUR".to_string(),
                "2026-01-01".to_string(),
            )
            .await;

        assert!(result.is_ok(), "delete should succeed: {result:?}");
    }

    // FXR-053 — delete_currency_rate returns RateNotFound when absent
    #[tokio::test]
    async fn delete_currency_rate_returns_rate_not_found_when_absent() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_get_by_key().returning(|_, _, _| Ok(None));

        let svc = make_service(pair_repo, rate_repo);
        let err = svc
            .delete_currency_rate(
                "USD".to_string(),
                "EUR".to_string(),
                "2026-01-01".to_string(),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(
                &err,
                CurrencyError::RateNotFound { from_currency, to_currency, date }
                    if from_currency == "USD" && to_currency == "EUR" && date == "2026-01-01"
            ),
            "got: {err:?}"
        );
    }

    // FXR-053 — delete_currency_rate publishes CurrencyRateUpdated on success
    #[tokio::test]
    async fn delete_currency_rate_publishes_currency_rate_updated_event() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_get_by_key().returning(|_, _, _| {
            Ok(Some(make_rate(
                "USD",
                "EUR",
                "2026-01-01",
                920_000,
                CurrencyRateSource::Manual,
            )))
        });
        rate_repo.expect_delete_rate().returning(|_, _, _| Ok(()));

        let bus = Arc::new(SideEffectEventBus::new());
        let mut rx = bus.subscribe();
        let svc = make_service(pair_repo, rate_repo).with_event_bus(Arc::clone(&bus));

        svc.delete_currency_rate(
            "USD".to_string(),
            "EUR".to_string(),
            "2026-01-01".to_string(),
        )
        .await
        .unwrap();

        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), Event::CurrencyRateUpdated);
    }

    // -------------------------------------------------------------------------
    // list_currency_rates
    // -------------------------------------------------------------------------

    // FXR-050 — list_currency_rates returns rates ordered by date descending
    #[tokio::test]
    async fn list_currency_rates_returns_rates_ordered_by_date_descending() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_list_rates_for_pair().returning(|_, _| {
            Ok(vec![
                make_rate(
                    "USD",
                    "EUR",
                    "2026-01-03",
                    930_000,
                    CurrencyRateSource::Manual,
                ),
                make_rate(
                    "USD",
                    "EUR",
                    "2026-01-02",
                    920_000,
                    CurrencyRateSource::Manual,
                ),
                make_rate(
                    "USD",
                    "EUR",
                    "2026-01-01",
                    910_000,
                    CurrencyRateSource::Manual,
                ),
            ])
        });

        let svc = make_service(pair_repo, rate_repo);
        let rates = svc
            .list_currency_rates("USD".to_string(), "EUR".to_string())
            .await
            .unwrap();

        assert_eq!(rates.len(), 3);
        assert_eq!(rates[0].date, "2026-01-03");
        assert_eq!(rates[1].date, "2026-01-02");
        assert_eq!(rates[2].date, "2026-01-01");
    }

    // FXR-050 — list_currency_rates returns empty list for an unknown pair (never RateNotFound)
    #[tokio::test]
    async fn list_currency_rates_returns_empty_list_for_unknown_pair() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_list_rates_for_pair()
            .returning(|_, _| Ok(vec![]));

        let svc = make_service(pair_repo, rate_repo);
        let result = svc
            .list_currency_rates("USD".to_string(), "EUR".to_string())
            .await;

        let rates = result.expect("should return Ok(empty vec) for unknown pair");
        assert!(rates.is_empty());
    }

    // -------------------------------------------------------------------------
    // list_currency_pairs
    // -------------------------------------------------------------------------

    // FXR-051 — list_currency_pairs delegates to list_pairs_with_latest_rate
    #[tokio::test]
    async fn list_currency_pairs_returns_all_pairs_with_latest_rate() {
        let mut pair_repo = MockCurrencyPairRepository::new();
        pair_repo
            .expect_list_pairs_with_latest_rate()
            .returning(|| {
                Ok(vec![
                    CurrencyPairSummary {
                        from_currency: "USD".to_string(),
                        to_currency: "EUR".to_string(),
                        latest_rate: Some(920_000),
                        latest_rate_date: Some("2026-01-01".to_string()),
                        latest_rate_source: Some(CurrencyRateSource::Manual),
                    },
                    CurrencyPairSummary {
                        from_currency: "GBP".to_string(),
                        to_currency: "EUR".to_string(),
                        latest_rate: None,
                        latest_rate_date: None,
                        latest_rate_source: None,
                    },
                ])
            });
        let rate_repo = MockCurrencyRateRepository::new();

        let svc = make_service(pair_repo, rate_repo);
        let summaries = svc.list_currency_pairs().await.unwrap();

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].from_currency, "USD");
        assert_eq!(summaries[0].latest_rate, Some(920_000));
        assert!(summaries[1].latest_rate.is_none());
    }

    // -------------------------------------------------------------------------
    // Infrastructure-failure translation (every map_err → DatabaseError branch)
    // -------------------------------------------------------------------------

    fn db_err() -> anyhow::Error {
        anyhow::anyhow!("db exploded")
    }

    // record_currency_rate: pair upsert failure → DatabaseError
    #[tokio::test]
    async fn record_currency_rate_maps_pair_upsert_failure_to_database_error() {
        let mut pair_repo = MockCurrencyPairRepository::new();
        pair_repo.expect_upsert_pair().returning(|_| Err(db_err()));
        let rate_repo = MockCurrencyRateRepository::new();

        let err = make_service(pair_repo, rate_repo)
            .record_currency_rate("USD".into(), "EUR".into(), "2026-01-01".into(), 920_000)
            .await
            .unwrap_err();
        assert!(matches!(err, CurrencyError::DatabaseError), "got: {err:?}");
    }

    // record_currency_rate: rate upsert failure → DatabaseError
    #[tokio::test]
    async fn record_currency_rate_maps_rate_upsert_failure_to_database_error() {
        let mut pair_repo = MockCurrencyPairRepository::new();
        pair_repo.expect_upsert_pair().returning(Ok);
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_upsert_rate().returning(|_| Err(db_err()));

        let err = make_service(pair_repo, rate_repo)
            .record_currency_rate("USD".into(), "EUR".into(), "2026-01-01".into(), 920_000)
            .await
            .unwrap_err();
        assert!(matches!(err, CurrencyError::DatabaseError), "got: {err:?}");
    }

    // update_currency_rate: get_by_key failure → DatabaseError
    #[tokio::test]
    async fn update_currency_rate_maps_get_by_key_failure_to_database_error() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_get_by_key()
            .returning(|_, _, _| Err(db_err()));

        let err = make_service(pair_repo, rate_repo)
            .update_currency_rate(
                "USD".into(),
                "EUR".into(),
                "2026-01-01".into(),
                "2026-01-01".into(),
                950_000,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CurrencyError::DatabaseError), "got: {err:?}");
    }

    // update_currency_rate: upsert failure → DatabaseError (same-date path)
    #[tokio::test]
    async fn update_currency_rate_maps_upsert_failure_to_database_error() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_get_by_key().returning(|_, _, _| {
            Ok(Some(make_rate(
                "USD",
                "EUR",
                "2026-01-01",
                920_000,
                CurrencyRateSource::Manual,
            )))
        });
        rate_repo.expect_upsert_rate().returning(|_| Err(db_err()));

        let err = make_service(pair_repo, rate_repo)
            .update_currency_rate(
                "USD".into(),
                "EUR".into(),
                "2026-01-01".into(),
                "2026-01-01".into(),
                950_000,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CurrencyError::DatabaseError), "got: {err:?}");
    }

    // update_currency_rate: delete failure (changed-date path) → DatabaseError
    #[tokio::test]
    async fn update_currency_rate_maps_delete_failure_to_database_error() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_get_by_key().returning(|_, _, _| {
            Ok(Some(make_rate(
                "USD",
                "EUR",
                "2026-01-01",
                920_000,
                CurrencyRateSource::Manual,
            )))
        });
        rate_repo
            .expect_delete_rate()
            .returning(|_, _, _| Err(db_err()));

        let err = make_service(pair_repo, rate_repo)
            .update_currency_rate(
                "USD".into(),
                "EUR".into(),
                "2026-01-01".into(),
                "2026-01-02".into(),
                950_000,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CurrencyError::DatabaseError), "got: {err:?}");
    }

    // delete_currency_rate: get_by_key failure → DatabaseError
    #[tokio::test]
    async fn delete_currency_rate_maps_get_by_key_failure_to_database_error() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_get_by_key()
            .returning(|_, _, _| Err(db_err()));

        let err = make_service(pair_repo, rate_repo)
            .delete_currency_rate("USD".into(), "EUR".into(), "2026-01-01".into())
            .await
            .unwrap_err();
        assert!(matches!(err, CurrencyError::DatabaseError), "got: {err:?}");
    }

    // delete_currency_rate: delete failure → DatabaseError
    #[tokio::test]
    async fn delete_currency_rate_maps_delete_failure_to_database_error() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo.expect_get_by_key().returning(|_, _, _| {
            Ok(Some(make_rate(
                "USD",
                "EUR",
                "2026-01-01",
                920_000,
                CurrencyRateSource::Manual,
            )))
        });
        rate_repo
            .expect_delete_rate()
            .returning(|_, _, _| Err(db_err()));

        let err = make_service(pair_repo, rate_repo)
            .delete_currency_rate("USD".into(), "EUR".into(), "2026-01-01".into())
            .await
            .unwrap_err();
        assert!(matches!(err, CurrencyError::DatabaseError), "got: {err:?}");
    }

    // list_currency_pairs: repo failure → DatabaseError
    #[tokio::test]
    async fn list_currency_pairs_maps_repo_failure_to_database_error() {
        let mut pair_repo = MockCurrencyPairRepository::new();
        pair_repo
            .expect_list_pairs_with_latest_rate()
            .returning(|| Err(db_err()));
        let rate_repo = MockCurrencyRateRepository::new();

        let err = make_service(pair_repo, rate_repo)
            .list_currency_pairs()
            .await
            .unwrap_err();
        assert!(matches!(err, CurrencyError::DatabaseError), "got: {err:?}");
    }

    // list_currency_rates: repo failure → DatabaseError
    #[tokio::test]
    async fn list_currency_rates_maps_repo_failure_to_database_error() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_list_rates_for_pair()
            .returning(|_, _| Err(db_err()));

        let err = make_service(pair_repo, rate_repo)
            .list_currency_rates("USD".into(), "EUR".into())
            .await
            .unwrap_err();
        assert!(matches!(err, CurrencyError::DatabaseError), "got: {err:?}");
    }

    // -------------------------------------------------------------------------
    // resolve_rate_micros — FXR-011 / FXR-034 / FXR-035
    // -------------------------------------------------------------------------

    // FXR-011 — identity pair (from == to) returns Ok(Some(1_000_000)) without
    // consulting the repository (0 calls to latest_rate_on_or_before)
    #[tokio::test]
    async fn resolve_rate_micros_identity_pair_returns_one_without_repo_call() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        // The mock will fail the test if latest_rate_on_or_before is called at all.
        rate_repo
            .expect_latest_rate_on_or_before()
            .times(0)
            .returning(|_, _, _| Ok(None));

        let svc = make_service(pair_repo, rate_repo);
        let result = svc.resolve_rate_micros("EUR", "EUR", "2026-06-01").await;

        assert_eq!(
            result.expect("identity resolve should succeed"),
            Some(1_000_000)
        );
    }

    // FXR-035 — rate found: returns Ok(Some(rate.rate))
    #[tokio::test]
    async fn resolve_rate_micros_returns_rate_when_found() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_latest_rate_on_or_before()
            .returning(|_, _, _| {
                Ok(Some(make_rate(
                    "USD",
                    "EUR",
                    "2026-05-30",
                    1_080_000,
                    CurrencyRateSource::Manual,
                )))
            });

        let svc = make_service(pair_repo, rate_repo);
        let result = svc.resolve_rate_micros("USD", "EUR", "2026-06-01").await;

        assert_eq!(result.expect("resolve should succeed"), Some(1_080_000));
    }

    // FXR-034 — repo returns Ok(None): resolve_rate_micros returns Ok(None)
    #[tokio::test]
    async fn resolve_rate_micros_returns_none_when_no_rate_found() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_latest_rate_on_or_before()
            .returning(|_, _, _| Ok(None));

        let svc = make_service(pair_repo, rate_repo);
        let result = svc.resolve_rate_micros("USD", "EUR", "2026-06-01").await;

        assert_eq!(result.expect("resolve should succeed"), None);
    }

    // repo returns Err → mapped to Err(CurrencyError::DatabaseError)
    #[tokio::test]
    async fn resolve_rate_micros_maps_repo_failure_to_database_error() {
        let pair_repo = MockCurrencyPairRepository::new();
        let mut rate_repo = MockCurrencyRateRepository::new();
        rate_repo
            .expect_latest_rate_on_or_before()
            .returning(|_, _, _| Err(db_err()));

        let svc = make_service(pair_repo, rate_repo);
        let err = svc
            .resolve_rate_micros("USD", "EUR", "2026-06-01")
            .await
            .unwrap_err();

        assert!(matches!(err, CurrencyError::DatabaseError), "got: {err:?}");
    }
}
