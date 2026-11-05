use crate::{execution::resolve_tools, state::BatchManager};
use keroro_encoder_core::{
    ActiveProcess, AspectRatio, Cancellation, ResolutionPreset, VideoSettings,
};
use std::{path::Path, sync::Arc};
use tauri::{AppHandle, State, ipc::Response};

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(super) async fn generate_preview_snapshot(
    app: AppHandle,
    manager: State<'_, BatchManager>,
    processes: State<'_, Arc<ActiveProcess>>,
    file_path: String,
    timestamp_secs: Option<u64>,
    sub_index: Option<u32>,
    video_scale: String,
    video: Option<VideoSettings>,
    external_subtitle_path: Option<String>,
) -> Result<Response, String> {
    let cancel = Cancellation::new();
    let _reservation = manager.reserve("snapshot".into(), cancel.clone())?;
    let tools = resolve_tools(&app).await?;
    let video = match video {
        Some(video) => video,
        None => VideoSettings {
            aspect_ratio: match video_scale.as_str() {
                "4:3" => AspectRatio::Standard,
                "16:9" => AspectRatio::Wide,
                "auto" => AspectRatio::Auto,
                _ => return Err("Invalid preview aspect ratio.".into()),
            },
            resolution_preset: if video_scale == "16:9" {
                ResolutionPreset::Anamorphic
            } else {
                ResolutionPreset::Standard
            },
            ..VideoSettings::default()
        },
    };
    let result = keroro_encoder_core::snapshot::generate(
        &tools,
        Path::new(&file_path),
        timestamp_secs.unwrap_or(300),
        sub_index,
        external_subtitle_path.as_deref().map(Path::new),
        &video,
        &cancel,
        processes.inner(),
    )
    .await;
    processes.wait_until_idle().await;
    result.map(Response::new)
}

#[tauri::command]
pub(super) async fn cancel_preview_snapshot(
    manager: State<'_, BatchManager>,
) -> Result<(), String> {
    let state = manager.0.lock().map_err(|e| e.to_string())?;
    if let Some(active) = state
        .active
        .as_ref()
        .filter(|task| task.job_id == "snapshot")
    {
        active.cancellation.cancel();
    }
    Ok(())
}
