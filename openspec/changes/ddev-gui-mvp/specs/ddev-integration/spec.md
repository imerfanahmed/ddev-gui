## ADDED Requirements

### Requirement: Detect DDEV availability

The system SHALL detect whether the `ddev` binary is available on the user's `PATH` and surface a clear state when it is not, so the user understands why no projects appear.

#### Scenario: DDEV installed

- **WHEN** the application starts and `ddev` is found on `PATH`
- **THEN** the system proceeds to discover projects and reports DDEV as available

#### Scenario: DDEV not installed

- **WHEN** the application starts and `ddev` is not found on `PATH`
- **THEN** the system displays a "DDEV not found" state with guidance to install DDEV instead of an empty or error screen

### Requirement: Discover projects and status

The system SHALL discover all DDEV projects and their current status by invoking `ddev list -j` and parsing the JSON output.

#### Scenario: List running and stopped projects

- **WHEN** the system requests the project list
- **THEN** it returns each project's name, status (running/stopped/paused), type, primary URL, and project root path parsed from the JSON output

#### Scenario: No projects configured

- **WHEN** `ddev list -j` returns an empty project set
- **THEN** the system reports an empty list without raising an error

### Requirement: Retrieve project details

The system SHALL retrieve detailed information for a single project by invoking `ddev describe <project> -j` and parsing the JSON output.

#### Scenario: Describe a project

- **WHEN** the user opens a project's details
- **THEN** the system returns the project's URLs (including Mailpit and DDEV dashboard where present), services, and database/router status

### Requirement: Run lifecycle commands

The system SHALL start, stop, and restart a project by invoking `ddev start <project>`, `ddev stop <project>`, and `ddev restart <project>` respectively, and SHALL report success or the captured error output.

#### Scenario: Start succeeds

- **WHEN** the user starts a stopped project and the command exits successfully
- **THEN** the system reports success and the project's status reflects "running" on the next refresh

#### Scenario: Command fails

- **WHEN** a lifecycle command exits with a non-zero status
- **THEN** the system reports failure and surfaces the captured stderr message to the caller

### Requirement: Resilient output parsing

The system SHALL handle malformed, empty, or unexpected CLI output without crashing.

#### Scenario: Unparseable output

- **WHEN** a `ddev` command returns output that cannot be parsed as the expected JSON
- **THEN** the system reports a parsing error to the caller and continues running rather than terminating
