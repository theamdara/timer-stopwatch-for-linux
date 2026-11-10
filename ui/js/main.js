// Catime UI controller: bootstrap, rendering, input, overlays and shortcuts.
// Timer math and persistence live in Rust; this file only presents state.

import { hasTauri, invoke, listen } from "./api.js";
import { primeAudio, playChime } from "./audio.js";
import {
  clockDisplay,
  countdownDisplay,
  stopwatchDisplay,
  pomodoroDisplay,
  humanDuration,
  phaseLabel,
  splitDuration,
} from "./format.js";
import { createSettings } from "./settings.js";

const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => Array.from(document.querySelectorAll(selector));

const MODE_LABELS = {
  clock: "Clock",
  countdown: "Countdown",
  stopwatch: "Stopwatch",
  pomodoro: "Pomodoro",
};

const state = {
  config: null,
  snapshot: null,
  version: "",
  configPath: "",
  menuOpen: false,
  popoverOpen: false,
  busy: false,
  lastMode: null,
  lastRunning: null,
  lastMetaKey: "",
  toastTimer: null,
  toastHideTimer: null,
};

const ui = {
  app: $("#app"),
  timeMain: $("#time-main"),
  timeSuffix: $("#time-suffix"),
  metaLeft: $("#meta-left"),
  metaRight: $("#meta-right"),
  metaEdit: $("#meta-edit"),
  cycleDots: $("#cycle-dots"),
  modeChip: $("#mode-chip"),
  toggleIcon: $("#toggle-icon-use"),
  btnToggle: $("#btn-toggle"),
  btnReset: $("#btn-reset"),
  btnSkip: $("#btn-skip"),
  btnSettings: $("#btn-settings"),
  btnMinimize: $("#btn-minimize"),
  btnClose: $("#btn-close"),
  timeButton: $("#time-button"),
  modeButtons: $$(".modebar .mode"),
  menu: $("#context-menu"),
  menuTop: $("#menu-always-top"),
  menuReset: $("#menu-reset"),
  menuSettings: $("#menu-settings"),
  menuQuit: $("#menu-quit"),
  popover: $("#duration-popover"),
  durationH: $("#duration-h"),
  durationM: $("#duration-m"),
  durationS: $("#duration-s"),
  durationPresets: $("#countdown-presets"),
  durationSet: $("#btn-duration-set"),
  durationStart: $("#btn-duration-start"),
  resizeGrip: $("#resize-grip"),
  toast: $("#toast"),
  toastTitle: $("#toast-title"),
  toastBody: $("#toast-body"),
  errorOverlay: $("#error-overlay"),
  errorMessage: $("#error-message"),
  configPathLabel: $("#config-path"),
  alwaysOnTop: $("#always-on-top"),
};

let settings = null;

/* ------------------------------------------------------------- appearance */

function hexToRgb(hex) {
  const match = /^#([0-9a-fA-F]{6})$/.exec(String(hex || ""));
  if (!match) return [16, 19, 32];
  const value = Number.parseInt(match[1], 16);
  return [(value >> 16) & 255, (value >> 8) & 255, value & 255];
}

function applyAppearance() {
  if (!state.config) return;
  const appearance = state.config.appearance;
  const root = document.documentElement.style;
  const background = hexToRgb(appearance.background_color);
  const text = hexToRgb(appearance.text_color);
  const accent = hexToRgb(appearance.accent_color);

  root.setProperty("--font-family", JSON.stringify(appearance.font_family || "Cantarell"));
  root.setProperty("--font-weight", String(appearance.font_weight || 300));
  root.setProperty("--time-size", `${appearance.font_size}px`);
  root.setProperty("--text", appearance.text_color);
  root.setProperty("--text-rgb", text.join(", "));
  root.setProperty("--accent", appearance.accent_color);
  root.setProperty("--accent-rgb", accent.join(", "));
  root.setProperty("--bg-rgb", background.join(", "));
  root.setProperty("--bg-alpha", String(appearance.background_opacity));
}

function setText(element, value) {
  if (element.textContent !== value) element.textContent = value;
}

