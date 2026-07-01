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

#[tauri::command]
pub fn open_ssh(name: String, approot: String) -> Result<(), String> {
    use std::process::Command;

    let shell_cmd = format!("ddev ssh {}; exec $SHELL", name);

    let terminals = [
        ("x-terminal-emulator", vec!["-e", "bash", "-c", &shell_cmd]),
        ("gnome-terminal", vec!["--", "bash", "-c", &shell_cmd]),
        ("konsole", vec!["-e", "bash", "-c", &shell_cmd]),
        ("xfce4-terminal", vec!["-e", "bash", "-c", &shell_cmd]),
        ("xterm", vec!["-e", "bash", "-c", &shell_cmd]),
        ("alacritty", vec!["-e", "bash", "-c", &shell_cmd]),
    ];

    let mut spawned = false;
    for (term, args) in &terminals {
        if Command::new(term)
            .current_dir(&approot)
            .args(args)
            .spawn()
            .is_ok()
        {
            spawned = true;
            break;
        }
    }

    if !spawned {
        return Err("No supported terminal emulator found (tried x-terminal-emulator, gnome-terminal, konsole, xfce4-terminal, xterm, alacritty). Please install one.".to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn save_project_config(
    approot: String,
    php_version: String,
    database: String,
    xdebug_enabled: bool,
    nodejs_version: String,
    webserver_type: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        use std::process::Command;
        let mut cmd = Command::new("ddev");
        cmd.current_dir(&approot)
           .args(&[
               "config",
               &format!("--php-version={}", php_version),
               &format!("--database={}", database),
               &format!("--xdebug-enabled={}", xdebug_enabled),
               &format!("--nodejs-version={}", nodejs_version),
               &format!("--webserver-type={}", webserver_type),
           ]);
        let output = cmd.output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                "Failed to update DDEV configuration".to_string()
            } else {
                stderr
            });
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub image: String,
    pub cpu_perc: String,
    pub mem_usage: String,
    pub mem_perc: String,
    pub net_io: String,
    pub block_io: String,
    pub pids: String,
}

#[tauri::command]
pub async fn get_project_containers(name: String) -> Result<Vec<ContainerInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use std::process::Command;
        
        // 1. Get associated containers via docker ps
        let filter = format!("label=com.ddev.site-name={}", name);
        let output = Command::new("docker")
            .args(&["ps", "--filter", &filter, "--format", "{{.ID}}\t{{.Names}}\t{{.Status}}\t{{.Image}}"])
            .output()
            .map_err(|e| e.to_string())?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                "Failed to run docker ps".to_string()
            } else {
                stderr
            });
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut containers = Vec::new();
        let mut names = Vec::new();
        
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 4 {
                let id = parts[0].to_string();
                let name = parts[1].to_string();
                let status = parts[2].to_string();
                let image = parts[3].to_string();
                
                containers.push((id, name.clone(), status, image));
                names.push(name);
            }
        }
        
        if names.is_empty() {
            return Ok(Vec::new());
        }
        
        // 2. Fetch stats for these containers
        let mut stats_args = vec!["stats", "--no-stream", "--format", "json"];
        for name in &names {
            stats_args.push(name);
        }
        
        let stats_output = Command::new("docker")
            .args(&stats_args)
            .output()
            .map_err(|e| e.to_string())?;
            
        let stats_stdout = String::from_utf8_lossy(&stats_output.stdout);
        
        // Parse stats lines
        let mut result = Vec::new();
        for line in stats_stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                let id = val.get("ID").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let name = val.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let cpu_perc = val.get("CPUPerc").and_then(|v| v.as_str()).unwrap_or("0%").to_string();
                let mem_usage = val.get("MemUsage").and_then(|v| v.as_str()).unwrap_or("0MiB / 0GiB").to_string();
                let mem_perc = val.get("MemPerc").and_then(|v| v.as_str()).unwrap_or("0%").to_string();
                let net_io = val.get("NetIO").and_then(|v| v.as_str()).unwrap_or("0B / 0B").to_string();
                let block_io = val.get("BlockIO").and_then(|v| v.as_str()).unwrap_or("0B / 0B").to_string();
                let pids = val.get("PIDs").and_then(|v| v.as_str()).unwrap_or("0").to_string();
                
                let mut status = "unknown".to_string();
                let mut image = "unknown".to_string();
                if let Some(c) = containers.iter().find(|(_, n, _, _)| n == &name || n == &format!("/{}", name)) {
                    status = c.2.clone();
                    image = c.3.clone();
                }
                
                result.push(ContainerInfo {
                    id,
                    name,
                    status,
                    image,
                    cpu_perc,
                    mem_usage,
                    mem_perc,
                    net_io,
                    block_io,
                    pids,
                });
            }
        }
        
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}



