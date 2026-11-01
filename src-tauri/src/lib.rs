//! Desktop bootstrap; media processing is isolated in encoder-core.
mod commands;
mod execution;
mod lifecycle;
mod state;

pub use keroro_encoder_core::ActiveProcess;
use lifecycle::{request_close, show_main};
use state::{BatchManager, ClosePreferences};
use std::sync::Arc;
use tauri::{
    Manager, WindowEvent,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // The single-instance plugin must run before all other plugins and setup locks.
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        .manage(BatchManager::default())
        .manage(Arc::new(ActiveProcess::default()))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::pause_encode,
            commands::resume_encode,
            commands::analyze_dropped_paths,
            commands::get_recovery,
            commands::resume_batch,
            commands::discard_recovery,
            commands::check_environment,
            commands::start_batch,
            commands::start_preview,
            commands::get_batch_state,
            commands::get_performance_report,
            commands::read_preview,
            commands::release_preview,
            commands::set_close_preferences
        ])
        .setup(|app| {
            // First setup operations: all following failures/panics have a local sink.
            let log_directory = app.path().app_log_dir()?;
            keroro_encoder_core::local_log::install(&log_directory)
                .map_err(std::io::Error::other)?;
            let data = app.path().app_data_dir()?;
            let local_data = app.path().app_local_data_dir()?;
            let sessions =
                keroro_encoder_core::session::SessionStore::open_with_legacy(&local_data, &data)
                    .map_err(std::io::Error::other)?;
            app.manage(sessions);
            let preferences_path = data.join("window_preferences.json");
            if preferences_path.exists() {
                keroro_encoder_core::artifacts::no_links(&preferences_path)
                    .map_err(std::io::Error::other)?;
                let metadata = std::fs::metadata(&preferences_path)?;
                if metadata.len() <= 1024 {
                    match std::fs::read(&preferences_path)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| {
                            serde_json::from_slice::<ClosePreferences>(&bytes)
                                .map_err(|e| e.to_string())
                        }) {
                        Ok(preferences) => {
                            app.state::<BatchManager>()
                                .inner()
                                .0
                                .lock()
                                .map_err(|e| std::io::Error::other(e.to_string()))?
                                .minimize_on_close = preferences.minimize_on_close
                        }
                        Err(error) => log::warn!(
                            "Cannot restore close preference; using close-to-tray: {error}"
                        ),
                    }
                } else {
                    log::warn!("Ignoring oversized window preference file.");
                }
            }
            let cache = app.path().app_cache_dir()?;
            let artifacts = keroro_encoder_core::artifacts::ArtifactStore::open(&data, &cache)
                .map_err(std::io::Error::other)?;
            std::fs::create_dir_all(artifacts.workspace())?;
            keroro_encoder_core::artifacts::no_links(&artifacts.workspace())
                .map_err(std::io::Error::other)?;
            for warning in artifacts.sweep().map_err(std::io::Error::other)? {
                log::warn!("Startup cleanup: {warning}");
            }
            keroro_encoder_core::artifacts::install(artifacts).map_err(std::io::Error::other)?;

            let show =
                MenuItem::with_id(app, "show", "Show Keroro Wii Encoder", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "Exit…", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &exit])?;
            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or_else(|| std::io::Error::other("Application icon is missing."))?;
            TrayIconBuilder::with_id("keroro-tray")
                .icon(icon)
                .tooltip("Keroro Wii Encoder — click to restore")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "exit" => request_close(app, true),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }
                // Hold native close while hiding or running orderly exit/cancellation.
                api.prevent_close();
                request_close(window.app_handle(), false);
            }
        })
        .run(tauri::generate_context!());
    if let Err(error) = result {
        log::error!("Keroro Wii Encoder could not start: {error}");
    }
}