/* ---------------------------------------------------------------- render -- */

function render(force = false) {
  if (!state.config || !state.snapshot) return;
  const snapshot = state.snapshot;
  const config = state.config;
  const mode = snapshot.mode;

  if (force || state.lastMode !== mode) {
    state.lastMode = mode;
    ui.app.dataset.mode = mode;
    ui.modeChip.textContent = MODE_LABELS[mode] || mode;
    for (const button of ui.modeButtons) {
      button.classList.toggle("active", button.dataset.mode === mode);
    }
  }

  ui.app.dataset.running = String(Boolean(snapshot.running));
  ui.app.dataset.phase = snapshot.pomodoro.phase;

  let display = { main: "00:00", suffix: "" };
  let metaLeft = "";
  let metaRight = "";
  let dots = null;

  switch (mode) {
    case "clock": {
      display = clockDisplay(snapshot.now_ms, config.clock);
      metaLeft = display.metaLeft;
      metaRight = display.metaRight;
      break;
    }
    case "countdown": {
      display = countdownDisplay(snapshot.countdown.remaining_ms);
      const total = snapshot.countdown.duration_ms;
      const remaining = snapshot.countdown.remaining_ms;
      metaLeft = snapshot.running
        ? "Running"
        : remaining <= 0
          ? "Complete"
          : remaining < total
            ? "Paused"
            : "Ready";
      metaRight = humanDuration(config.countdown.duration_seconds);
      break;
    }
    case "stopwatch": {
      display = stopwatchDisplay(snapshot.stopwatch.elapsed_ms);
      metaLeft = snapshot.running
        ? "Running"
        : snapshot.stopwatch.elapsed_ms > 0
          ? "Paused"
          : "Ready";
      metaRight = "Hundredths";
      break;
    }
    case "pomodoro": {
      display = pomodoroDisplay(snapshot.pomodoro.remaining_ms);
      const every = Math.max(1, snapshot.pomodoro.long_break_every);
      const completed = snapshot.pomodoro.completed_work;
      const round =
        snapshot.pomodoro.phase === "work"
          ? (completed % every) + 1
          : snapshot.pomodoro.cycle_position === 0
            ? every
            : snapshot.pomodoro.cycle_position;
      metaLeft = phaseLabel(snapshot.pomodoro.phase);
      metaRight = `Round ${Math.min(round, every)} / ${every}`;
      const done =
        snapshot.pomodoro.phase === "work"
          ? completed % every
          : snapshot.pomodoro.cycle_position;
      dots = { done, total: every };
      break;
    }
    default:
      break;
  }

  setText(ui.timeMain, display.main);
  setText(ui.timeSuffix, display.suffix || "");
  ui.timeSuffix.hidden = !display.suffix;

  const metaKey = `${metaLeft}|${metaRight}|${dots ? `${dots.done}/${dots.total}` : ""}`;
  if (force || metaKey !== state.lastMetaKey) {
    state.lastMetaKey = metaKey;
    setText(ui.metaLeft, metaLeft);
    setText(ui.metaRight, metaRight);
    renderCycleDots(dots);
  }

  ui.metaEdit.hidden = mode !== "countdown";
  ui.btnSkip.hidden = mode !== "pomodoro";

  if (force || state.lastRunning !== snapshot.running) {
    state.lastRunning = snapshot.running;
    ui.toggleIcon.setAttribute("href", snapshot.running ? "#i-pause" : "#i-play");
    const label = snapshot.running ? "Pause timer" : "Start timer";
    ui.btnToggle.setAttribute("aria-label", label);
    ui.btnToggle.title = `${snapshot.running ? "Pause" : "Start"} (Space)`;
    ui.timeButton.setAttribute("aria-label", label);
  }
}

function renderCycleDots(dots) {
  if (!dots || dots.total > 8) {
    ui.cycleDots.hidden = true;
    ui.cycleDots.replaceChildren();
    return;
  }

  ui.cycleDots.hidden = false;
  if (ui.cycleDots.childElementCount !== dots.total) {
    ui.cycleDots.replaceChildren(
      ...Array.from({ length: dots.total }, () => document.createElement("span")),
    );
  }
  Array.from(ui.cycleDots.children).forEach((dot, index) => {
    dot.classList.toggle("done", index < dots.done);
  });
}

