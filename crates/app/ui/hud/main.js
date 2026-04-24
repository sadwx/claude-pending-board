const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const entryList      = document.getElementById("entryList");
const emptyState     = document.getElementById("emptyState");
const countBadge     = document.getElementById("countBadge");
const dismissBtn     = document.getElementById("dismissBtn");
const settingsBtn    = document.getElementById("settingsBtn");
const hudFooter      = document.getElementById("hudFooter");
const footerSummary  = document.getElementById("footerSummary");

const dismissPanel        = document.getElementById("dismissPanel");
const dismissHeading      = document.getElementById("dismissHeading");
const dismissSubtitle     = document.getElementById("dismissSubtitle");
const btnWakeMe           = document.getElementById("btnWakeMe");
const btnStaySilent       = document.getElementById("btnStaySilent");
const wakeMeCountdown     = document.getElementById("wakeMeCountdown");
const staySilentCountdown = document.getElementById("staySilentCountdown");

const setupCard      = document.getElementById("setupCard");
const setupTitle     = document.getElementById("setupTitle");
const setupSubtitle  = document.getElementById("setupSubtitle");
const setupInstallBtn = document.getElementById("setupInstallBtn");
const setupManualBtn  = document.getElementById("setupManualBtn");
const setupError     = document.getElementById("setupError");
const setupManual    = document.getElementById("setupManual");

let currentEntries = [];
let dismissCountdownTimer = null;
let isDismissPanelVisible = false;
// One of "installed", "not_installed", "cli_missing", or null while unknown.
let hookStatus = null;

function formatTime(ts) {
  const d = new Date(ts);
  const now = new Date();
  const diffMs = now - d;
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return "now";
  if (diffMin < 60) return diffMin + "m";
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return diffHr + "h";
  return Math.floor(diffHr / 24) + "d";
}

function extractProjectName(cwd) {
  if (!cwd) return "unknown";
  const parts = cwd.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || parts[parts.length - 2] || "unknown";
}

function typeOf(entry) {
  if (entry.state === "stale") return "stale";
  return entry.notification_type === "permission_prompt" ? "permission" : "idle";
}

function chipLabelFor(type) {
  return type === "permission" ? "perm" : type === "idle" ? "idle" : "stale";
}

function buildRow(entry) {
  const type = typeOf(entry);

  const row = document.createElement("div");
  row.className = "entry-row " + type;
  row.dataset.sessionId = entry.session_id;

  const dot = document.createElement("div");
  dot.className = "entry-dot";
  row.appendChild(dot);

  const content = document.createElement("div");
  content.className = "entry-content";

  const head = document.createElement("div");
  head.className = "entry-head";

  const chip = document.createElement("span");
  chip.className = "entry-chip " + type;
  chip.textContent = chipLabelFor(type);
  head.appendChild(chip);

  const project = document.createElement("span");
  project.className = "entry-project";
  project.textContent = extractProjectName(entry.cwd);
  head.appendChild(project);

  content.appendChild(head);

  const message = document.createElement("div");
  message.className = "entry-message";
  message.textContent = entry.message || "";
  content.appendChild(message);

  row.appendChild(content);

  const meta = document.createElement("div");
  meta.className = "entry-meta";
  const time = document.createElement("div");
  time.className = "entry-time";
  time.textContent = formatTime(entry.ts);
  meta.appendChild(time);
  row.appendChild(meta);

  row.addEventListener("click", function () { onEntryClick(entry.session_id); });
  return row;
}

function sortEntries(entries) {
  const rank = function (e) {
    if (e.state === "stale") return 2;
    return e.notification_type === "permission_prompt" ? 0 : 1;
  };
  return entries.slice().sort(function (a, b) {
    const r = rank(a) - rank(b);
    if (r !== 0) return r;
    return new Date(b.ts) - new Date(a.ts);
  });
}

function renderEntries(entries) {
  currentEntries = entries;

  const rows = entryList.querySelectorAll(".entry-row");
  rows.forEach(function (el) { el.remove(); });

  // If we have entries, the pipeline is clearly working — always show them
  // (and self-heal hookStatus if it's stale).
  if (entries.length > 0 && hookStatus !== "installed") {
    hookStatus = "installed";
  }
  setupCard.classList.add("hidden");

  if (entries.length === 0 && hookStatus && hookStatus !== "installed") {
    emptyState.style.display = "none";
    setupCard.classList.remove("hidden");
    countBadge.textContent = "0";
    countBadge.classList.add("empty");
    hudFooter.classList.add("hidden");
    footerSummary.textContent = "";
    return;
  }

  if (entries.length === 0) {
    emptyState.style.display = "flex";
    countBadge.textContent = "0";
    countBadge.classList.add("empty");
    hudFooter.classList.add("hidden");
    footerSummary.textContent = "";
    return;
  }

  emptyState.style.display = "none";
  countBadge.textContent = entries.length + " waiting";
  countBadge.classList.remove("empty");
  hudFooter.classList.remove("hidden");

  const projects = new Set();
  entries.forEach(function (e) { projects.add(extractProjectName(e.cwd)); });
  footerSummary.textContent =
    entries.length + " session" + (entries.length === 1 ? "" : "s") +
    " \u00B7 " + projects.size + " project" + (projects.size === 1 ? "" : "s");

  const sorted = sortEntries(entries);
  for (var i = 0; i < sorted.length; i++) {
    entryList.appendChild(buildRow(sorted[i]));
  }
}

async function onEntryClick(sessionId) {
  try {
    const result = await invoke("focus_entry", { sessionId: sessionId });
    console.log("focus result:", result);
  } catch (e) {
    console.error("focus error:", e);
  }
}

