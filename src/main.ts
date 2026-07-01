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
  openSsh,
  describeProject,
  saveProjectConfig,
  getProjectContainers,
} from "./ddev";

const $ = <T extends Element>(sel: string): T => {
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

const configDialog = $<HTMLDialogElement>("#config-dialog");
const configTitle = $<HTMLHeadingElement>("#config-project-title");
const configPhp = $<HTMLSelectElement>("#config-php-version");
const configDb = $<HTMLSelectElement>("#config-database");
const configNode = $<HTMLSelectElement>("#config-nodejs");
const configWeb = $<HTMLSelectElement>("#config-webserver");
const configXdebug = $<HTMLInputElement>("#config-xdebug");
const configSaveBtn = $<HTMLButtonElement>("#config-save-btn");
const configSaveRestartBtn = $<HTMLButtonElement>("#config-save-restart-btn");

const detailsPlaceholder = $<HTMLDivElement>("#details-placeholder");
const detailsContent = $<HTMLDivElement>("#details-content");
const detailsActions = $<HTMLDivElement>("#details-actions");
const detailsName = $<HTMLHeadingElement>("#details-name");
const detailsStatusDot = $<HTMLSpanElement>("#details-status-dot");
const detailsStatusText = $<HTMLSpanElement>("#details-status-text");
const detailsMeta = $<HTMLParagraphElement>("#details-meta");
const detailsContainersList = $<HTMLTableSectionElement>("#details-containers-list");

const cpuChartLine = $<SVGPathElement>("#cpu-chart-line");
const cpuChartArea = $<SVGPathElement>("#cpu-chart-area");
const cpuChartValue = $<HTMLDivElement>("#cpu-chart-value");

const memChartLine = $<SVGPathElement>("#mem-chart-line");
const memChartArea = $<SVGPathElement>("#mem-chart-area");
const memChartValue = $<HTMLDivElement>("#mem-chart-value");

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

  if (activeDetailsProject && activeDetailsProject.name === p.name) {
    li.classList.add("selected");
  }

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
  meta.textContent = `${p.type} · ${working ? "working…" : statusLabel(p.status)}`;
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

  li.append(dot, info, actions);

  // Open details view when card is clicked (but not when clicking action buttons)
  li.addEventListener("click", (e) => {
    if ((e.target as HTMLElement).closest("button") || (e.target as HTMLElement).closest("a")) {
      return;
    }
    showProjectDetails(p);
  });

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

    if (activeDetailsProject) {
      const updated = projects.find((p) => p.name === activeDetailsProject!.name);
      if (updated) {
        activeDetailsProject = updated;
        detailsStatusDot.className = `dot dot-${updated.status}`;
        detailsStatusText.textContent = statusLabel(updated.status);
        detailsMeta.textContent = `${updated.type} · ${updated.approot}`;
        renderDetailsActions(updated);
      } else {
        activeDetailsProject = null;
        stopDetailsPolling();
        detailsPlaceholder.classList.remove("hidden");
        detailsContent.classList.add("hidden");
      }
    }
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
  if (document.hidden) {
    stopDetailsPolling();
  } else {
    if (activeDetailsProject) {
      void pollProjectStats();
      detailsTimer = window.setInterval(() => void pollProjectStats(), 3000);
    } else {
      void refresh();
    }
  }
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

let activeConfigProject: Project | null = null;

async function openConfigModal(p: Project): Promise<void> {
  activeConfigProject = p;
  configTitle.textContent = `Configure ${p.name}`;
  
  // Set default form values
  configPhp.value = "8.2";
  configDb.value = "mariadb:10.11";
  configNode.value = "20";
  configWeb.value = "nginx-fpm";
  configXdebug.checked = false;
  
  configDialog.showModal();

  try {
    const desc = await describeProject(p.name) as {
      php_version?: string;
      database_type?: string;
      database_version?: string;
      xdebug_enabled?: boolean;
      nodejs_version?: string;
      webserver_type?: string;
    };
    
    if (desc) {
      if (desc.php_version) configPhp.value = desc.php_version;
      
      if (desc.database_type && desc.database_version) {
        configDb.value = `${desc.database_type}:${desc.database_version}`;
      } else if (desc.database_type) {
        const found = Array.from(configDb.options).find(o => o.value.startsWith(desc.database_type + ":"));
        if (found) configDb.value = found.value;
      }
      
      if (desc.nodejs_version) configNode.value = desc.nodejs_version;
      if (desc.webserver_type) configWeb.value = desc.webserver_type;
      if (desc.xdebug_enabled !== undefined) configXdebug.checked = desc.xdebug_enabled;
    }
  } catch (err) {
    showBanner("error", `Could not read project configuration: ${escapeHtml(String(err))}`);
  }
}

async function saveConfig(restartAfterSave: boolean): Promise<void> {
  if (!activeConfigProject) return;
  const p = activeConfigProject;
  
  busy.add(p.name);
  rerenderRow(p.name);
  configDialog.close();

  try {
    showBanner("info", `Saving configuration for ${escapeHtml(p.name)}...`);
    await saveProjectConfig(
      p.approot,
      configPhp.value,
      configDb.value,
      configXdebug.checked,
      configNode.value,
      configWeb.value
    );
    
    if (restartAfterSave) {
      showBanner("info", `Restarting ${escapeHtml(p.name)} to apply configuration...`);
      await restartProject(p.name);
      showBanner("info", `Configuration saved and ${escapeHtml(p.name)} restarted.`);
    } else {
      showBanner("info", `Configuration saved for ${escapeHtml(p.name)}. Restart the project to apply changes.`);
    }
  } catch (err) {
    showBanner("error", `Failed to save configuration: ${escapeHtml(String(err))}`);
  } finally {
    busy.delete(p.name);
    await refresh();
  }
}

function wireConfig(): void {
  configSaveBtn.addEventListener("click", () => void saveConfig(false));
  configSaveRestartBtn.addEventListener("click", () => void saveConfig(true));
}

let activeDetailsProject: Project | null = null;
let detailsTimer: number | undefined = undefined;
const cpuHistory: number[] = [];
const memHistory: number[] = [];

function showProjectDetails(p: Project): void {
  activeDetailsProject = p;
  
  // Highlight select card in list
  listEl.querySelectorAll(".project").forEach((el) => {
    el.classList.remove("selected");
  });
  const currentEl = listEl.querySelector(`.project[data-name="${cssEscape(p.name)}"]`);
  if (currentEl) {
    currentEl.classList.add("selected");
  }

  // Toggle placeholder/content view in right panel
  detailsPlaceholder.classList.add("hidden");
  detailsContent.classList.remove("hidden");
  
  // Set header values
  detailsName.textContent = p.name;
  detailsStatusDot.className = `dot dot-${p.status}`;
  detailsStatusText.textContent = statusLabel(p.status);
  detailsMeta.textContent = `${p.type} · ${p.approot}`;
  
  // Render quick-launch action buttons
  renderDetailsActions(p);

  // Clear lists and histories
  detailsContainersList.replaceChildren();
  cpuHistory.length = 0;
  memHistory.length = 0;
  
  // Reset SVGs
  cpuChartLine.setAttribute("d", "");
  cpuChartArea.setAttribute("d", "");
  cpuChartValue.textContent = "0%";
  memChartLine.setAttribute("d", "");
  memChartArea.setAttribute("d", "");
  memChartValue.textContent = "0%";
  
  // Start polling
  stopDetailsPolling();
  
  // Fetch immediately
  void pollProjectStats();
  
  // Poll every 3 seconds
  detailsTimer = window.setInterval(() => {
    void pollProjectStats();
  }, 3000);
}

function renderDetailsActions(p: Project): void {
  detailsActions.replaceChildren();
  
  const working = busy.has(p.name);
  
  if (isLaunchable(p)) {
    detailsActions.appendChild(button("Open site", working, () => openUrl(p.primaryUrl)));
    if (p.mailpitUrl) {
      detailsActions.appendChild(button("Mailpit", working, () => openUrl(p.mailpitUrl)));
    }
  }
  if (p.status === "running") {
    detailsActions.appendChild(button("SSH", working, () => openSsh(p.name, p.approot)));
  }
  if (p.approot) {
    detailsActions.appendChild(button("Config", working, () => openConfigModal(p)));
    detailsActions.appendChild(button("Folder", working, () => openPath(p.approot)));
  }
}

function stopDetailsPolling(): void {
  if (detailsTimer !== undefined) {
    clearInterval(detailsTimer);
    detailsTimer = undefined;
  }
}

async function pollProjectStats(): Promise<void> {
  if (!activeDetailsProject) return;
  const p = activeDetailsProject;
  
  if (p.status !== "running") {
    detailsContainersList.replaceChildren();
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = 5;
    td.className = "empty";
    td.textContent = "Project is not running. Start it to monitor resources.";
    tr.appendChild(td);
    detailsContainersList.appendChild(tr);
    
    cpuChartValue.textContent = "0%";
    memChartValue.textContent = "0%";
    return;
  }
  
  try {
    const containers = await getProjectContainers(p.name);
    
    containers.sort((a, b) => a.name.localeCompare(b.name));
    
    detailsContainersList.replaceChildren();
    
    let totalCpu = 0;
    let totalMemPerc = 0;
    
    if (containers.length === 0) {
      const tr = document.createElement("tr");
      const td = document.createElement("td");
      td.colSpan = 5;
      td.className = "empty";
      td.textContent = "No containers found.";
      tr.appendChild(td);
      detailsContainersList.appendChild(tr);
    }
    
    for (const c of containers) {
      const tr = document.createElement("tr");
      
      const tdName = document.createElement("td");
      tdName.innerHTML = `<strong>${escapeHtml(c.name.replace(`ddev-${p.name}-`, ""))}</strong><div style="font-size:11px;color:var(--text-dim);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:180px;" title="${escapeHtml(c.image)}">${escapeHtml(c.image)}</div>`;
      
      const tdStatus = document.createElement("td");
      tdStatus.textContent = c.status;
      
      const tdCpu = document.createElement("td");
      tdCpu.textContent = c.cpuPerc;
      
      const tdMem = document.createElement("td");
      tdMem.innerHTML = `${escapeHtml(c.memUsage.split(" / ")[0])} <span style="font-size:11px;color:var(--text-dim);">(${escapeHtml(c.memPerc)})</span>`;
      
      const tdPids = document.createElement("td");
      tdPids.textContent = c.pids;
      
      tr.append(tdName, tdStatus, tdCpu, tdMem, tdPids);
      detailsContainersList.appendChild(tr);
      
      totalCpu += parseFloat(c.cpuPerc.replace(/[^\d.]/g, "")) || 0;
      totalMemPerc += parseFloat(c.memPerc.replace(/[^\d.]/g, "")) || 0;
    }
    
    cpuHistory.push(totalCpu);
    if (cpuHistory.length > 20) cpuHistory.shift();
    
    memHistory.push(totalMemPerc);
    if (memHistory.length > 20) memHistory.shift();
    
    cpuChartValue.textContent = `${totalCpu.toFixed(1)}%`;
    memChartValue.textContent = `${totalMemPerc.toFixed(2)}%`;
    
    drawChart(cpuChartLine, cpuChartArea, cpuHistory);
    drawChart(memChartLine, memChartArea, memHistory);
    
  } catch (err) {
    console.error("Error polling container stats", err);
  }
}

function drawChart(
  lineEl: SVGPathElement,
  areaEl: SVGPathElement,
  history: number[]
): void {
  if (history.length < 2) {
    return;
  }
  
  const width = 400;
  const height = 150;
  const maxVal = Math.max(5, ...history);
  
  let linePath = "";
  let areaPath = "";
  
  for (let i = 0; i < history.length; i++) {
    const val = history[i];
    const x = (i / (history.length - 1)) * width;
    const y = height - (val / maxVal) * (height - 20) - 10;
    
    if (i === 0) {
      linePath = `M ${x} ${y}`;
      areaPath = `M ${x} ${height} L ${x} ${y}`;
    } else {
      linePath += ` L ${x} ${y}`;
      areaPath += ` L ${x} ${y}`;
    }
  }
  
  const lastX = width;
  areaPath += ` L ${lastX} ${height} Z`;
  
  lineEl.setAttribute("d", linePath);
  areaEl.setAttribute("d", areaPath);
}

async function init(): Promise<void> {
  await loadPrefs();
  wirePrefs();
  wireConfig();

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
