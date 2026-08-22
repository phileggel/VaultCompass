//! Headless entry point for the OS-triggered scheduled run (SPF-016, SPF-020).
//! Invoked by `main.rs` when the process is launched with `--scheduled-fetch`
//! (no Tauri window is ever created for this path).

use std::path::PathBuf;
use std::sync::Arc;

use crate::context::asset::{PriceProvider, ReqwestYahooClient};
use crate::context::currency::{RateHistoryProvider, ReqwestFrankfurterClient};
use crate::context::sync::SqliteChangeRecorder;
use crate::core::{Database, BACKEND};
use crate::shared::infrastructure::change_recorder::ChangeRecorder;
use crate::shared::infrastructure::container::AppContainer;
use crate::shared::infrastructure::scheduler::platform_scheduler;

use super::orchestrator::ScheduledFetchOrchestrator;
use super::repository::SqliteScheduledFetchRepository;

/// The application identifier Tauri derives its per-app directories from
/// (`tauri.conf.json` → `identifier`). The headless path has no Tauri handle,
/// so it reproduces `app_local_data_dir()` = platform data-local dir + this
/// identifier; a mismatch would silently split the two entries onto two
/// databases (guarded by a test below).
const APP_IDENTIFIER: &str = "com.phileggel.vault-compass";

/// Reproduces Tauri's `app_local_data_dir()` without an app handle.
pub fn resolve_app_local_data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join(APP_IDENTIFIER))
}

/// Reproduces Tauri's `app_log_dir()` without an app handle, so the headless
/// run logs into the same file the interactive app uses (macOS puts logs
/// under `~/Library/Logs`, everywhere else they sit next to the app data).
fn resolve_app_log_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|home| home.join("Library/Logs").join(APP_IDENTIFIER))
    }
    #[cfg(not(target_os = "macos"))]
    {
        resolve_app_local_data_dir().map(|data_dir| data_dir.join("logs"))
    }
}

/// Resolves the app's data directory, opens the database, wires the minimal
/// service graph, runs [`ScheduledFetchOrchestrator::run_scheduled_fetch`],
/// and returns a process exit code — `0` unless the run record itself could
/// not be written.
pub async fn run() -> i32 {
    // A logging failure must not abandon the fetch — the subscriber is
    // best-effort; its absence falls back to the eprintln below only.
    match resolve_app_log_dir() {
        Some(log_dir) => {
            if let Err(error) = std::fs::create_dir_all(&log_dir)
                .map_err(anyhow::Error::from)
                .and_then(|()| crate::initialize_tracing(&log_dir))
            {
                eprintln!("scheduled fetch: tracing initialization failed: {error:#}");
            }
        }
        None => eprintln!("scheduled fetch: no platform log directory available"),
    }

    let Some(data_dir) = resolve_app_local_data_dir() else {
        tracing::error!(target: BACKEND, "scheduled fetch: no platform data directory available");
        return 1;
    };
    let database = match Database::new(data_dir).await {
        Ok(database) => database,
        Err(error) => {
            tracing::error!(target: BACKEND, err = %format!("{error:#}"), "scheduled fetch: database initialization failed");
            return 1;
        }
    };
    let pool = database.pool;

    let frankfurter_client = match ReqwestFrankfurterClient::new() {
        Ok(client) => Arc::new(client),
        Err(error) => {
            tracing::error!(target: BACKEND, err = %format!("{error:#}"), "scheduled fetch: HTTP client initialization failed");
            return 1;
        }
    };
    let price_provider = match ReqwestYahooClient::new() {
        Ok(client) => Arc::new(client),
        Err(error) => {
            tracing::error!(target: BACKEND, err = %format!("{error:#}"), "scheduled fetch: HTTP client initialization failed");
            return 1;
        }
    };

    // No event bus: the headless run must never publish side-effect events
    // (SPF-024 — there is no window to forward them to).
    let container = AppContainer::build(
        pool.clone(),
        price_provider as Arc<dyn PriceProvider>,
        None,
        Some(frankfurter_client as Arc<dyn RateHistoryProvider>),
        None,
        Arc::new(SqliteChangeRecorder::new(pool.clone())) as Arc<dyn ChangeRecorder>,
    );
    let repository = Arc::new(SqliteScheduledFetchRepository::new(pool));

    let orchestrator = ScheduledFetchOrchestrator::new(
        container.account_service,
        container.asset_service,
        container.price_provider,
        container.currency_service,
        repository,
        platform_scheduler(),
        Arc::new(|| chrono::Local::now().naive_local()),
    );

    match orchestrator.run_scheduled_fetch().await {
        Ok(run) => {
            tracing::info!(
                target: BACKEND,
                outcome = %run.outcome,
                updated = run.updated_count,
                skipped = run.skipped_count,
                trigger_date = %run.trigger_date,
                "scheduled fetch run completed"
            );
            0
        }
        Err(error) => {
            tracing::error!(target: BACKEND, err = %error, "scheduled fetch: run failed");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The identifier must match tauri.conf.json — a drift would split the
    // headless run and the app onto two databases.
    #[test]
    fn app_identifier_matches_tauri_conf() {
        let conf = include_str!("../../../tauri.conf.json");
        assert!(
            conf.contains(&format!("\"identifier\": \"{APP_IDENTIFIER}\"")),
            "APP_IDENTIFIER must match tauri.conf.json's identifier"
        );
    }
}
