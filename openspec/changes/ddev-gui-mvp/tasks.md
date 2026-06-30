## 1. Project scaffold & tooling

- [x] 1.1 Verify prerequisites (Rust + Cargo, Node toolchain, Tauri CLI) and document required versions in the repo README
- [x] 1.2 Scaffold a Tauri app in the repo: `src-tauri/` Rust core and a web frontend (Svelte or vanilla+Vite — finalize here) — chose vanilla TS + Vite
- [ ] 1.3 Confirm the dev build runs (`tauri dev`) and shows an empty window on Ubuntu/GNOME — BLOCKED: needs `sudo apt` system libs (see README) + interactive GUI session
- [x] 1.4 Add base dependencies: serde/serde_json, async runtime (Tauri's), and Tauri plugins for tray, notifications, single-instance, and autostart

## 2. DDEV integration (Rust core)

- [x] 2.1 Add a `ddev` detection function that checks for the `ddev` binary on `PATH` and exposes an "available / not found" state
- [x] 2.2 Implement a process helper that runs a `ddev` subcommand, capturing stdout, stderr, and exit code, with typed error results
- [x] 2.3 Define serde structs for `ddev list -j` output and implement `list_projects` (name, status, type, primary URL, project root); tolerate missing/unknown fields
- [x] 2.4 Define structs for `ddev describe <project> -j` and implement `describe_project` (URLs incl. Mailpit/dashboard, services, router/db status)
- [x] 2.5 Implement async `start_project`, `stop_project`, `restart_project` returning success or captured stderr
- [x] 2.6 Handle empty project lists and unparseable output gracefully (no crash; typed error surfaced)
- [x] 2.7 Expose the above as Tauri commands and add unit tests for the JSON parsers using sample DDEV output (8 tests against real fixtures; `cargo test` requires the system libs to build)

## 3. Dashboard UI

- [x] 3.1 Build the project list view: one row/card per project with name, status indicator (running/stopped/paused), type, and primary URL
- [x] 3.2 Add the empty state ("no DDEV projects") and the "DDEV not found" state
- [x] 3.3 Wire auto-refresh on an interval plus a manual refresh control; pause polling when the window is hidden
- [x] 3.4 Add Start/Stop/Restart controls per project with in-progress feedback and status-aware enabling
- [x] 3.5 Show per-action error feedback (failure message) and reconcile to actual state on the next refresh
- [x] 3.6 Add quick-launch actions: open primary URL in browser, open Mailpit, open project folder; disable when stopped/URLs unavailable

## 4. Tray / status-bar indicator

- [x] 4.1 Create the tray indicator with an icon and a summary of running projects (count/active state)
- [x] 4.2 Build the indicator menu listing projects with start/stop quick actions, an "Open dashboard" item, and "Quit"
- [x] 4.3 Keep the indicator/menu in sync with project state changes from lifecycle actions and polling (background refresh loop)
- [x] 4.4 Implement run-in-background: closing the main window keeps the app alive in the tray; re-open from the menu; Quit exits fully
- [x] 4.5 Detect when the tray cannot be shown (no AppIndicator support) and fall back to window-only mode with guidance

## 5. Desktop notifications

- [x] 5.1 Emit notifications on start/stop/restart completion (project now running/stopped)
- [x] 5.2 Emit notifications on lifecycle errors (which project + action failed)
- [x] 5.3 Add a notifications on/off preference and respect it everywhere (checked in `notify()`)

## 6. App lifecycle, packaging & preferences

- [x] 6.1 Enable single-instance behavior (second launch focuses the existing window)
- [x] 6.2 Add a preferences store for notifications and autostart settings (`prefs.rs`, JSON in app config dir)
- [x] 6.3 Implement optional autostart-to-tray (autostart plugin + `--minimized` startup hides to tray)
- [x] 6.4 Add app icon and `.desktop` metadata; configure the Tauri bundler for `.deb` and AppImage targets
- [ ] 6.5 Produce a release build generating `.deb` and AppImage artifacts — BLOCKED: needs `sudo apt` system libs; run `npm run app:build`

## 7. Verification & docs

- [ ] 7.1 Manually verify list/status, start/stop/restart, quick-launch, tray, and notifications against local DDEV 1.25.2 on Ubuntu/GNOME 50 — BLOCKED: needs the built app (system libs) + interactive session
- [ ] 7.2 Verify single-instance, autostart-to-tray, and install/run of both `.deb` and AppImage on Ubuntu — BLOCKED: needs the release build
- [x] 7.3 Write README/usage docs: prerequisites (ddev on PATH, AppIndicator for tray), install, and known caveats
