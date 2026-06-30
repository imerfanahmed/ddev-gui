## ADDED Requirements

### Requirement: Notify on lifecycle completion

The system SHALL emit a desktop notification when a project lifecycle command (start/stop/restart) completes, using the freedesktop notification mechanism.

#### Scenario: Start completes

- **WHEN** a project finishes starting successfully
- **THEN** a desktop notification is shown indicating the project is now running

#### Scenario: Stop completes

- **WHEN** a project finishes stopping successfully
- **THEN** a desktop notification is shown indicating the project has stopped

### Requirement: Notify on errors

The system SHALL emit a desktop notification when a lifecycle command fails, summarizing the failure.

#### Scenario: Command error

- **WHEN** a lifecycle command fails
- **THEN** a desktop notification is shown indicating which project and action failed

### Requirement: Notification preference

The system SHALL allow the user to enable or disable desktop notifications.

#### Scenario: Notifications disabled

- **WHEN** the user has disabled notifications
- **THEN** no desktop notifications are emitted for lifecycle events or errors
