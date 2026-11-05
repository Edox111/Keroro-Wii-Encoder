use crate::{ActiveProcess, commands::change_pause, state::BatchManager};
use std::sync::Arc;
use tauri::{State, WebviewWindow};

// Queue cards use the native job ID, so a stale click cannot suspend a later file.
#[tauri::command]
pub(super) async fn pause_encoding(
    window: WebviewWindow,
    manager: State<'_, BatchManager>,
    processes: State<'_, Arc<ActiveProcess>>,
    file_id: String,
) -> Result<(), String> {
    change_pause(window, manager.inner(), processes.inner(), &file_id, true)
}
#[tauri::command]
pub(super) async fn resume_encoding(
    window: WebviewWindow,
    manager: State<'_, BatchManager>,
    processes: State<'_, Arc<ActiveProcess>>,
    file_id: String,
) -> Result<(), String> {
    change_pause(window, manager.inner(), processes.inner(), &file_id, false)
}
