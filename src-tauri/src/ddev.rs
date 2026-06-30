//! DDEV integration: detect the `ddev` binary, run subcommands, and parse the
//! JSON payloads from `ddev list -j` / `ddev describe <name> -j`.
//!
//! DDEV is the single source of truth for project state; this module only
//! shells out to it and deserializes its output defensively.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::process::Command;

/// Name of the DDEV binary; resolved via the process PATH.
const DDEV_BIN: &str = "ddev";

#[derive(Debug, thiserror::Error)]
pub enum DdevError {
    #[error("`ddev` was not found on your PATH")]
    NotFound,
    #[error("{0}")]
    CommandFailed(String),
    #[error("could not parse ddev output: {0}")]
    Parse(String),
    #[error("failed to run ddev: {0}")]
    Io(String),
}

/// Whether the `ddev` binary is available, plus its version string.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DdevAvailability {
    pub available: bool,
    pub version: Option<String>,
    pub message: Option<String>,
}

/// Normalized lifecycle status of a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Running,
    Stopped,
    Paused,
    Starting,
    Unknown,
}

impl ProjectStatus {
    fn from_ddev(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "running" => Self::Running,
            "stopped" => Self::Stopped,
            "paused" => Self::Paused,
            "starting" => Self::Starting,
            _ => Self::Unknown,
        }
    }
}

/// A project as exposed to the frontend (camelCase JSON).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    pub status: ProjectStatus,
    pub status_desc: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub primary_url: String,
    pub http_url: String,
    pub https_url: String,
    pub mailpit_url: String,
    pub approot: String,
    pub shortroot: String,
}

/// The exact (snake_case) shape DDEV emits in `list -j`'s `raw` array.
/// Every field is optional so unexpected/missing keys never break parsing.
#[derive(Debug, Default, Deserialize)]
struct RawProject {
    #[serde(default)]
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    status_desc: String,
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    primary_url: String,
    #[serde(default)]
    httpurl: String,
    #[serde(default)]
    httpsurl: String,
    #[serde(default)]
    mailpit_url: String,
    #[serde(default)]
    approot: String,
    #[serde(default)]
    shortroot: String,
}

impl From<RawProject> for Project {
    fn from(r: RawProject) -> Self {
        Project {
            status: ProjectStatus::from_ddev(&r.status),
            name: r.name,
            status_desc: r.status_desc,
            kind: r.kind,
            primary_url: r.primary_url,
            http_url: r.httpurl,
            https_url: r.httpsurl,
            mailpit_url: r.mailpit_url,
            approot: r.approot,
            shortroot: r.shortroot,
        }
    }
}

/// Run a `ddev` subcommand, capturing stdout/stderr and the exit code.
/// Maps a missing binary to `NotFound` and any non-zero exit to `CommandFailed`.
fn run_ddev(args: &[&str]) -> Result<String, DdevError> {
    let output = Command::new(DDEV_BIN).args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            DdevError::NotFound
        } else {
            DdevError::Io(e.to_string())
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let msg = if stderr.is_empty() {
            format!("`ddev {}` exited with {}", args.join(" "), output.status)
        } else {
            stderr
        };
        return Err(DdevError::CommandFailed(msg));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// DDEV's `-j` output is line-delimited JSON log records; the data payload is
/// the record carrying a top-level `raw` field. Tolerates a single pretty
/// document too (used by tests).
fn extract_raw(stdout: &str) -> Result<JsonValue, DdevError> {
    // Whole output as one JSON document.
    if let Ok(val) = serde_json::from_str::<JsonValue>(stdout.trim()) {
        if let Some(raw) = val.get("raw") {
            return Ok(raw.clone());
        }
    }
    // Otherwise scan line-delimited records for the one with `raw`.
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<JsonValue>(line) {
            if let Some(raw) = val.get("raw") {
                return Ok(raw.clone());
            }
        }
    }
    Err(DdevError::Parse(
        "no `raw` payload found in ddev output".to_string(),
    ))
}

fn parse_project_list(raw: &JsonValue) -> Result<Vec<Project>, DdevError> {
    // `raw` is `null` when there are no projects.
    if raw.is_null() {
        return Ok(Vec::new());
    }
    let arr = raw
        .as_array()
        .ok_or_else(|| DdevError::Parse("expected `raw` to be an array".to_string()))?;

    let mut projects = Vec::with_capacity(arr.len());
    for item in arr {
        let rp: RawProject =
            serde_json::from_value(item.clone()).map_err(|e| DdevError::Parse(e.to_string()))?;
        projects.push(Project::from(rp));
    }
    projects.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    Ok(projects)
}

