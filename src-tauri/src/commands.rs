//! Narrow Tauri IPC adapters; validate job identity before process controls.
use crate::{
    ActiveProcess,
    execution::{emit_progress, remove_preview, resolve_tools, start_task},
    state::*,
};
use keroro_encoder_core::{Cancellation, Progress, Settings, TaskKind};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tokio::io::AsyncReadExt;

#[tauri::command]
pub(super) async fn probe_media_streams(
    app: AppHandle,
    path: String,
) -> Result<keroro_encoder_core::probe::MediaInfo, String> {
    static PROBES: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
    let _permit = PROBES.acquire().await.map_err(|e| e.to_string())?;
    let tools = resolve_tools(&app).await?;
    // Metadata probes own separate tracked children; they never replace FFmpeg's slot.
    let processes = Arc::new(ActiveProcess::default());
    keroro_encoder_core::probe::media_streams(
        &tools,
        std::path::Path::new(&path),
        &Cancellation::new(),
        &processes,
    )
    .await
}

pub(super) fn change_pause(
    window: WebviewWindow,
    manager: &BatchManager,
    processes: &ActiveProcess,
    job_id: &str,
    paused: bool,
) -> Result<(), String> {
    let progress = manager.control_pause(job_id, paused, || processes.set_paused(paused))?;
    emit_progress(&window, &progress);
    Ok(())
}

#[tauri::command]
pub(super) async fn get_performance_report(
    app: AppHandle,
) -> Result<Option<keroro_encoder_core::EncodingMeasurements>, String> {
    let store = app
        .state::<Arc<keroro_encoder_core::session::SessionStore>>()
        .inner()
        .clone();
    match keroro_encoder_core::session::blocking(move || store.last_measurements()).await {
        Ok(report) => Ok(report),
        Err(error) => {
            log::warn!("Optional performance report unavailable: {error}");
            Ok(None)
        }
    }
}

#[tauri::command]
pub(super) async fn pause_encode(
    window: WebviewWindow,
    manager: State<'_, BatchManager>,
    processes: State<'_, Arc<ActiveProcess>>,
    job_id: String,
) -> Result<(), String> {
    change_pause(window, manager.inner(), processes.inner(), &job_id, true)
}

#[tauri::command]
pub(super) async fn resume_encode(
    window: WebviewWindow,
    manager: State<'_, BatchManager>,
    processes: State<'_, Arc<ActiveProcess>>,
    job_id: String,
) -> Result<(), String> {
    change_pause(window, manager.inner(), processes.inner(), &job_id, false)
}

#[tauri::command]
pub(super) async fn analyze_dropped_paths(paths: Vec<String>) -> Result<serde_json::Value, String> {
    keroro_encoder_core::session::blocking(move || keroro_encoder_core::drop_scan::analyze(paths))
        .await
}

#[tauri::command]
pub(super) async fn get_recovery(
    app: AppHandle,
    manager: State<'_, BatchManager>,
) -> Result<Option<keroro_encoder_core::session::RecoveryInfo>, String> {
    let unavailable = {
        let state = manager.inner().0.lock().map_err(|e| e.to_string())?;
        state.active.is_some()
    };
    if unavailable {
        return Ok(None);
    }
    let store = app
        .state::<Arc<keroro_encoder_core::session::SessionStore>>()
        .inner()
        .clone();
    keroro_encoder_core::session::blocking(move || store.info()).await
}
#[tauri::command]
pub(super) async fn discard_recovery(
    app: AppHandle,
    manager: State<'_, BatchManager>,
    recovery_id: String,
) -> Result<(), String> {
    let reservation = manager.reserve("discard-recovery".into(), Cancellation::new())?;
    let store = app
        .state::<Arc<keroro_encoder_core::session::SessionStore>>()
        .inner()
        .clone();
    let result = keroro_encoder_core::session::blocking(move || store.discard(&recovery_id)).await;
    drop(reservation);
    result
}
#[tauri::command]
pub(super) async fn resume_batch(
    app: AppHandle,
    window: WebviewWindow,
    manager: State<'_, BatchManager>,
    recovery_id: String,
) -> Result<String, String> {
    let store = app
        .state::<Arc<keroro_encoder_core::session::SessionStore>>()
        .inner()
        .clone();
    let info = keroro_encoder_core::session::blocking(move || store.info())
        .await?
        .ok_or("No interrupted batch exists.")?;
    let settings = info
        .settings
        .ok_or("Recovery checkpoint cannot be resumed; review its error.")?;
    start_task(
        app,
        window,
        manager.inner().clone(),
        settings,
        TaskKind::Batch,
        Some(recovery_id),
    )
    .await
}

