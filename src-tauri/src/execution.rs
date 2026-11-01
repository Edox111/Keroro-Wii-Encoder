//! Async media orchestration and progress delivery; encoding lives in encoder-core.
use crate::{ActiveProcess, state::*};
use keroro_encoder_core::{
    BatchPlan, Cancellation, ItemFailure, Phase, Progress, Settings, TaskKind, Tools,
    run_batch_with_session,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

pub(super) fn publish(manager: &BatchManager, window: &WebviewWindow, mut progress: Progress) {
    let tooltip = match manager.0.lock() {
        Ok(mut state) => {
            if !state
                .active
                .as_ref()
                .is_some_and(|active| active.job_id == progress.job_id)
            {
                return;
            }
            progress.paused = !progress.phase.is_terminal()
                && window
                    .app_handle()
                    .state::<Arc<ActiveProcess>>()
                    .is_paused();
            if progress.paused && state.progress.as_ref().is_some_and(|p| p.paused) {
                return;
            }
            progress.sequence = state
                .progress
                .as_ref()
                .filter(|p| p.job_id == progress.job_id)
                .map_or(0, |p| p.sequence + 1);
            if progress.paused {
                progress.message = "Encoding paused. Select Resume to continue.".into();
            }
            state.progress = Some(progress.clone());
            if progress.phase.is_terminal() {
                state.active = None;
            }
            let next = if !progress.phase.is_terminal() && progress.kind == TaskKind::Batch {
                Some(if progress.batch_percent.is_finite() {
                    progress.batch_percent.clamp(0.0, 100.0).floor() as u8
                } else {
                    0
                })
            } else {
                None
            };
            // Reuse the core's progress publication; never refresh per stderr line or timer.
            // Lifecycle/warning events with unchanged integer progress do no OS work.
            if next == state.tray_percent {
                None
            } else {
                state.tray_percent = next;
                Some(next.map_or_else(
                    || "Keroro Wii Encoder".into(),
                    |percent| format!("Keroro Wii Encoder - {percent}%"),
                ))
            }
        }
        Err(error) => {
            log::error!("Cannot update task state: {error}");
            return;
        }
    };
    if let Some(tooltip) = tooltip {
        if let Some(tray) = window.app_handle().tray_by_id("keroro-tray") {
            if let Err(error) = tray.set_tooltip(Some(tooltip)) {
                log::warn!("Cannot update tray tooltip: {error}");
            }
        }
    }
    emit_progress(window, &progress);
}

// Emission never writes state: delayed pause/resume delivery cannot replace a
// newer worker snapshot. Frontend sequence checks discard out-of-order events.
pub(super) fn emit_progress(window: &WebviewWindow, progress: &Progress) {
    if let Err(error) = window.emit("wii-progress", &progress) {
        log::error!("Cannot deliver progress: {error}");
    }
    if progress.kind == TaskKind::Batch {
        let payload = serde_json::json!({
            "file": format!("Ep{:02}", progress.current_file.max(1)),
            "percentage": progress.file_percent,
            "eta": keroro_encoder_core::progress::format_eta(progress.eta_seconds),
            "jobId": progress.job_id,
            "sequence": progress.sequence,
            "phase": progress.phase,
            "paused": progress.paused,
        });
        if let Err(error) = window.emit("encoding-progress", payload) {
            log::warn!("Cannot deliver ETA progress: {error}");
        }
    }
}

pub(super) async fn resolve_tools(app: &AppHandle) -> Result<Tools, String> {
    // On Windows NSIS installs externalBin entries alongside the main executable.
    // Retain owned Tokio children for pause/resume, cancellation, and reaping.
    let directory = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries")
    } else {
        app.path().resource_dir().map_err(|e| {
            format!("ENVIRONMENT_UNAVAILABLE: Cannot resolve bundled tools directory: {e}")
        })?
    };
    let suffix = if cfg!(debug_assertions) {
        "-x86_64-pc-windows-msvc.exe"
    } else {
        ".exe"
    };
    async fn locate(
        directory: &std::path::Path,
        name: &str,
        suffix: &str,
    ) -> Result<PathBuf, String> {
        let path = directory.join(format!("{name}{suffix}"));
        keroro_encoder_core::environment::validate_binary(&path, name).await?;
        Ok(path)
    }
    Ok(Tools {
        ffmpeg: locate(&directory, "ffmpeg", suffix).await?,
        ffprobe: locate(&directory, "ffprobe", suffix).await?,
    })
}

pub(super) async fn remove_preview(file: PreviewFile) -> Result<(), String> {
    match tokio::fs::remove_file(&file.path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Cannot remove temporary preview: {error}")),
    }
    let _ = keroro_encoder_core::artifacts::forget(&file.path).await;
    if let Some(parent) = file.path.parent() {
        match tokio::fs::remove_dir(parent).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("Cannot remove preview directory: {error}")),
        }
    }
    Ok(())
}

