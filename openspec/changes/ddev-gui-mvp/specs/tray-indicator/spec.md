## ADDED Requirements

### Requirement: Status-bar indicator

The application SHALL provide an optional status-bar / system-tray indicator that is present while the app runs, integrating with the Ubuntu/GNOME top bar via the system tray (AppIndicator) mechanism.

#### Scenario: Indicator visible

- **WHEN** the application is running with the tray indicator enabled
- **THEN** an icon appears in the status bar showing a summary of running projects (e.g. a count or active/inactive state)

#### Scenario: Indicator reflects running count

- **WHEN** the number of running projects changes
- **THEN** the indicator updates its summary to reflect the new count

### Requirement: Quick menu from the indicator

The indicator SHALL expose a menu listing projects with quick actions so the user can manage projects without opening the main window.

#### Scenario: Toggle a project from the menu

- **WHEN** the user opens the indicator menu and activates start or stop on a project
- **THEN** the system runs the corresponding lifecycle command and the menu reflects the updated status

#### Scenario: Open the dashboard from the menu

- **WHEN** the user selects "Open dashboard" from the indicator menu
- **THEN** the main window is shown and focused

### Requirement: Run in background

The application SHALL be able to keep running in the tray when the main window is closed, so the indicator remains available.

#### Scenario: Close window keeps tray alive

- **WHEN** the user closes the main window while the tray indicator is enabled
- **THEN** the application keeps running in the background with the indicator still present, and can re-open the window from the menu

#### Scenario: Quit from the menu

- **WHEN** the user selects "Quit" from the indicator menu
- **THEN** the application exits fully and the indicator is removed
