# DDEV GUI

A lightweight Linux desktop dashboard and **status-bar (tray) indicator** for managing your local [DDEV](https://ddev.com) projects — see what's running at a glance, start/stop/restart projects, open sites and Mailpit, and get desktop notifications. Primary target: **Ubuntu / GNOME**.

Built with [Tauri 2](https://tauri.app) (Rust core + a small vanilla-TypeScript/Vite frontend). The app never talks to Docker directly — it shells out to the `ddev` CLI and parses its JSON output, so DDEV stays the single source of truth.

## Features

- **Project dashboard** — every DDEV project with live status (running / stopped / paused), type, and primary URL; auto-refreshing with a manual refresh button.
- **Lifecycle control** — start / stop / restart any project, with in-progress feedback and inline error messages.
- **Quick launch** — open the site URL, Mailpit, or the project folder.
- **Status-bar indicator** — a tray icon summarizing how many projects are running, with a quick menu to start/stop projects and open the dashboard, without opening the window.
- **Run in background** — closing the window keeps the app alive in the tray; quit fully from the tray menu.
- **Desktop notifications** — on lifecycle completion and errors (toggleable).
- **Start on login** — optionally autostart minimized to the tray.

## Prerequisites

### Runtime

- **DDEV** installed and on your `PATH` (verified against **v1.25.2**). See the [DDEV install docs](https://ddev.readthedocs.io/en/stable/users/install/ddev-installation/). Docker (or another container provider) is required by DDEV itself.
- For the **tray indicator on GNOME**: GNOME has no native tray. Install the **AppIndicator/KStatusNotifier** support extension (e.g. `gnome-shell-extension-appindicator`) and enable it. Without it, the app still runs as a window-only dashboard and shows a notice.

### Build toolchain

- **Rust** + Cargo (1.77+). Verified with Rust 1.93.
- **Node.js** + npm. Verified with Node 24 / npm 11.
- **Tauri Linux system libraries** (Debian/Ubuntu):

  ```bash
  sudo apt update
  sudo apt install -y \
    libwebkit2gtk-4.1-dev build-essential curl wget file pkg-config \
    libxdo-dev libssl-dev libdbus-1-dev \
    libayatana-appindicator3-dev librsvg2-dev
  ```

  `libwebkit2gtk-4.1-dev` is required to build/run any Tauri app; `libdbus-1-dev` is needed for desktop notifications; `libayatana-appindicator3-dev` is required for the tray indicator.

## Develop

```bash
npm install          # install frontend + Tauri CLI
npm run app:dev      # tauri dev — launches the app with hot-reload
```

`npm run dev` alone runs just the Vite frontend in a browser (no Tauri APIs).

## Build & package

```bash
npm run app:build    # tauri build — produces a .deb and an AppImage
```

Artifacts are written under `src-tauri/target/release/bundle/` (`deb/` and `appimage/`). The bundle targets are configured in `src-tauri/tauri.conf.json`.

## Project layout

```
.
├── index.html              # app shell
├── src/                    # frontend (vanilla TS + Vite)
│   ├── main.ts             # dashboard logic, polling, prefs
│   ├── ddev.ts             # typed bridge to Rust commands
│   └── styles.css
├── src-tauri/
│   ├── src/
│   │   ├── ddev.rs         # ddev CLI integration + JSON parsing (+ tests)
│   │   ├── commands.rs     # Tauri commands + lifecycle runner
│   │   ├── tray.rs         # status-bar indicator + menu + refresh loop
│   │   ├── prefs.rs        # preferences (JSON in app config dir)
│   │   └── lib.rs          # app setup, plugins, single-instance, window events
│   ├── icons/
│   ├── capabilities/default.json
│   └── tauri.conf.json
└── openspec/               # the OpenSpec change that produced this app
```

## Running the Rust tests

The JSON parsers are covered by unit tests using real DDEV fixtures
(`src-tauri/src/testdata/`):

```bash
cd src-tauri && cargo test
```

(Building the test binary still requires the Tauri system libraries above.)

## Known caveats

- **GNOME tray** requires the AppIndicator extension (see Prerequisites). The app detects when the tray can't be created and falls back to window-only mode.
- Status is **polled** from `ddev` on an interval (default 4s, configurable 2–60s); polling pauses while the window is hidden. Lifecycle commands can be slow (Docker image pulls/builds) and run asynchronously with progress feedback.
- DDEV's JSON schema is parsed defensively (unknown/missing fields tolerated). Pinned/tested against DDEV 1.25.x.
- Linux only for this release (no macOS/Windows bundles).

## License

MIT