function applyView(view, { syncForm = false } = {}) {
  if (!view) return;
  state.config = view.config;
  state.snapshot = view.state;
  applyAppearance();
  render(true);
  updateMenuState();
  if (ui.alwaysOnTop) {
    ui.alwaysOnTop.checked = Boolean(state.config.appearance.always_on_top);
  }
  if (syncForm && settings) settings.syncForm();
}

/* --------------------------------------------------------------- actions -- */

async function performAction(action, { silent = false } = {}) {
  if (!state.snapshot || state.busy) return;
  if (state.snapshot.mode === "clock" && action !== "reset") return;
  state.busy = true;
  try {
    const view = await invoke("timer_action", { action });
    applyView(view);
    if (action === "start" || action === "toggle") primeAudio();
  } catch (error) {
    if (!silent) showToast("Timer action failed", String(error), true);
  } finally {
    state.busy = false;
  }
}

async function setMode(mode) {
  if (!state.snapshot || state.snapshot.mode === mode) {
    closeMenu();
    return;
  }
  try {
    const view = await invoke("set_mode", { mode });
    applyView(view);
    closeMenu();
    closeDurationPopover();
  } catch (error) {
    showToast("Could not switch mode", String(error), true);
  }
}

async function toggleAlwaysOnTop() {
  if (!state.config) return;
  const enabled = !state.config.appearance.always_on_top;
  try {
    const view = await invoke("set_always_on_top", { enabled });
    applyView(view);
  } catch (error) {
    showToast("Could not change window layer", String(error), true);
  }
}

/* ----------------------------------------------------------- context menu -- */

function openMenu(clientX, clientY) {
  closeDurationPopover();
  ui.menu.hidden = false;
  state.menuOpen = true;
  updateMenuState();

  const appRect = ui.app.getBoundingClientRect();
  const menuRect = ui.menu.getBoundingClientRect();
  const maxLeft = Math.max(8, appRect.width - menuRect.width - 8);
  const maxTop = Math.max(8, appRect.height - menuRect.height - 8);
  const left = Math.min(maxLeft, Math.max(8, clientX - appRect.left));
  const top = Math.min(maxTop, Math.max(8, clientY - appRect.top));

  ui.menu.style.left = `${left}px`;
  ui.menu.style.top = `${top}px`;
}

function closeMenu() {
  if (!state.menuOpen) return;
  ui.menu.hidden = true;
  state.menuOpen = false;
}

function updateMenuState() {
  if (!state.config || !state.snapshot) return;
  for (const item of ui.menu.querySelectorAll("[data-menu-mode]")) {
    item.classList.toggle("active", item.dataset.menuMode === state.snapshot.mode);
  }
  ui.menuTop.classList.toggle("active", Boolean(state.config.appearance.always_on_top));
}

/* ------------------------------------------------------- duration popover -- */

function fillDurationInputs(totalSeconds) {
  const { hours, minutes, seconds } = splitDuration(totalSeconds);
  ui.durationH.value = String(hours);
  ui.durationM.value = String(minutes);
  ui.durationS.value = String(seconds);
  updatePresetSelection();
}

function readDurationInputs() {
  const hours = Math.max(0, Number.parseInt(ui.durationH.value, 10) || 0);
  const minutes = Math.max(0, Number.parseInt(ui.durationM.value, 10) || 0);
  const seconds = Math.max(0, Number.parseInt(ui.durationS.value, 10) || 0);
  return Math.min(24 * 3600, Math.max(1, hours * 3600 + minutes * 60 + seconds));
}

function updatePresetSelection() {
  const current = (() => {
    const hours = Number.parseInt(ui.durationH.value, 10) || 0;
    const minutes = Number.parseInt(ui.durationM.value, 10) || 0;
    const seconds = Number.parseInt(ui.durationS.value, 10) || 0;
    return hours * 3600 + minutes * 60 + seconds;
  })();

  for (const button of ui.durationPresets.querySelectorAll("button")) {
    button.classList.toggle("active", Number(button.dataset.seconds) === current);
  }
}

