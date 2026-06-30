import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { enable as enableAutostart, disable as disableAutostart } from "@tauri-apps/plugin-autostart";
import {
  type Project,
  type ProjectStatus,
  type Prefs,
  checkDdev,
  listProjects,
  startProject,
  stopProject,
  restartProject,
  openPath,
  getPrefs,
  setPrefs,
  trayAvailable,
} from "./ddev";

const $ = <T extends HTMLElement>(sel: string): T => {
  const el = document.querySelector<T>(sel);
  if (!el) throw new Error(`Missing element: ${sel}`);
  return el;
};

const listEl = $<HTMLUListElement>("#project-list");
const emptyEl = $<HTMLDivElement>("#empty-state");
const bannerEl = $<HTMLDivElement>("#state-banner");
const summaryEl = $<HTMLSpanElement>("#summary");
const refreshBtn = $<HTMLButtonElement>("#refresh-btn");
const prefsBtn = $<HTMLButtonElement>("#prefs-btn");
const prefsDialog = $<HTMLDialogElement>("#prefs-dialog");

// Projects currently mid-action, so we can show spinners and disable buttons.
const busy = new Set<string>();
let prefs: Prefs = { notifications: true, autostart: false, refreshIntervalSecs: 4 };
let refreshTimer: number | undefined;
let lastProjects: Project[] = [];

// ---------- Rendering ----------

function statusLabel(s: ProjectStatus): string {
  switch (s) {
    case "running":
      return "running";
    case "stopped":
      return "stopped";
    case "paused":
      return "paused";
    case "starting":
      return "starting…";
    default:
      return "unknown";
  }
}

function isLaunchable(p: Project): boolean {
  return p.status === "running" && !!p.primaryUrl;
}

function render(projects: Project[]): void {
  lastProjects = projects;
  listEl.replaceChildren();

  const running = projects.filter((p) => p.status === "running").length;
  summaryEl.textContent =
    projects.length === 0 ? "" : `${running}/${projects.length} running`;

  if (projects.length === 0) {
    emptyEl.classList.remove("hidden");
    emptyEl.textContent = "No DDEV projects configured yet.";
    return;
  }
  emptyEl.classList.add("hidden");

  for (const p of projects) {
    listEl.appendChild(renderRow(p));
  }
}

function renderRow(p: Project): HTMLLIElement {
  const li = document.createElement("li");
  li.className = "project";
  li.dataset.name = p.name;

  const working = busy.has(p.name);

  const dot = document.createElement("span");
  dot.className = `dot dot-${p.status}`;
  if (working) dot.classList.add("dot-busy");

  const info = document.createElement("div");
  info.className = "project-info";
  const title = document.createElement("div");
  title.className = "project-name";
  title.textContent = p.name;
  const meta = document.createElement("div");
  meta.className = "project-meta";
  meta.textContent = `${p.type} · ${working ? "working…" : statusLabel(p.status)}` +
    (p.primaryUrl ? ` · ${p.primaryUrl}` : "");
  info.append(title, meta);

  const actions = document.createElement("div");
  actions.className = "project-actions";

  // Lifecycle controls reflect current status.
  if (p.status === "running" || p.status === "paused") {
    actions.appendChild(button("Stop", working, () => runAction(p.name, "stop")));
    actions.appendChild(button("Restart", working, () => runAction(p.name, "restart")));
  } else {
    actions.appendChild(button("Start", working, () => runAction(p.name, "start"), "btn-primary"));
  }

  // Quick-launch — only when running with a URL.
  if (isLaunchable(p)) {
    actions.appendChild(button("Open site", working, () => openUrl(p.primaryUrl)));
    if (p.mailpitUrl) {
      actions.appendChild(button("Mailpit", working, () => openUrl(p.mailpitUrl)));
    }
  }
  if (p.approot) {
    actions.appendChild(button("Folder", working, () => openPath(p.approot)));
  }

  li.append(dot, info, actions);
  return li;
}

function button(
  label: string,
  disabled: boolean,
  onClick: () => void | Promise<void>,
  extra = "btn-ghost",
): HTMLButtonElement {
  const b = document.createElement("button");
  b.className = `btn ${extra}`;
  b.textContent = label;
  b.disabled = disabled;
  b.addEventListener("click", () => void onClick());
  return b;
}

function showBanner(kind: "error" | "info" | "warn", html: string): void {
  bannerEl.className = `banner banner-${kind}`;
  bannerEl.innerHTML = html;
  bannerEl.classList.remove("hidden");
}

function clearBanner(): void {
  bannerEl.classList.add("hidden");
  bannerEl.textContent = "";
}

// ---------- Data flow ----------

