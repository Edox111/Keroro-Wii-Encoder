//! Tray/window exit policy and orderly shutdown of active or suspended media.
use crate::{ActiveProcess, execution::cleanup_saved_preview, state::BatchManager};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

pub(super) fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main")
        && let Err(error) = window
            .show()
            .and_then(|_| window.unminimize())
            .and_then(|_| window.set_focus())
    {
        log::error!("Cannot restore window: {error}");
    }
}

pub(super) fn request_close(app: &AppHandle, force_exit: bool) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let manager = app.state::<BatchManager>().inner().clone();
    let active = match manager.0.lock() {
        Ok(mut state) => {
            if state.close_prompt_open || state.exiting || state.updating {
                return;
            }
            if !force_exit && state.minimize_on_close {
                // Close-to-tray never cancels or confirms an active media task.
                drop(state);
                if let Err(error) = window.hide() {
                    log::error!("Cannot hide window: {error}");
                }
                return;
            }
            let active = state.active.clone();
            if active.is_some() {
                state.close_prompt_open = true;
            } else {
                state.exiting = true;
            }
            active
        }
        Err(error) => {
            log::error!("Cannot inspect active task: {error}");
            return;
        }
    };
    let Some(active) = active else {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            app.state::<Arc<ActiveProcess>>()
                .inner()
                .wait_until_idle()
                .await;
            cleanup_saved_preview(&manager).await;
            app.exit(0);
        });
        return;
    };
    show_main(app);
    let app = app.clone();
    let dialog = app
        .dialog()
        .message("Conversion is currently ongoing! Are you sure you want to cancel and exit?")
        .title("Cancel conversion?")
        .parent(&window)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Cancel and exit".into(),
            "Keep encoding".into(),
        ));
    dialog.show(move |confirmed| {
        if let Ok(mut state) = manager.0.lock() {
            state.close_prompt_open = false;
            if confirmed {
                state.exiting = true;
            }
        }
        if !confirmed {
            return;
        }
        active.cancellation.cancel();
        let _ = window.emit(
            "task-cancelling",
            "Cancelling conversion and cleaning temporary files before exit...",
        );
        tauri::async_runtime::spawn(async move {
            let mut finished = active.finished;
            loop {
                if *finished.borrow_and_update() {
                    break;
                }
                if finished.changed().await.is_err() {
                    break;
                }
            }
            // The worker has stopped/reaped FFmpeg and completed its cleanup.
            app.state::<Arc<ActiveProcess>>()
                .inner()
                .wait_until_idle()
                .await;
            cleanup_saved_preview(&manager).await;
            app.exit(0);
        });
    });
}
