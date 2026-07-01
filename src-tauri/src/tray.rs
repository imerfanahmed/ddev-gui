//! Status-bar / system-tray indicator: a summary icon plus a per-project quick
//! menu, kept in sync with project state by a background refresh loop.

use crate::commands::{self, Lifecycle};
use crate::ddev::{self, Project, ProjectStatus};
use crate::AppState;
use std::time::Duration;
use tauri::menu::{Menu, MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

const TRAY_ID: &str = "main";

fn status_word(s: ProjectStatus) -> &'static str {
    match s {
        ProjectStatus::Running => "running",
        ProjectStatus::Stopped => "stopped",
        ProjectStatus::Paused => "paused",
        ProjectStatus::Starting => "starting",
        ProjectStatus::Unknown => "unknown",
    }
}

fn is_active(s: ProjectStatus) -> bool {
    matches!(s, ProjectStatus::Running | ProjectStatus::Paused)
}

/// Build the tray menu from the current project list.
fn build_menu(app: &AppHandle, projects: &[Project]) -> tauri::Result<Menu<Wry>> {
    let mut builder = MenuBuilder::new(app);

    if projects.is_empty() {
        let none = MenuItem::with_id(app, "noop", "No DDEV projects", false, None::<&str>)?;
        builder = builder.item(&none);
    } else {
        for p in projects {
            let active = is_active(p.status);
            let label = format!("{} — {}", p.name, status_word(p.status));
            let toggle_id = format!("{}::{}", if active { "stop" } else { "start" }, p.name);
            let toggle_label = if active { "Stop" } else { "Start" };

            let mut sub = SubmenuBuilder::new(app, label).item(&MenuItem::with_id(
                app,
                toggle_id,
                toggle_label,
                true,
                None::<&str>,
            )?);

            // Quick-launch the site only when it is reachable.
            if active && !p.primary_url.is_empty() {
                sub = sub.item(&MenuItem::with_id(
                    app,
                    format!("open::{}", p.name),
                    "Open site",
                    true,
                    None::<&str>,
                )?);
            }

            // Quick SSH into the container only when running.
            if p.status == ProjectStatus::Running {
                sub = sub.item(&MenuItem::with_id(
                    app,
                    format!("ssh::{}", p.name),
                    "SSH",
                    true,
                    None::<&str>,
                )?);
            }

            builder = builder.item(&sub.build()?);
        }
    }

    let menu = builder
        .separator()
        .item(&MenuItem::with_id(app, "show", "Open dashboard", true, None::<&str>)?)
        .item(&MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?)
        .separator()
        .item(&MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?)
        .build()?;
    Ok(menu)
}

/// Create the tray icon. Returns an error if the platform has no tray support,
/// which the caller uses to fall back to a window-only experience.
pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app, &[])?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("DDEV GUI")
        .menu(&menu)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_event);

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// Re-list projects, update the cache, and rebuild the tray menu + tooltip.
pub fn refresh_tray(app: &AppHandle) {
    let projects = ddev::list_projects().unwrap_or_default();
    if let Some(state) = app.try_state::<AppState>() {
        *state.projects.lock().unwrap() = projects.clone();
    }

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(menu) = build_menu(app, &projects) {
            let _ = tray.set_menu(Some(menu));
        }
        let running = projects
            .iter()
            .filter(|p| p.status == ProjectStatus::Running)
            .count();
        let _ = tray.set_tooltip(Some(&format!(
            "DDEV GUI — {running}/{} running",
            projects.len()
        )));
    }
}

/// Background loop that keeps the indicator current even when the window is
/// hidden. Honors the user's configured refresh interval.
pub fn start_background_refresh(app: AppHandle) {
    std::thread::spawn(move || loop {
        refresh_tray(&app);
        let secs = app
            .try_state::<AppState>()
            .map(|s| s.prefs.lock().unwrap().interval_secs())
            .unwrap_or(4);
        std::thread::sleep(Duration::from_secs(secs));
    });
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn open_site(app: &AppHandle, name: &str) {
    let url = app
        .try_state::<AppState>()
        .and_then(|state| {
            state
                .projects
                .lock()
                .unwrap()
                .iter()
                .find(|p| p.name == name)
                .map(|p| p.primary_url.clone())
        })
        .unwrap_or_default();

    if !url.is_empty() {
        use tauri_plugin_opener::OpenerExt;
        let _ = app.opener().open_url(url, None::<&str>);
    }
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref().to_string();
    match id.as_str() {
        "quit" => app.exit(0),
        "show" => show_main_window(app),
        "noop" => {}
        "refresh" => {
            let handle = app.clone();
            std::thread::spawn(move || {
                refresh_tray(&handle);
                let _ = handle.emit("ddev://refresh", ());
            });
        }
        other => {
            if let Some(name) = other.strip_prefix("start::") {
                spawn_tray_lifecycle(app.clone(), name.to_string(), Lifecycle::Start);
            } else if let Some(name) = other.strip_prefix("stop::") {
                spawn_tray_lifecycle(app.clone(), name.to_string(), Lifecycle::Stop);
            } else if let Some(name) = other.strip_prefix("open::") {
                open_site(app, name);
            } else if let Some(name) = other.strip_prefix("ssh::") {
                open_ssh_from_tray(app, name);
            }
        }
    }
}

fn spawn_tray_lifecycle(app: AppHandle, name: String, action: Lifecycle) {
    std::thread::spawn(move || {
        let _ = commands::run_lifecycle_blocking(&app, &name, action);
    });
}

fn open_ssh_from_tray(app: &AppHandle, name: &str) {
    let approot = app
        .try_state::<AppState>()
        .and_then(|state| {
            state
                .projects
                .lock()
                .unwrap()
                .iter()
                .find(|p| p.name == name)
                .map(|p| p.approot.clone())
        })
        .unwrap_or_default();

    if !approot.is_empty() {
        let _ = commands::open_ssh(name.to_string(), approot);
    }
}

fn handle_tray_event(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        show_main_window(tray.app_handle());
    }
}
