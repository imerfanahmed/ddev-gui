//! Tauri command handlers exposed to the frontend, plus the shared lifecycle
//! runner used by both the dashboard and the tray menu.

use crate::ddev::{self, DdevAvailability, Project};
use crate::prefs::{self, Prefs};
use crate::AppState;
use serde_json::Value as JsonValue;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

/// A project lifecycle operation.
#[derive(Clone, Copy)]
pub enum Lifecycle {
    Start,
    Stop,
    Restart,
}

impl Lifecycle {
    fn verb(self) -> &'static str {
        match self {
            Lifecycle::Start => "start",
            Lifecycle::Stop => "stop",
            Lifecycle::Restart => "restart",
        }
    }

    fn past(self) -> &'static str {
        match self {
            Lifecycle::Start => "started",
            Lifecycle::Stop => "stopped",
            Lifecycle::Restart => "restarted",
        }
    }
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    if app.state::<AppState>().prefs.lock().unwrap().notifications {
        let _ = app.notification().builder().title(title).body(body).show();
    }
}

/// Run a lifecycle command (blocking), emit a notification, refresh the tray,
/// and ask the dashboard to reconcile. Shared by the command and the tray.
pub fn run_lifecycle_blocking(
    app: &AppHandle,
    name: &str,
    action: Lifecycle,
) -> Result<(), String> {
    let result = match action {
        Lifecycle::Start => ddev::start_project(name),
        Lifecycle::Stop => ddev::stop_project(name),
        Lifecycle::Restart => ddev::restart_project(name),
    };

    match &result {
        Ok(()) => notify(app, "DDEV", &format!("{name} {}", action.past())),
        Err(e) => notify(
            app,
            "DDEV error",
            &format!("{name} failed to {}: {e}", action.verb()),
        ),
    }

    // Keep the tray and dashboard in sync with the new state.
    crate::tray::refresh_tray(app);
    let _ = app.emit("ddev://refresh", ());

    result.map_err(|e| e.to_string())
}

async fn spawn_lifecycle(app: AppHandle, name: String, action: Lifecycle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || run_lifecycle_blocking(&app, &name, action))
        .await
        .map_err(|e| e.to_string())?
}

// ---------- Commands ----------

#[tauri::command]
pub fn check_ddev() -> DdevAvailability {
    ddev::check()
}

#[tauri::command]
pub async fn list_projects(app: AppHandle) -> Result<Vec<Project>, String> {
    let projects = tauri::async_runtime::spawn_blocking(ddev::list_projects)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    // Cache for the tray menu's quick-launch lookups.
    *app.state::<AppState>().projects.lock().unwrap() = projects.clone();
    Ok(projects)
}

#[tauri::command]
pub async fn describe_project(name: String) -> Result<JsonValue, String> {
    tauri::async_runtime::spawn_blocking(move || ddev::describe_project(&name))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_project(app: AppHandle, name: String) -> Result<(), String> {
    spawn_lifecycle(app, name, Lifecycle::Start).await
}

#[tauri::command]
pub async fn stop_project(app: AppHandle, name: String) -> Result<(), String> {
    spawn_lifecycle(app, name, Lifecycle::Stop).await
}

#[tauri::command]
pub async fn restart_project(app: AppHandle, name: String) -> Result<(), String> {
    spawn_lifecycle(app, name, Lifecycle::Restart).await
}

#[tauri::command]
pub fn open_path(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_prefs(state: State<'_, AppState>) -> Prefs {
    state.prefs.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_prefs(app: AppHandle, state: State<'_, AppState>, prefs: Prefs) -> Result<(), String> {
    *state.prefs.lock().unwrap() = prefs.clone();
    prefs::save(&app, &prefs)
}

#[tauri::command]
pub fn tray_available(state: State<'_, AppState>) -> bool {
    state.tray_available
}