function openDurationPopover() {
  if (!state.snapshot || state.snapshot.mode !== "countdown") return;
  closeMenu();
  fillDurationInputs(state.config.countdown.duration_seconds);
  ui.popover.hidden = false;
  state.popoverOpen = true;
  const minutes = ui.durationM;
  setTimeout(() => {
    minutes.focus();
    minutes.select();
  }, 20);
}

function closeDurationPopover() {
  if (!state.popoverOpen) return;
  ui.popover.hidden = true;
  state.popoverOpen = false;
}

async function setDurationAndMaybeStart(start) {
  if (!state.snapshot) return;
  const seconds = readDurationInputs();
  try {
    const view = await invoke("set_countdown", { seconds });
    applyView(view);
    if (start) {
      const started = await invoke("timer_action", { action: "start" });
      applyView(started);
      primeAudio();
    }
    closeDurationPopover();
  } catch (error) {
    showToast("Could not set duration", String(error), true);
  }
}

/* ---------------------------------------------------------------- toasts -- */

function showToast(title, body, isError = false) {
  clearTimeout(state.toastTimer);
  clearTimeout(state.toastHideTimer);
  ui.toast.classList.remove("out");
  ui.toast.classList.toggle("error", Boolean(isError));
  ui.toastTitle.textContent = title;
  ui.toastBody.textContent = body;
  ui.toast.hidden = false;

  state.toastTimer = setTimeout(() => {
    ui.toast.classList.add("out");
    state.toastHideTimer = setTimeout(() => {
      ui.toast.hidden = true;
      ui.toast.classList.remove("out");
    }, 200);
  }, 4200);
}

function onFinished(event) {
  if (event.sound) playChime();
  showToast(event.title, event.body);
  ui.app.classList.remove("finish-flash");
  // Restart the CSS animation on repeated completions.
  void ui.app.offsetWidth;
  ui.app.classList.add("finish-flash");
  setTimeout(() => ui.app.classList.remove("finish-flash"), 1400);
}

/* -------------------------------------------------------------- overlays -- */

async function openSettings() {
  closeMenu();
  closeDurationPopover();
  if (settings) await settings.open();
}

async function closeSettings() {
  if (settings) await settings.close();
}

/* ------------------------------------------------------------------ boot -- */