pub(super) async fn cleanup_saved_preview(manager: &BatchManager) {
    let file = match manager.0.lock() {
        Ok(mut state) => state.preview.take(),
        Err(error) => {
            log::error!("Cannot access preview cleanup state: {error}");
            None
        }
    };
    if let Some(file) = file {
        if let Err(error) = remove_preview(file).await {
            log::error!("{error}");
        }
    }
}

pub(super) async fn start_task(
    app: AppHandle,
    window: WebviewWindow,
    manager: BatchManager,
    settings: Settings,
    kind: TaskKind,
    resume_id: Option<String>,
) -> Result<String, String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let job_id = format!("{timestamp}-{}", COUNTER.fetch_add(1, Ordering::Relaxed));
    let cancellation = Cancellation::new();
    let reservation = manager.reserve(job_id.clone(), cancellation.clone())?;
    let wake_lock = keroro_encoder_core::power::WakeLock::acquire()?;
    let tools = resolve_tools(&app).await?;
    let sessions = app
        .state::<Arc<keroro_encoder_core::session::SessionStore>>()
        .inner()
        .clone();
    let is_resume = resume_id.is_some();
    let plan = if let Some(resume_id) = resume_id {
        let store = sessions.clone();
        let id = job_id.clone();
        let stop = cancellation.clone();
        let plan = keroro_encoder_core::session::blocking(move || {
            store.resume_checked(&resume_id, &id, &|| stop.is_cancelled())
        })
        .await?;
        keroro_encoder_core::preflight::output(std::path::Path::new(&plan.settings().output_dir))
            .await?;
        plan
    } else if kind == TaskKind::Preview {
        let old = manager.0.lock().map_err(|e| e.to_string())?.preview.take();
        if let Some(old) = old {
            remove_preview(old).await?;
        }
        let directory = app
            .path()
            .app_cache_dir()
            .map_err(|e| e.to_string())?
            .join("previews")
            .join(&job_id);
        match BatchPlan::prepare_preview(job_id.clone(), settings, &directory).await {
            Ok(plan) => plan,
            Err(error) => {
                let _ = tokio::fs::remove_dir(&directory).await;
                return Err(error);
            }
        }
    } else {
        BatchPlan::prepare(job_id.clone(), settings).await?
    };
    if kind == TaskKind::Batch && !is_resume {
        let store = sessions.clone();
        let saved_plan = plan.clone();
        keroro_encoder_core::session::blocking(move || store.begin(&saved_plan)).await?;
    }
    let checkpoint = (kind == TaskKind::Batch).then_some(sessions);
    let preview_path = plan.preview_path();
    let initial = plan.initial_progress();
    let processes = app.state::<Arc<ActiveProcess>>().inner().clone();
    publish(&manager, &window, plan.initial_progress());
    tauri::async_runtime::spawn(async move {
        let _reservation = reservation;
        let wake_lock = wake_lock;
        // The terminal event is held until preview bookkeeping and cleanup finish.
        let worker_manager = manager.clone();
        let worker_window = window.clone();
        let worker_processes = processes.clone();
        // JoinError catches an unwinding panic instead of leaving a stale running snapshot.
        let worker = tokio::spawn(async move {
            run_batch_with_session(
                plan,
                tools,
                cancellation,
                worker_processes,
                checkpoint,
                |progress| {
                    if !progress.phase.is_terminal() {
                        publish(&worker_manager, &worker_window, progress);
                    }
                },
            )
            .await
        });
        let outcome = worker.await;
        processes.wait_until_idle().await;
        drop(wake_lock);
        let mut result = match outcome {
            Ok(result) => result,
            Err(error) => {
                log::error!("Media worker stopped unexpectedly: {error}");
                let mut progress = manager
                    .0
                    .lock()
                    .ok()
                    .and_then(|state| state.progress.clone())
                    .filter(|progress| progress.job_id == initial.job_id)
                    .unwrap_or(initial);
                progress.sequence += 1;
                progress.phase = Phase::Failed;
                progress.file_percent = None;
                progress.eta_seconds = None;
                progress.message = "Encoding worker stopped unexpectedly. Partial files may need removal; retry the task.".into();
                if progress.succeeded + progress.skipped + progress.failed < progress.total_files {
                    progress.failed += 1;
                }
                progress.errors.push(ItemFailure {
                    file_name: progress
                        .file_name
                        .clone()
                        .unwrap_or_else(|| "Background task".into()),
                    reason: "The media worker failed unexpectedly; its child process was stopped."
                        .into(),
                });
                progress
            }
        };
        if let Some(path) = preview_path {
            let file = PreviewFile {
                job_id: result.job_id.clone(),
                path,
            };
            if result.phase == Phase::Succeeded {
                if let Err(error) = keroro_encoder_core::artifacts::record(&file.path).await {
                    log::warn!("Cannot journal preview: {error}");
                }
                if let Ok(mut state) = manager.0.lock() {
                    state.preview = Some(file);
                }
                result.message =
                    "Preview ready — first 10 seconds (or the full video if shorter).".into();
            } else if let Err(error) = remove_preview(file).await {
                log::error!("{error}");
            }
        }
        publish(&manager, &window, result);
    });
    Ok(job_id)
}
