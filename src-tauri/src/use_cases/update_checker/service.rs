//! Update checker service — detects, downloads, and installs application updates.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

use crate::core::BACKEND;

/// Information about an available application update.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct UpdateInfo {
    /// Semantic version string of the available update (e.g. "1.2.3").
    pub version: String,
}

/// Shared state for the update lifecycle, managed across Tauri commands.
///
/// Tracks whether a download is in progress (R10) and stores downloaded bytes
/// between the download command and the install command.
#[derive(Debug, Default)]
pub struct UpdateState {
    /// True while a download is in progress — prevents concurrent downloads (R10).
    pub is_downloading: AtomicBool,
    /// Downloaded installer bytes stored after a successful download (R9).
    downloaded_bytes: Mutex<Option<Vec<u8>>>,
}

impl UpdateState {
    /// Creates a new, empty update state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores downloaded bytes for later installation.
    pub fn set_bytes(&self, bytes: Vec<u8>) {
        if let Ok(mut guard) = self.downloaded_bytes.lock() {
            *guard = Some(bytes);
        }
    }

    /// Takes the downloaded bytes, clearing the stored value.
    pub fn take_bytes(&self) -> Option<Vec<u8>> {
        self.downloaded_bytes.lock().ok()?.take()
    }
}

/// Checks whether a new application version is available.
///
/// Returns `None` silently on network or server errors (R21), logging them for
/// diagnostics (R22). Emits `"update:available"` on the app handle if an update
/// is found, so that all listeners (banner, manual check) react consistently.
pub async fn check(app_handle: &AppHandle) -> anyhow::Result<Option<UpdateInfo>> {
    let updater = match app_handle.updater() {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(target: BACKEND, error = %e, "Failed to initialize updater (R22)");
            return Ok(None);
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            tracing::info!(target: BACKEND, version = %version, "Update available");
            let info = UpdateInfo { version };
            let _ = app_handle.emit("update:available", &info);
            Ok(Some(info))
        }
        Ok(None) => {
            tracing::info!(target: BACKEND, "Application is up to date");
            Ok(None)
        }
        Err(e) => {
            tracing::warn!(target: BACKEND, error = %e, "Update check failed — silent (R21, R22)");
            Ok(None)
        }
    }
}

/// Downloads the available update in the background, emitting progress events (R8).
///
/// Does nothing if a download is already in progress (R10).
/// Emits `"update:progress"` (percent 0–100) during download,
/// `"update:complete"` on success, or `"update:error"` on failure (R23).
/// Checksum verification is performed by the Tauri updater plugin (R9).
pub async fn download(app_handle: AppHandle, state: Arc<UpdateState>) -> anyhow::Result<()> {
    // R10 — prevent concurrent downloads
    if state
        .is_downloading
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        tracing::warn!(target: BACKEND, "Download already in progress — ignoring (R10)");
        return Ok(());
    }

    let result = do_download(&app_handle, &state).await;
    state.is_downloading.store(false, Ordering::SeqCst);

    if let Err(ref e) = result {
        tracing::error!(target: BACKEND, error = %e, "Update download failed (R23)");
        let _ = app_handle.emit("update:error", e.to_string());
    }

    result
}

async fn do_download(app_handle: &AppHandle, state: &UpdateState) -> anyhow::Result<()> {
    use anyhow::Context;

    let updater = app_handle
        .updater()
        .context("Failed to initialize updater")?;

    let update = updater
        .check()
        .await
        .context("Failed to check for update during download")?
        .ok_or_else(|| anyhow::anyhow!("No update available to download"))?;

    let downloaded = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let ah = app_handle.clone();

    // R8 — emit progress events; checksum is verified by the plugin (R9)
    let bytes = update
        .download(
            move |chunk, total| {
                let current = downloaded.fetch_add(chunk as u64, Ordering::Relaxed) + chunk as u64;
                let percent = total
                    .map(|t| {
                        if t > 0 {
                            (current * 100 / t).min(100)
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0);
                let _ = ah.emit("update:progress", percent);
            },
            || {},
        )
        .await
        .context("Download or checksum verification failed (R9, R23)")?;

    // Store bytes BEFORE emitting complete — prevents install racing (R11)
    state.set_bytes(bytes);
    let _ = app_handle.emit("update:complete", ());
    tracing::info!(target: BACKEND, "Update downloaded and checksum verified (R9)");
    Ok(())
}

/// Installs the previously downloaded update and restarts the application (R13).
///
/// Re-checks for the update to obtain a fresh handle for the install call.
/// Requires that `download` has been called successfully beforehand.
pub async fn install(app_handle: AppHandle, state: Arc<UpdateState>) -> anyhow::Result<()> {
    use anyhow::Context;

    let bytes = state
        .take_bytes()
        .ok_or_else(|| anyhow::anyhow!("No downloaded update available — call download first"))?;

    let updater = app_handle
        .updater()
        .context("Failed to initialize updater for install")?;

    let update = updater
        .check()
        .await
        .context("Failed to get update for installation")?
        .ok_or_else(|| anyhow::anyhow!("No update found for installation"))?;

    update.install(bytes).context("Installation failed")?;
    tracing::info!(target: BACKEND, "Update installed — restarting application (R13)");
    app_handle.restart();
}
