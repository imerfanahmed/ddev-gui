## Context

DDEV is a CLI-driven local development environment built on Docker. Everything users do — checking status, starting/stopping projects, opening sites — happens through `ddev` subcommands in a terminal. There is no native Linux desktop UI. The verified target environment is Ubuntu with GNOME Shell 50.1 and DDEV v1.25.2.

This is a greenfield project. We are building a **Tauri** desktop app: a Rust core that orchestrates the `ddev` CLI and a web-based frontend for the dashboard, plus a status-bar/tray indicator and desktop notifications. DDEV provides machine-readable output via `-j/--json-output` on commands like `ddev list -j` and `ddev describe <project> -j`, which gives us a stable integration surface without reimplementing DDEV logic.

Constraints:
- Must integrate with the GNOME/Ubuntu top bar (status bar) for the tray indicator. On modern GNOME this requires AppIndicator support (the user has GNOME 50; the AppIndicator/KStatusNotifier extension is the common path).
- Runtime requires the `ddev` binary on `PATH`; Docker is required by DDEV itself, not by this app.
- Single source of truth for project state is the `ddev` CLI; the app holds no persistent project database.

## Goals / Non-Goals

**Goals:**
- A lightweight (~10–15MB) native-feeling desktop app for managing DDEV projects on Ubuntu/GNOME.
- Live project list with status, start/stop/restart, and quick-launch (open site, Mailpit, folder).
- Optional status-bar/tray indicator summarizing running projects with a quick menu.
- Desktop notifications for lifecycle completion and errors.
- Distributable as `.deb` and AppImage.

**Non-Goals (MVP):**
- No reimplementation of DDEV features (creating/deleting/configuring projects, importing databases, SSH) — MVP wraps existing projects only.
- No remote/SSH or multi-host management.
- No support beyond Linux (no macOS/Windows builds in this iteration).
- No deep per-service control (individual container management) beyond what whole-project lifecycle commands provide.
- No bundled DDEV/Docker installer.

## Decisions

### Tauri (Rust + web UI) over Electron / GTK / Wails
- **Why**: Small binary and low memory vs Electron's bundled Chromium; first-class tray + notification plugins; clean `.deb`/AppImage bundling; Rust is well-suited to spawning and parsing CLI processes safely. Chosen by the user.
- **Alternatives**: Electron (easiest web path but ~150MB/heavy RAM); Python+GTK4/libadwaita (most native GNOME feel, but weaker single-binary packaging story); Go+Wails (good, but Tauri has a more mature tray/notification/bundler ecosystem). Trade-off accepted: Rust learning curve in the core.

### Integrate via the `ddev` CLI with JSON output, not a library or Docker API
- **Why**: `ddev list -j` / `ddev describe -j` are stable, supported, and keep DDEV as the single source of truth. Talking to Docker directly would duplicate DDEV's logic and break on DDEV upgrades.
- **How**: A Rust `ddev` module spawns commands (`std::process::Command` / `tokio`), captures stdout/stderr and exit codes, and deserializes JSON into typed structs (serde). Non-zero exits and unparseable output map to typed errors surfaced to the UI.
- **Alternative**: Parse human-readable output (brittle) or call the Docker API directly (reinvents DDEV) — both rejected.

### State model: poll-and-refresh, no local DB
- **Why**: Project state lives in DDEV/Docker; the app is a viewer/controller. A short polling interval (e.g. ~3–5s) plus manual refresh and post-action refresh keeps the UI current without a cache to invalidate.
- **How**: Rust exposes `list_projects`, `describe_project`, `start/stop/restart_project` as Tauri commands; the frontend calls them and renders. Long-running lifecycle commands run async and emit Tauri events for progress/completion, which also trigger notifications and indicator updates.
- **Trade-off**: Polling has a small latency/CPU cost; mitigated by a modest interval and pausing polling when the window is hidden (tray-only).

### Tray indicator via Tauri's tray API + AppIndicator
- **Why**: Meets the user's core ask (status bar). The tray menu is built from the same project data as the dashboard. On GNOME the indicator relies on AppIndicator support; we document the dependency rather than ship a GNOME Shell extension (which would tie us to Shell versions and a separate codebase).
- **Alternative considered**: A native GNOME Shell extension (best top-bar integration) — rejected for MVP because it fragments the codebase, is version-fragile across Shell releases, and can't host the full dashboard.

### Frontend framework: a lightweight web stack (Svelte or vanilla + Vite)
- **Why**: Keep the bundle small and the UI simple (a list + actions). Svelte gives reactivity with minimal overhead; vanilla is also viable. Decision can be finalized at scaffold time.

### Packaging via Tauri bundler
- **Why**: Built-in `.deb` and AppImage targets, icon and `.desktop` generation. Single-instance and autostart handled via Tauri plugins (`tauri-plugin-single-instance`, autostart plugin / freedesktop autostart entry).

## Risks / Trade-offs

- **GNOME has no native tray; AppIndicator extension may be absent** → Detect indicator availability; if the tray can't be shown, the app still works as a window-only dashboard and informs the user how to enable AppIndicator support.
- **DDEV JSON schema changes across versions** → Deserialize defensively (tolerate unknown/missing fields), pin tested behavior to DDEV 1.25.x, and surface a clear error if parsing fails rather than crashing (covered by the resilient-parsing requirement).
- **Lifecycle commands can be slow (Docker pulls/builds)** → Run async with in-progress UI feedback and completion notifications; never block the UI thread.
- **`ddev` not on PATH / Docker not running** → Detect `ddev` at startup and show a guidance state; surface DDEV's own error text when Docker is unavailable.
- **Rust learning curve** → Keep the Rust core thin (process orchestration + serde); push all UX into the web layer.
- **AppImage tray/notification quirks on some distros** → Test both `.deb` and AppImage on Ubuntu; document any sandbox/portal caveats.

## Migration Plan

Greenfield — no migration. Rollout:
1. Scaffold the Tauri project (Rust core + chosen frontend) in the repo.
2. Implement the `ddev` integration module and Tauri commands.
3. Build the dashboard UI, then the tray indicator and notifications.
4. Wire single-instance, autostart, and packaging; produce `.deb`/AppImage.
5. Manual verification against local DDEV 1.25.2 on Ubuntu/GNOME 50.

Rollback: as a standalone, read-mostly app that only invokes user-initiated `ddev` commands, "rollback" is simply uninstalling the package; it changes no DDEV/project state on its own.

## Open Questions

- Frontend framework: Svelte vs vanilla+Vite — finalize at scaffold.
- Exact polling interval and whether to back off when the window is hidden.
- Where to persist user preferences (notifications on/off, autostart) — Tauri store/config file location.
- Whether to detect and prompt to install the AppIndicator GNOME extension, or just document it.
- Minimum supported DDEV version to officially claim (test matrix beyond 1.25.x).