async function showDismissPanel() {
  if (isDismissPanelVisible) return;

  const config = await invoke("get_config");
  isDismissPanelVisible = true;
  entryList.style.display = "none";
  hudFooter.classList.add("hidden");
  dismissPanel.classList.add("active");

  dismissHeading.textContent = "Hide for " + config.cooldown_minutes + " minutes?";
  const n = currentEntries.length;
  dismissSubtitle.textContent = n + " item" + (n === 1 ? "" : "s") + " will stay on the board";

  if (config.reminding_enabled) {
    btnWakeMe.classList.add("default");
    btnStaySilent.classList.remove("default");
  } else {
    btnStaySilent.classList.add("default");
    btnWakeMe.classList.remove("default");
  }

  let remaining = config.dismiss_countdown_secs || 5;
  updateCountdown(remaining, config.reminding_enabled);

  dismissCountdownTimer = setInterval(function () {
    remaining--;
    updateCountdown(remaining, config.reminding_enabled);
    if (remaining <= 0) {
      clearInterval(dismissCountdownTimer);
      commitDismiss(null);
    }
  }, 1000);
}

function updateCountdown(secs, isWakeMeDefault) {
  if (isWakeMeDefault) {
    wakeMeCountdown.textContent = " \u00B7 " + secs + "s";
    staySilentCountdown.textContent = "";
  } else {
    wakeMeCountdown.textContent = "";
    staySilentCountdown.textContent = " \u00B7 " + secs + "s";
  }
}

function hideDismissPanel() {
  isDismissPanelVisible = false;
  if (dismissCountdownTimer) {
    clearInterval(dismissCountdownTimer);
    dismissCountdownTimer = null;
  }
  dismissPanel.classList.remove("active");
  entryList.style.display = "block";
  if (currentEntries.length > 0) {
    hudFooter.classList.remove("hidden");
  }
}

async function commitDismiss(remindingOverride) {
  hideDismissPanel();
  try {
    await invoke("dismiss_hud", { remindingOverride: remindingOverride });
  } catch (e) {
    console.error("dismiss error:", e);
  }
}

dismissBtn.addEventListener("click", function (e) {
  e.stopPropagation();
  showDismissPanel();
});

btnWakeMe.addEventListener("click", function () { commitDismiss(true); });
btnStaySilent.addEventListener("click", function () { commitDismiss(false); });

document.addEventListener("keydown", function (e) {
  if (!isDismissPanelVisible) return;
  if (e.key === "Escape") {
    commitDismiss(null);
  } else if (e.key === "Enter") {
    if (btnWakeMe.classList.contains("default")) commitDismiss(true);
    else commitDismiss(false);
  }
});

settingsBtn.addEventListener("click", async function () {
  try {
    await invoke("open_settings");
  } catch (e) {
    console.error("open_settings error:", e);
  }
});

// Manual drag fallback: data-tauri-drag-region is unreliable on macOS when
// the window is decorations(false) + always-on-top. Explicit startDragging
// on header mousedown works consistently across platforms.
document.querySelector(".header").addEventListener("mousedown", function (e) {
  if (e.button !== 0) return;
  if (e.target.closest("button")) return;
  getCurrentWindow().startDragging().catch(function (err) {
    console.error("startDragging error:", err);
  });
});

listen("entries-updated", function (event) {
  // If ops are reaching the store, the hook pipeline is demonstrably working.
  // Self-heal the status so the setup card stops showing.
  if (event.payload && event.payload.length > 0 && hookStatus !== "installed") {
    hookStatus = "installed";
    setupCard.classList.add("hidden");
  }
  renderEntries(event.payload);
});

listen("badge-count", function (event) {
  const n = event.payload;
  if (n > 0) {
    countBadge.textContent = n + " waiting";
    countBadge.classList.remove("empty");
  } else {
    countBadge.textContent = "0";
    countBadge.classList.add("empty");
  }
});

async function refreshHookStatus() {
  try {
    hookStatus = await invoke("check_hooks_installed");
  } catch (e) {
    console.error("check_hooks_installed error:", e);
    hookStatus = null;
  }
  if (hookStatus === "cli_missing") {
    setupTitle.textContent = "Claude Code not found";
    setupSubtitle.textContent = "The `claude` CLI isn't in PATH. Install Claude Code, then reopen this window.";
    setupInstallBtn.disabled = true;
  } else {
    setupTitle.textContent = "Hooks not installed";
    setupSubtitle.textContent = "The tray can't surface pending sessions until the Claude Code plugin is installed.";
    setupInstallBtn.disabled = false;
  }
  renderEntries(currentEntries);
}

setupInstallBtn.addEventListener("click", async function () {
  setupInstallBtn.disabled = true;
  setupManualBtn.disabled = true;
  setupError.classList.add("hidden");
  const originalLabel = setupInstallBtn.querySelector(".btn-label").textContent;
  setupInstallBtn.querySelector(".btn-label").textContent = "Installing…";
  try {
    await invoke("install_plugin");
    await refreshHookStatus();
  } catch (e) {
    console.error("install_plugin error:", e);
    setupError.textContent = String(e);
    setupError.classList.remove("hidden");
  } finally {
    setupInstallBtn.querySelector(".btn-label").textContent = originalLabel;
    setupInstallBtn.disabled = (hookStatus === "cli_missing");
    setupManualBtn.disabled = false;
  }
});

setupManualBtn.addEventListener("click", function () {
  setupManual.classList.toggle("hidden");
});

(async function () {
  try {
    const entries = await invoke("list_entries");
    await refreshHookStatus();
    renderEntries(entries);
  } catch (e) {
    console.error("initial load error:", e);
  }
})();
