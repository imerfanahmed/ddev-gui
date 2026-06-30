import { invoke } from "@tauri-apps/api/core";

// Mirrors the camelCase payloads emitted by the Rust commands
// (serde rename_all = "camelCase").

export type ProjectStatus =
  | "running"
  | "stopped"
  | "paused"
  | "starting"
  | "unknown";

export interface Project {
  name: string;
  status: ProjectStatus;
  statusDesc: string;
  type: string;
  primaryUrl: string;
  httpUrl: string;
  httpsUrl: string;
  mailpitUrl: string;
  approot: string;
  shortroot: string;
}

export interface DdevAvailability {
  available: boolean;
  version: string | null;
  message: string | null;
}

export interface Prefs {
  notifications: boolean;
  autostart: boolean;
  refreshIntervalSecs: number;
}

/** Whether the `ddev` binary is on PATH, and its version. */
export function checkDdev(): Promise<DdevAvailability> {
  return invoke("check_ddev");
}

/** List all DDEV projects with current status. Rejects with an error string. */
export function listProjects(): Promise<Project[]> {
  return invoke("list_projects");
}

/** Detailed describe output for a single project (raw JSON value). */
export function describeProject(name: string): Promise<unknown> {
  return invoke("describe_project", { name });
}

export function startProject(name: string): Promise<void> {
  return invoke("start_project", { name });
}

export function stopProject(name: string): Promise<void> {
  return invoke("stop_project", { name });
}

export function restartProject(name: string): Promise<void> {
  return invoke("restart_project", { name });
}

/** Open an arbitrary local path in the file manager. */
export function openPath(path: string): Promise<void> {
  return invoke("open_path", { path });
}

export function getPrefs(): Promise<Prefs> {
  return invoke("get_prefs");
}

export function setPrefs(prefs: Prefs): Promise<void> {
  return invoke("set_prefs", { prefs });
}

/** Whether the tray indicator could be created at startup. */
export function trayAvailable(): Promise<boolean> {
  return invoke("tray_available");
}