async function refresh(): Promise<void> {
  const avail = await checkDdev();
  if (!avail.available) {
    listEl.replaceChildren();
    emptyEl.classList.add("hidden");
    summaryEl.textContent = "";
    showBanner(
      "error",
      `<strong>DDEV not found.</strong> ${
        avail.message ?? "Install DDEV and make sure <code>ddev</code> is on your PATH."
      } See <a href="https://ddev.readthedocs.io/en/stable/users/install/ddev-installation/" target="_blank" rel="noreferrer">installation docs</a>.`,
    );
    return;
  }

  try {
    const projects = await listProjects();
    clearBanner();
    render(projects);
  } catch (err) {
    showBanner("error", `<strong>Could not list projects.</strong> ${escapeHtml(String(err))}`);
  }
}

type Action = "start" | "stop" | "restart";

async function runAction(name: string, action: Action): Promise<void> {
  busy.add(name);
  rerenderRow(name);
  try {
    if (action === "start") await startProject(name);
    else if (action === "stop") await stopProject(name);
    else await restartProject(name);
  } catch (err) {
    // Lifecycle errors are also surfaced as native notifications from Rust;
    // show an inline banner so the dashboard reflects the failure too.
    showBanner("error", `<strong>${action} failed for ${escapeHtml(name)}:</strong> ${escapeHtml(String(err))}`);
  } finally {
    busy.delete(name);
    // Reconcile to the real state from DDEV.
    await refresh();
  }
}

function rerenderRow(name: string): void {
  const existing = listEl.querySelector<HTMLLIElement>(`li[data-name="${cssEscape(name)}"]`);
  const project = lastProjects.find((p) => p.name === name);
  if (existing && project) existing.replaceWith(renderRow(project));
}

// ---------- Polling (pause when window hidden) ----------

function startPolling(): void {
  stopPolling();
  refreshTimer = window.setInterval(() => {
    if (!document.hidden) void refresh();
  }, Math.max(2, prefs.refreshIntervalSecs) * 1000);
}

function stopPolling(): void {
  if (refreshTimer !== undefined) {
    clearInterval(refreshTimer);
    refreshTimer = undefined;
  }
}

document.addEventListener("visibilitychange", () => {
  if (!document.hidden) void refresh();
});

// ---------- Preferences dialog ----------

async function loadPrefs(): Promise<void> {
  prefs = await getPrefs();
  ($("#pref-notifications") as HTMLInputElement).checked = prefs.notifications;
  ($("#pref-autostart") as HTMLInputElement).checked = prefs.autostart;
  ($("#pref-interval") as HTMLInputElement).value = String(prefs.refreshIntervalSecs);
}

function wirePrefs(): void {
  prefsBtn.addEventListener("click", () => prefsDialog.showModal());

  const notif = $("#pref-notifications") as HTMLInputElement;
  const auto = $("#pref-autostart") as HTMLInputElement;
  const interval = $("#pref-interval") as HTMLInputElement;

  const persist = async () => {
    prefs = {
      notifications: notif.checked,
      autostart: auto.checked,
      refreshIntervalSecs: clampInterval(parseInt(interval.value, 10)),
    };
    await setPrefs(prefs);
    startPolling();
  };

  notif.addEventListener("change", persist);
  interval.addEventListener("change", persist);
  auto.addEventListener("change", async () => {
    // The OS-level autostart entry is managed by the autostart plugin.
    try {
      if (auto.checked) await enableAutostart();
      else await disableAutostart();
    } catch (err) {
      showBanner("warn", `Could not change autostart: ${escapeHtml(String(err))}`);
    }
    await persist();
  });
}

function clampInterval(n: number): number {
  if (Number.isNaN(n)) return 4;
  return Math.min(60, Math.max(2, n));
}

// ---------- Bootstrap ----------

async function init(): Promise<void> {
  await loadPrefs();
  wirePrefs();

  refreshBtn.addEventListener("click", () => void refresh());

  // The tray menu asks the dashboard to refresh after a lifecycle action.
  await listen("ddev://refresh", () => void refresh());

  // If the tray could not be created, tell the user the app is window-only.
  if (!(await trayAvailable())) {
    showBanner(
      "warn",
      "Status-bar indicator is unavailable (no AppIndicator support detected). " +
        "The app runs as a window only. On GNOME, install the AppIndicator extension to enable the tray.",
    );
  }

  await refresh();
  startPolling();
}

// ---------- helpers ----------

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!),
  );
}

function cssEscape(s: string): string {
  // Project names can contain dots; escape for attribute selectors.
  return s.replace(/["\\]/g, "\\$&");
}

void init();
