//! DDEV GUI — a lightweight Linux desktop dashboard and tray indicator for DDEV.

mod commands;
mod ddev;
mod prefs;
mod tray;

use std::sync::Mutex;
use tauri::Manager;

/// Shared application state.
pub struct AppState {
    pub prefs: Mutex<prefs::Prefs>,
    pub projects: Mutex<Vec<ddev::Project>>,
    /// Whether the tray indicator was created successfully at startup.
    pub tray_available: bool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single-instance MUST be registered first: a second launch focuses
        // the existing window instead of starting a duplicate.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            // When autostarted, launch minimized to the tray.
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            let handle = app.handle().clone();
            let loaded = prefs::load(&handle);

            // Build the tray; record whether it succeeded so the UI can fall
            // back to window-only mode with guidance.
            let tray_available = tray::build_tray(&handle).is_ok();

            app.manage(AppState {
                prefs: Mutex::new(loaded),
                projects: Mutex::new(Vec::new()),
                tray_available,
            });

            // Autostart-to-tray: hide the window when launched with --minimized.
            let minimized = std::env::args().any(|a| a == "--minimized");
            if minimized && tray_available {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            tray::start_background_refresh(handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps the app alive in the tray (run in
            // background). With no tray, let the close proceed normally.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let keep_alive = window
                    .try_state::<AppState>()
                    .map(|s| s.tray_available)
                    .unwrap_or(false);
                if keep_alive {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_ddev,
            commands::list_projects,
            commands::describe_project,
            commands::start_project,
            commands::stop_project,
            commands::restart_project,
            commands::open_path,
            commands::get_prefs,
            commands::set_prefs,
            commands::tray_available,
            commands::open_ssh,
            commands::save_project_config,
            commands::get_project_containers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DDEV GUI");
}