// ---------- Public API ----------

/// Check whether `ddev` is installed and return its version.
pub fn check() -> DdevAvailability {
    match run_ddev(&["--version"]) {
        Ok(out) => DdevAvailability {
            available: true,
            version: Some(out.trim().to_string()),
            message: None,
        },
        Err(DdevError::NotFound) => DdevAvailability {
            available: false,
            version: None,
            message: Some("`ddev` was not found on your PATH.".to_string()),
        },
        Err(e) => DdevAvailability {
            available: false,
            version: None,
            message: Some(e.to_string()),
        },
    }
}

/// List all DDEV projects with normalized status.
pub fn list_projects() -> Result<Vec<Project>, DdevError> {
    let stdout = run_ddev(&["list", "-j"])?;
    let raw = extract_raw(&stdout)?;
    parse_project_list(&raw)
}

/// Return the full `describe` payload for a single project as raw JSON.
pub fn describe_project(name: &str) -> Result<JsonValue, DdevError> {
    let stdout = run_ddev(&["describe", name, "-j"])?;
    extract_raw(&stdout)
}

pub fn start_project(name: &str) -> Result<(), DdevError> {
    run_ddev(&["start", name]).map(|_| ())
}

pub fn stop_project(name: &str) -> Result<(), DdevError> {
    run_ddev(&["stop", name]).map(|_| ())
}

pub fn restart_project(name: &str) -> Result<(), DdevError> {
    run_ddev(&["restart", name]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_real_list_fixture() {
        let data = include_str!("testdata/list.json");
        let raw = extract_raw(data).expect("raw payload present");
        let projects = parse_project_list(&raw).expect("parses");
        assert!(!projects.is_empty(), "fixture has projects");

        let agora = projects
            .iter()
            .find(|p| p.name == "agora.community")
            .expect("agora.community present");
        assert_eq!(agora.status, ProjectStatus::Running);
        assert_eq!(agora.primary_url, "https://agora.community.ddev.site");
        assert_eq!(agora.kind, "laravel");
        assert!(agora.mailpit_url.starts_with("http"));
    }

    #[test]
    fn list_is_sorted_case_insensitively() {
        let data = include_str!("testdata/list.json");
        let raw = extract_raw(data).unwrap();
        let projects = parse_project_list(&raw).unwrap();
        let names: Vec<String> = projects.iter().map(|p| p.name.to_ascii_lowercase()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn parses_describe_fixture() {
        let data = include_str!("testdata/describe.json");
        let raw = extract_raw(data).expect("raw payload present");
        assert_eq!(raw.get("name").and_then(|v| v.as_str()), Some("agora.community"));
        assert!(raw.get("services").is_some());
    }

    #[test]
    fn empty_project_list_is_ok() {
        assert!(parse_project_list(&json!(null)).unwrap().is_empty());
        assert!(parse_project_list(&json!([])).unwrap().is_empty());
    }

    #[test]
    fn status_normalization() {
        assert_eq!(ProjectStatus::from_ddev("running"), ProjectStatus::Running);
        assert_eq!(ProjectStatus::from_ddev("OK"), ProjectStatus::Unknown);
        assert_eq!(ProjectStatus::from_ddev("Paused"), ProjectStatus::Paused);
        assert_eq!(ProjectStatus::from_ddev(""), ProjectStatus::Unknown);
    }

    #[test]
    fn tolerates_missing_fields() {
        let raw = json!([{ "name": "minimal" }]);
        let projects = parse_project_list(&raw).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "minimal");
        assert_eq!(projects[0].status, ProjectStatus::Unknown);
        assert_eq!(projects[0].primary_url, "");
    }

    #[test]
    fn unparseable_output_errors_cleanly() {
        let err = extract_raw("not json at all\n###\n").unwrap_err();
        assert!(matches!(err, DdevError::Parse(_)));
    }

    #[test]
    fn project_serializes_camel_case() {
        let p = Project {
            name: "x".into(),
            status: ProjectStatus::Running,
            status_desc: "running".into(),
            kind: "laravel".into(),
            primary_url: "https://x".into(),
            http_url: "http://x".into(),
            https_url: "https://x".into(),
            mailpit_url: "http://x:8025".into(),
            approot: "/p".into(),
            shortroot: "~/p".into(),
        };
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["type"], "laravel");
        assert_eq!(v["primaryUrl"], "https://x");
        assert_eq!(v["httpsUrl"], "https://x");
        assert_eq!(v["statusDesc"], "running");
        assert_eq!(v["status"], "running");
    }
}
