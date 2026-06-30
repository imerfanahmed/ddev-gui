## ADDED Requirements

### Requirement: Single-instance application

The application SHALL run as a single instance so that launching it again focuses the existing window/indicator rather than starting a duplicate.

#### Scenario: Second launch focuses existing instance

- **WHEN** the application is already running and the user launches it again
- **THEN** no second instance starts and the existing main window is shown and focused

### Requirement: Linux distributable packages

The build SHALL produce installable Linux artifacts in `.deb` and AppImage formats, including an application icon and a `.desktop` entry.

#### Scenario: Build .deb and AppImage

- **WHEN** the release build is run
- **THEN** it produces a `.deb` package and an AppImage that install/run the app with its icon and menu entry on Ubuntu

#### Scenario: Desktop entry present

- **WHEN** the app is installed from the package
- **THEN** it appears in the application launcher with its name and icon

### Requirement: Optional autostart to tray

The application SHALL offer an option to start automatically on login, minimized to the tray indicator.

#### Scenario: Enable autostart

- **WHEN** the user enables "Start on login"
- **THEN** an autostart entry is created so the app launches to the tray on the next login

#### Scenario: Disable autostart

- **WHEN** the user disables "Start on login"
- **THEN** the autostart entry is removed and the app no longer launches automatically