function bindEvents() {
  ui.btnSettings.addEventListener("click", () => void openSettings());
  ui.btnMinimize.addEventListener("click", () => {
    invoke("minimize_window").catch(() => {});
  });
  ui.btnClose.addEventListener("click", () => void quit());
  $("#btn-settings-close").addEventListener("click", () => void closeSettings());

  ui.timeButton.addEventListener("click", () => {
    if (!state.snapshot) return;
    if (state.snapshot.mode === "clock") {
      void openSettings();
      return;
    }
    void performAction("toggle");
  });

  ui.btnToggle.addEventListener("click", () => void performAction("toggle"));
  ui.btnReset.addEventListener("click", () => void performAction("reset"));
  ui.btnSkip.addEventListener("click", () => void performAction("skip"));

  for (const button of ui.modeButtons) {
    button.addEventListener("click", () => void setMode(button.dataset.mode));
  }

  for (const item of ui.menu.querySelectorAll("[data-menu-mode]")) {
    item.addEventListener("click", () => void setMode(item.dataset.menuMode));
  }
  ui.menuTop.addEventListener("click", () => {
    closeMenu();
    void toggleAlwaysOnTop();
  });
  ui.menuReset.addEventListener("click", () => {
    closeMenu();
    void performAction("reset");
  });
  ui.menuSettings.addEventListener("click", () => {
    closeMenu();
    void openSettings();
  });
  ui.menuQuit.addEventListener("click", () => void quit());

  ui.metaEdit.addEventListener("click", () => openDurationPopover());
  $("#btn-popover-close").addEventListener("click", closeDurationPopover);
  ui.durationSet.addEventListener("click", () => void setDurationAndMaybeStart(false));
  ui.durationStart.addEventListener("click", () => void setDurationAndMaybeStart(true));

  for (const input of [ui.durationH, ui.durationM, ui.durationS]) {
    input.addEventListener("input", updatePresetSelection);
  }
  ui.durationPresets.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-seconds]");
    if (!button) return;
    fillDurationInputs(Number(button.dataset.seconds));
  });

  ui.resizeGrip.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    event.stopPropagation();
    invoke("start_resize").catch(() => {});
  });

  // Native-feeling drag from any non-interactive surface.
  document.addEventListener("pointerdown", (event) => {
    primeAudio();

    if (state.menuOpen && !event.target.closest("#context-menu")) {
      closeMenu();
    }
    if (
      state.popoverOpen &&
      !event.target.closest("#duration-popover") &&
      !event.target.closest("#meta-edit")
    ) {
      closeDurationPopover();
    }

    if (event.button !== 0) return;
    const dragSurface = event.target.closest("[data-drag]");
    if (!dragSurface) return;
    if (
      event.target.closest(
        "button, input, select, textarea, a, [data-no-drag], .settings-scroll",
      )
    ) {
      return;
    }
    event.preventDefault();
    invoke("start_dragging").catch(() => {});
  });

  document.addEventListener("contextmenu", (event) => {
    const editing = event.target.closest("input, textarea");
    if (editing) return;
    event.preventDefault();
    if (settings?.isOpen || state.popoverOpen) return;
    openMenu(event.clientX, event.clientY);
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      if (state.menuOpen) {
        closeMenu();
      } else if (state.popoverOpen) {
        closeDurationPopover();
      } else if (settings?.isOpen) {
        void closeSettings();
      }
      return;
    }

    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "q") {
      event.preventDefault();
      void quit();
      return;
    }

    const target = event.target;
    const tag = target?.tagName;
    const editing =
      tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA" || target?.isContentEditable;
    if (editing || settings?.isOpen) return;

    if (event.code === "Space") {
      if (tag === "BUTTON") return; // Let the focused button activate itself.
      event.preventDefault();
      void performAction("toggle");
      return;
    }

    switch (event.key.toLowerCase()) {
      case "r":
        event.preventDefault();
        void performAction("reset");
        break;
      case "s":
        event.preventDefault();
        void openSettings();
        break;
      case "t":
        event.preventDefault();
        void toggleAlwaysOnTop();
        break;
      case "1":
      case "2":
      case "3":
      case "4":
        event.preventDefault();
        void setMode(["clock", "countdown", "stopwatch", "pomodoro"][Number(event.key) - 1]);
        break;
      default:
        break;
    }
  });

  window.addEventListener("blur", () => closeMenu());
}

async function quit() {
  try {
    if (settings) await settings.flushNow();
  } catch {
    // Continue shutting down even if the last save failed.
  }
  try {
    await invoke("close_window");
  } catch {
    try {
      await invoke("quit_app");
    } catch {
      window.close();
    }
  }
}

async function boot() {
  bindEvents();

  settings = createSettings({
    state,
    invoke,
    applyAppearance,
    render,
    applyView,
    showToast,
    updateMenuState,
    primeAudio,
  });

  if (!hasTauri) {
    showBootError(
      "The native bridge is unavailable. Start Catime with `cargo tauri dev` (opening index.html directly cannot access the timer backend).",
    );
    return;
  }

  try {
    const bootstrap = await invoke("get_bootstrap");
    state.config = bootstrap.config;
    state.snapshot = bootstrap.state;
    state.version = bootstrap.version;
    state.configPath = bootstrap.config_path;

    ui.configPathLabel.textContent = bootstrap.config_path;
    applyAppearance();
    settings.syncForm();
    render(true);
    updateMenuState();

    await listen("timer-tick", (snapshot) => {
      state.snapshot = snapshot;
      render();
    });
    await listen("timer-finished", onFinished);
  } catch (error) {
    showBootError(String(error));
  }
}

function showBootError(message) {
  ui.errorOverlay.hidden = false;
  ui.errorMessage.textContent = message;
  console.error("[catime]", message);
}

void boot();
