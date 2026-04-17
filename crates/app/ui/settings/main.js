// Wait for Tauri to be ready, then initialize the settings page.
function initSettings() {
  if (!window.__TAURI__ || !window.__TAURI__.core) {
    console.warn("__TAURI__ not ready, retrying in 50ms");
    setTimeout(initSettings, 50);
    return;
  }
  run();
}

function run() {
  const { invoke } = window.__TAURI__.core;

  var cooldownSlider = document.getElementById("cooldownMinutes");
var cooldownValue = document.getElementById("cooldownValue");
var remindingCheckbox = document.getElementById("remindingEnabled");
var graceSlider = document.getElementById("autoHideGrace");
var graceValue = document.getElementById("graceValue");
var countdownSlider = document.getElementById("dismissCountdown");
var countdownValue = document.getElementById("countdownValue");
var skipConfirmation = document.getElementById("skipConfirmation");
var defaultAdapter = document.getElementById("defaultAdapter");
var debugLogging = document.getElementById("debugLogging");
var saveBtn = document.getElementById("saveBtn");
var resetPositionBtn = document.getElementById("resetPositionBtn");
var statusMsg = document.getElementById("statusMsg");

cooldownSlider.addEventListener("input", function() { cooldownValue.textContent = cooldownSlider.value; });
graceSlider.addEventListener("input", function() { graceValue.textContent = graceSlider.value; });
countdownSlider.addEventListener("input", function() { countdownValue.textContent = countdownSlider.value; });

async function loadConfig() {
  try {
    var config = await invoke("get_config");
    cooldownSlider.value = config.cooldown_minutes;
    cooldownValue.textContent = config.cooldown_minutes;
    remindingCheckbox.checked = config.reminding_enabled;
    graceSlider.value = config.auto_hide_grace_secs;
    graceValue.textContent = config.auto_hide_grace_secs;
    countdownSlider.value = config.dismiss_countdown_secs;
    countdownValue.textContent = config.dismiss_countdown_secs;
    skipConfirmation.checked = config.skip_dismiss_confirmation;
    defaultAdapter.value = config.default_adapter;
    debugLogging.checked = config.debug_logging;
  } catch (e) {
    statusMsg.textContent = "Failed to load config: " + e;
    statusMsg.style.color = "#f38ba8";
  }
}

saveBtn.addEventListener("click", async function() {
  var config = {
    cooldown_minutes: parseInt(cooldownSlider.value),
    reminding_enabled: remindingCheckbox.checked,
    auto_hide_grace_secs: parseInt(graceSlider.value),
    dismiss_countdown_secs: parseInt(countdownSlider.value),
    skip_dismiss_confirmation: skipConfirmation.checked,
    default_adapter: defaultAdapter.value,
    hud_position: null,
    debug_logging: debugLogging.checked
  };

  try {
    await invoke("apply_config", { config: config });
    statusMsg.textContent = "Settings saved";
    statusMsg.style.color = "#a6e3a1";
    // Hide the settings window shortly after saving so the user can see
    // the confirmation flash.
    setTimeout(function() {
      statusMsg.textContent = "";
      if (window.__TAURI__ && window.__TAURI__.window) {
        window.__TAURI__.window.getCurrentWindow().hide().catch(function(err) {
          console.warn("failed to hide settings window:", err);
        });
      }
    }, 600);
  } catch (e) {
    statusMsg.textContent = "Failed to save: " + e;
    statusMsg.style.color = "#f38ba8";
  }
});

resetPositionBtn.addEventListener("click", async function() {
  var config = await invoke("get_config");
  config.hud_position = null;
  try {
    await invoke("apply_config", { config: config });
    statusMsg.textContent = "HUD position reset";
    statusMsg.style.color = "#a6e3a1";
    setTimeout(function() { statusMsg.textContent = ""; }, 2000);
  } catch (e) {
    statusMsg.textContent = "Failed: " + e;
    statusMsg.style.color = "#f38ba8";
  }
});

  loadConfig();
}

// Start when DOM is ready
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initSettings);
} else {
  initSettings();
}
