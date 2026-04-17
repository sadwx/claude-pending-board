const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const entryList = document.getElementById("entryList");
const emptyState = document.getElementById("emptyState");
const countBadge = document.getElementById("countBadge");
const statusDot = document.getElementById("statusDot");
const dismissBtn = document.getElementById("dismissBtn");
const settingsBtn = document.getElementById("settingsBtn");
const dismissPanel = document.getElementById("dismissPanel");
const dismissHeading = document.getElementById("dismissHeading");
const dismissSubtitle = document.getElementById("dismissSubtitle");
const btnWakeMe = document.getElementById("btnWakeMe");
const btnStaySilent = document.getElementById("btnStaySilent");
const wakeMeCountdown = document.getElementById("wakeMeCountdown");
const staySilentCountdown = document.getElementById("staySilentCountdown");
const wakeMePill = document.getElementById("wakeMePill");
const staySilentPill = document.getElementById("staySilentPill");
const wakeMeCaptionTail = document.getElementById("wakeMeCaptionTail");

let currentEntries = [];
let dismissCountdownTimer = null;
let isDismissPanelVisible = false;

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

function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}

function renderEntries(entries) {
  currentEntries = entries;
  const oldEntries = entryList.querySelectorAll(".section-label, .entry-row");
  oldEntries.forEach(function(el) { el.remove(); });

  if (entries.length === 0) {
    emptyState.style.display = "flex";
    countBadge.textContent = "0";
    statusDot.classList.remove("has-items");
    return;
  }

  emptyState.style.display = "none";
  countBadge.textContent = entries.length.toString();
  statusDot.classList.add("has-items");

  var permissions = entries.filter(function(e) { return e.state === "live" && e.notification_type === "permission_prompt"; });
  var idles = entries.filter(function(e) { return e.state === "live" && e.notification_type === "idle_prompt"; });
  var stales = entries.filter(function(e) { return e.state === "stale"; });

  var groups = [
    { label: "PERMISSION", entries: permissions, cls: "permission" },
    { label: "IDLE", entries: idles, cls: "idle" },
    { label: "STALE", entries: stales, cls: "stale" }
  ];

  for (var g = 0; g < groups.length; g++) {
    var group = groups[g];
    if (group.entries.length === 0) continue;

    var label = document.createElement("div");
    label.className = "section-label";
    label.textContent = group.label;
    entryList.appendChild(label);

    for (var i = 0; i < group.entries.length; i++) {
      var entry = group.entries[i];
      var row = document.createElement("div");
      row.className = "entry-row " + group.cls;
      row.dataset.sessionId = entry.session_id;

      var icon = group.cls === "permission" ? "\uD83D\uDD10" : group.cls === "idle" ? "\uD83D\uDCAC" : "\uD83D\uDC7B";

      row.innerHTML =
        '<span class="entry-icon">' + icon + '</span>' +
        '<div class="entry-content">' +
          '<div class="entry-project">' + escapeHtml(extractProjectName(entry.cwd)) + '</div>' +
          '<div class="entry-message">' + escapeHtml(entry.message || "") + '</div>' +
        '</div>' +
        '<span class="entry-time">' + formatTime(entry.ts) + '</span>';

      (function(sid) {
        row.addEventListener("click", function() { onEntryClick(sid); });
      })(entry.session_id);

      entryList.appendChild(row);
    }
  }
}

async function onEntryClick(sessionId) {
  try {
    var result = await invoke("focus_entry", { sessionId: sessionId });
    console.log("focus result:", result);
  } catch (e) {
    console.error("focus error:", e);
  }
}

async function showDismissPanel() {
  if (isDismissPanelVisible) return;

  var config = await invoke("get_config");
  isDismissPanelVisible = true;
  entryList.style.display = "none";
  dismissPanel.classList.add("active");

  dismissHeading.textContent = "Going silent for " + config.cooldown_minutes + " minutes";
  dismissSubtitle.textContent = currentEntries.length + " items stay on board";

  // Update the caption under the Wake me button to reflect the actual
  // configured cooldown (not the hardcoded 15).
  if (wakeMeCaptionTail) {
    wakeMeCaptionTail.textContent = "after " + config.cooldown_minutes + " minutes";
  }

  // Apply "default" styling + DEFAULT pill to whichever button matches the
  // global Reminding setting.
  if (config.reminding_enabled) {
    btnWakeMe.classList.add("default");
    btnStaySilent.classList.remove("default");
    wakeMePill.style.display = "";
    staySilentPill.style.display = "none";
  } else {
    btnStaySilent.classList.add("default");
    btnWakeMe.classList.remove("default");
    wakeMePill.style.display = "none";
    staySilentPill.style.display = "";
  }

  var remaining = config.dismiss_countdown_secs || 5;
  updateCountdown(remaining, config.reminding_enabled);

  dismissCountdownTimer = setInterval(function() {
    remaining--;
    updateCountdown(remaining, config.reminding_enabled);
    if (remaining <= 0) {
      clearInterval(dismissCountdownTimer);
      commitDismiss(null);
    }
  }, 1000);
}

function updateCountdown(secs, isWakeMeDefault) {
  // Only the default button shows the countdown, inline after the label.
  // The element is a sibling span to btn-label, so we just append " · Ns".
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
}

async function commitDismiss(remindingOverride) {
  hideDismissPanel();
  try {
    await invoke("dismiss_hud", { remindingOverride: remindingOverride });
  } catch (e) {
    console.error("dismiss error:", e);
  }
}

dismissBtn.addEventListener("click", function(e) {
  e.stopPropagation();
  showDismissPanel();
});

btnWakeMe.addEventListener("click", function() { commitDismiss(true); });
btnStaySilent.addEventListener("click", function() { commitDismiss(false); });

document.addEventListener("keydown", function(e) {
  if (e.key === "Escape" && isDismissPanelVisible) {
    commitDismiss(null);
  }
});

settingsBtn.addEventListener("click", async function() {
  try {
    await invoke("open_settings");
  } catch (e) {
    console.error("open_settings error:", e);
  }
});

listen("entries-updated", function(event) {
  renderEntries(event.payload);
});

listen("badge-count", function(event) {
  countBadge.textContent = event.payload.toString();
});

(async function() {
  try {
    var entries = await invoke("list_entries");
    renderEntries(entries);
  } catch (e) {
    console.error("initial load error:", e);
  }
})();
