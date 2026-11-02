//! Update installation must never race an active encode or close confirmation.
use crate::{ActiveProcess, state::BatchManager};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub(super) fn updater_is_configured(app: AppHandle) -> bool {
    !cfg!(debug_assertions)
        && app
            .config()
            .plugins
            .0
            .get("updater")
            .and_then(|config| config.get("pubkey"))
            .and_then(|key| key.as_str())
            .is_some_and(|key| !key.trim().is_empty() && key != "YOUR_PUBLIC_KEY_HERE")
}

#[tauri::command]
pub(super) async fn prepare_update_install(
    app: AppHandle,
    manager: State<'_, BatchManager>,
) -> Result<bool, String> {
    if !updater_is_configured(app.clone()) || !manager.begin_update()? {
        return Ok(false);
    }
    app.state::<Arc<ActiveProcess>>().wait_until_idle().await;
    Ok(true)
}

#[tauri::command]
pub(super) fn release_update_install(manager: State<'_, BatchManager>) -> Result<(), String> {
    manager.end_update()
}