#[tauri::command]
pub(super) async fn check_environment(app: AppHandle) -> Result<(), String> {
    resolve_tools(&app).await.map(|_| ())
}

#[tauri::command]
pub(super) async fn start_batch(
    app: AppHandle,
    window: WebviewWindow,
    manager: State<'_, BatchManager>,
    settings: Settings,
) -> Result<String, String> {
    start_task(
        app,
        window,
        manager.inner().clone(),
        settings,
        TaskKind::Batch,
        None,
    )
    .await
}
#[tauri::command]
pub(super) async fn start_preview(
    app: AppHandle,
    window: WebviewWindow,
    manager: State<'_, BatchManager>,
    settings: Settings,
) -> Result<String, String> {
    start_task(
        app,
        window,
        manager.inner().clone(),
        settings,
        TaskKind::Preview,
        None,
    )
    .await
}
#[tauri::command]
pub(super) fn get_batch_state(
    manager: State<'_, BatchManager>,
) -> Result<Option<Progress>, String> {
    manager
        .inner()
        .0
        .lock()
        .map(|state| state.progress.clone())
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(super) async fn read_preview(
    job_id: String,
    manager: State<'_, BatchManager>,
) -> Result<tauri::ipc::Response, String> {
    let file = manager
        .inner()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .preview
        .clone()
        .filter(|file| file.job_id == job_id)
        .ok_or("This preview is no longer available.")?;
    const LIMIT: u64 = 32 * 1024 * 1024;
    let reader = tokio::fs::File::open(&file.path)
        .await
        .map_err(|e| e.to_string())?;
    if reader.metadata().await.map_err(|e| e.to_string())?.len() > LIMIT {
        return Err("The preview exceeds the 32 MiB playback limit.".into());
    }
    let mut bytes = Vec::new();
    reader
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("The preview exceeds the playback limit.".into());
    }
    Ok(tauri::ipc::Response::new(bytes))
}
#[tauri::command]
pub(super) async fn release_preview(
    job_id: String,
    manager: State<'_, BatchManager>,
) -> Result<(), String> {
    let file = {
        let mut state = manager.inner().0.lock().map_err(|e| e.to_string())?;
        if state
            .preview
            .as_ref()
            .is_some_and(|file| file.job_id == job_id)
        {
            let file = state.preview.take();
            if state.progress.as_ref().is_some_and(|progress| {
                progress.kind == TaskKind::Preview && progress.job_id == job_id
            }) {
                state.progress = None;
            }
            file
        } else {
            None
        }
    };
    if let Some(file) = file {
        remove_preview(file).await?;
    }
    Ok(())
}
#[tauri::command]
pub(super) async fn set_close_preferences(
    app: AppHandle,
    manager: State<'_, BatchManager>,
    minimize_on_close: bool,
) -> Result<(), String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("window_preferences.json");
    let preferences = ClosePreferences { minimize_on_close };
    keroro_encoder_core::session::blocking(move || {
        keroro_encoder_core::atomic_file::json(&path, &preferences)
    })
    .await?;
    manager
        .inner()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .minimize_on_close = minimize_on_close;
    Ok(())
}

#[tauri::command]
pub(super) async fn get_available_encoders(app: AppHandle) -> Result<Vec<String>, String> {
    crate::gpu_probe::detect_gpu_encoders(&app).await
}
#[tauri::command]
pub(super) async fn get_connected_drives()
-> Result<Vec<keroro_encoder_core::storage::DriveInfo>, String> {
    crate::drive_target::connected().await
}
#[tauri::command]
pub(super) async fn inspect_output_target(
    path: String,
) -> Result<keroro_encoder_core::storage::DriveInfo, String> {
    crate::drive_target::inspect(path).await
}
#[tauri::command]
pub(super) async fn prepare_wii_drive(
    drive_id: String,
    manager: State<'_, BatchManager>,
) -> Result<String, String> {
    let _reservation = manager.reserve("drive-target".into(), Cancellation::new())?;
    crate::drive_target::prepare(drive_id).await
}
