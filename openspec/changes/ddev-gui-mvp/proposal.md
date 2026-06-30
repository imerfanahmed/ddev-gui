## Why

DDEV is controlled entirely through its CLI, so checking which projects are running, starting or stopping them, or opening a site means remembering commands and switching to a terminal. Linux/Ubuntu users have no native desktop way to see and manage their DDEV projects at a glance. A lightweight desktop app with a window dashboard and an optional status-bar (tray) indicator makes the everyday "what's running / start this / open that" workflow a single click.

## What Changes

- Introduce a new **DDEV GUI desktop application** for Linux (primary target: Ubuntu/GNOME), built with **Tauri** (Rust core + web UI).
- Add a **project dashboard window** listing all DDEV projects with live status (running / stopped), type, and primary URL, auto-refreshing.
- Add **lifecycle controls** to start, stop, and restart any project from the UI, with progress/error feedback.
- Add **quick-launch actions** per project: open the primary site URL in the browser, open the DDEV dashboard/Mailpit, and open the project folder.
- Add an optional **status-bar / system-tray indicator** showing a summary of running projects with a quick menu (start/stop, open) without opening the main window.
- Add **desktop notifications** for project start/stop completion and command errors.
- Integrate with DDEV by shelling out to the `ddev` CLI and parsing its JSON output (`ddev list -j`, `ddev describe -j`).
- Package the app for distribution as **.deb** and **AppImage**.

## Capabilities

### New Capabilities
- `ddev-integration`: Discovering DDEV projects and their status, and running lifecycle commands (start/stop/restart) by invoking the `ddev` CLI and parsing its JSON output.
- `project-dashboard`: The main window listing projects with live status, details, and lifecycle/quick-launch actions.
- `tray-indicator`: The optional status-bar/system-tray indicator with a summary and quick actions.
- `desktop-notifications`: Desktop notifications for lifecycle events and errors.
- `app-packaging`: Building and packaging the Tauri app as a distributable Linux artifact (.deb / AppImage) plus app lifecycle (launch, single-instance, autostart-to-tray).

### Modified Capabilities
<!-- None — this is a greenfield project with no existing specs. -->

## Impact

- **New codebase**: Tauri project scaffold — Rust backend (`src-tauri/`) for invoking `ddev` and exposing commands/events to the frontend; web frontend (`src/`) for the dashboard UI.
- **External dependency**: requires the `ddev` binary to be installed and on `PATH` at runtime (DDEV v1.25.x verified locally); Docker/the configured container provider is required by DDEV itself.
- **System integration**: GNOME/Ubuntu status bar via AppIndicator/tray, freedesktop desktop notifications, `.desktop` autostart entry.
- **Build/release tooling**: Rust + Cargo, Node toolchain for the frontend, Tauri bundler for `.deb`/AppImage.
- No existing code or specs are affected (greenfield).
