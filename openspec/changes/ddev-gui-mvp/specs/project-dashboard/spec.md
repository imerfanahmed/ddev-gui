## ADDED Requirements

### Requirement: List projects with live status

The dashboard window SHALL display all discovered DDEV projects, each showing its name, status, type, and primary URL.

#### Scenario: Projects displayed

- **WHEN** the dashboard loads and projects exist
- **THEN** each project is shown as a row/card with a status indicator (e.g. running = green, stopped = grey) and its primary URL

#### Scenario: Empty state

- **WHEN** no DDEV projects are configured
- **THEN** the dashboard shows an explanatory empty state rather than a blank window

### Requirement: Auto-refresh status

The dashboard SHALL keep project status current by refreshing automatically and SHALL provide a manual refresh control.

#### Scenario: Periodic refresh

- **WHEN** the dashboard is open
- **THEN** project status is re-fetched on a recurring interval and the UI updates to reflect changes

#### Scenario: Manual refresh

- **WHEN** the user activates the refresh control
- **THEN** the system immediately re-fetches the project list and updates the UI

### Requirement: Lifecycle controls

The dashboard SHALL allow the user to start, stop, and restart each project, with the available actions reflecting the project's current status.

#### Scenario: Start a stopped project

- **WHEN** the user activates "Start" on a stopped project
- **THEN** the system runs the start command, shows in-progress feedback while it runs, and updates the row to "running" on success

#### Scenario: Stop a running project

- **WHEN** the user activates "Stop" on a running project
- **THEN** the system runs the stop command, shows in-progress feedback, and updates the row to "stopped" on success

#### Scenario: Action error feedback

- **WHEN** a lifecycle action fails
- **THEN** the dashboard shows an error indication with the failure message and leaves the project in its actual state after the next refresh

### Requirement: Quick-launch actions

The dashboard SHALL provide per-project quick-launch actions to open the primary site URL in the default browser, open the project's DDEV dashboard/Mailpit, and open the project folder in the file manager.

#### Scenario: Open site URL

- **WHEN** the user activates "Open site" on a running project
- **THEN** the system opens the project's primary URL in the default browser

#### Scenario: Quick-launch unavailable when stopped

- **WHEN** a project is stopped and its URLs are not available
- **THEN** browser/Mailpit quick-launch actions are disabled or hidden for that project
