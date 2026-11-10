// Preferences sheet controller. All inputs are live-previewed and saved with a
// short debounce; geometry is owned by the Rust side.

import { splitDuration } from "./format.js";

const THEMES = {
  midnight: { background: "#101320", text: "#F5F7FF", accent: "#8AB4FF" },
  ember: { background: "#1C1210", text: "#FFF1E8", accent: "#FF9E6D" },
  moss: { background: "#0F1A17", text: "#ECFFF6", accent: "#7AD9AE" },
  lilac: { background: "#161223", text: "#F7F2FF", accent: "#C4A7FF" },
  mono: { background: "#0B0B0C", text: "#FAFAFA", accent: "#D7D7D7" },
};

const clamp = (value, min, max) => Math.min(max, Math.max(min, value));

function integer(value, fallback = 0) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : fallback;
}

export function createSettings(ctx) {
  const { state, invoke } = ctx;
  const app = document.getElementById("app");
  const sheet = document.getElementById("settings");

  const el = {
    fontFamily: document.getElementById("font-family"),
    fontWeight: document.getElementById("font-weight"),
    fontSize: document.getElementById("font-size"),
    fontSizeOut: document.getElementById("font-size-out"),
    textColor: document.getElementById("text-color"),
    accentColor: document.getElementById("accent-color"),
    backgroundColor: document.getElementById("background-color"),
    opacity: document.getElementById("background-opacity"),
    opacityOut: document.getElementById("opacity-out"),
    alwaysOnTop: document.getElementById("always-on-top"),
    sizePresets: document.getElementById("size-presets"),
    themes: document.getElementById("theme-swatches"),

    clock24h: document.getElementById("clock-24h"),
    clockSeconds: document.getElementById("clock-seconds"),
    clockDate: document.getElementById("clock-date"),

    countdownH: document.getElementById("countdown-h"),
    countdownM: document.getElementById("countdown-m"),
    countdownS: document.getElementById("countdown-s"),
    countdownSound: document.getElementById("countdown-sound"),
    countdownNotify: document.getElementById("countdown-notify"),

    pomoWork: document.getElementById("pomo-work"),
    pomoShort: document.getElementById("pomo-short"),
    pomoLong: document.getElementById("pomo-long"),
    pomoEvery: document.getElementById("pomo-every"),
    pomoAuto: document.getElementById("pomo-auto"),
    pomoSound: document.getElementById("pomo-sound"),
    pomoNotify: document.getElementById("pomo-notify"),

    reset: document.getElementById("btn-reset-config"),
    saveState: document.getElementById("save-state"),
  };

  let isOpen = false;
  let saveTimer = null;
  let saveSequence = 0;
  let lastSavedAt = 0;
  let suppress = false;

  function setSaveState(text, kind = "") {
    el.saveState.textContent = text;
    el.saveState.classList.toggle("saving", kind === "saving");
    el.saveState.classList.toggle("error", kind === "error");
  }

  function scheduleSave() {
    if (suppress) return;
    clearTimeout(saveTimer);
    setSaveState("Saving…", "saving");
    saveTimer = setTimeout(() => {
      saveTimer = null;
      void flushSave();
    }, 280);
  }

  async function flushSave() {
    const sequence = ++saveSequence;
    const outgoing = JSON.parse(JSON.stringify(state.config));
    try {
      const view = await invoke("save_config", { config: outgoing });
      if (sequence !== saveSequence) return;
      ctx.applyView(view, { syncForm: false });
      lastSavedAt = Date.now();
      setSaveState("Saved");
    } catch (error) {
      if (sequence === saveSequence) setSaveState("Save failed", "error");
      ctx.showToast("Could not save settings", String(error), true);
    }
  }

  async function flushNow() {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    await flushSave();
  }

  function touch({ render = false, save = true } = {}) {
    if (suppress) return;
    ctx.applyAppearance();
    if (render) ctx.render(true);
    if (save) scheduleSave();
  }

  function syncThemeActive() {
    const { appearance } = state.config;
    for (const button of el.themes.querySelectorAll("button")) {
      const theme = THEMES[button.dataset.theme];
      const active =
        theme &&
        theme.background.toUpperCase() === appearance.background_color.toUpperCase() &&
        theme.text.toUpperCase() === appearance.text_color.toUpperCase() &&
        theme.accent.toUpperCase() === appearance.accent_color.toUpperCase();
      button.classList.toggle("active", Boolean(active));
    }
  }

  function syncSizeActive() {
    const { width, height } = state.config.window;
    for (const button of el.sizePresets.querySelectorAll("button")) {
      const active =
        Math.abs(Number(button.dataset.width) - width) < 2 &&
        Math.abs(Number(button.dataset.height) - height) < 2;
      button.classList.toggle("active", active);
    }
  }

  function syncForm() {
    suppress = true;
    const { appearance, clock, countdown, pomodoro } = state.config;

    el.fontFamily.value = appearance.font_family;
    el.fontWeight.value = String(appearance.font_weight);
    el.fontSize.value = String(Math.round(appearance.font_size));
    el.fontSizeOut.textContent = `${Math.round(appearance.font_size)}px`;
    el.textColor.value = appearance.text_color;
    el.accentColor.value = appearance.accent_color;
    el.backgroundColor.value = appearance.background_color;
    el.opacity.value = String(Math.round(appearance.background_opacity * 100));
    el.opacityOut.textContent = `${Math.round(appearance.background_opacity * 100)}%`;
    el.alwaysOnTop.checked = appearance.always_on_top;

    el.clock24h.checked = clock.use_24h;
    el.clockSeconds.checked = clock.show_seconds;
    el.clockDate.checked = clock.show_date;

    const duration = splitDuration(countdown.duration_seconds);
    el.countdownH.value = String(duration.hours);
    el.countdownM.value = String(duration.minutes);
    el.countdownS.value = String(duration.seconds);
    el.countdownSound.checked = countdown.sound;
    el.countdownNotify.checked = countdown.notify;

    el.pomoWork.value = String(pomodoro.work_minutes);
    el.pomoShort.value = String(pomodoro.short_break_minutes);
    el.pomoLong.value = String(pomodoro.long_break_minutes);
    el.pomoEvery.value = String(pomodoro.long_break_every);
    el.pomoAuto.checked = pomodoro.auto_start_next;
    el.pomoSound.checked = pomodoro.sound;
    el.pomoNotify.checked = pomodoro.notify;

    syncThemeActive();
    syncSizeActive();
    suppress = false;
  }

  function bind() {
    el.fontFamily.addEventListener("input", () => {
      const value = el.fontFamily.value.trim() || "Cantarell";
      state.config.appearance.font_family = value;
      touch();
    });

    el.fontWeight.addEventListener("change", () => {
      state.config.appearance.font_weight = integer(el.fontWeight.value, 400);
      touch();
    });

    el.fontSize.addEventListener("input", () => {
      const size = integer(el.fontSize.value, 62);
      state.config.appearance.font_size = size;
      el.fontSizeOut.textContent = `${size}px`;
      touch();
    });

    for (const [input, key] of [
      [el.textColor, "text_color"],
      [el.accentColor, "accent_color"],
      [el.backgroundColor, "background_color"],
    ]) {
      input.addEventListener("input", () => {
        state.config.appearance[key] = input.value.toUpperCase();
        touch();
        syncThemeActive();
      });
    }

    el.opacity.addEventListener("input", () => {
      const percent = integer(el.opacity.value, 72);
      state.config.appearance.background_opacity = percent / 100;
      el.opacityOut.textContent = `${percent}%`;
      touch();
    });

    el.alwaysOnTop.addEventListener("change", async () => {
      const enabled = el.alwaysOnTop.checked;
      state.config.appearance.always_on_top = enabled;
      try {
        const view = await invoke("set_always_on_top", { enabled });
        ctx.applyView(view, { syncForm: false });
        ctx.updateMenuState();
      } catch (error) {
        el.alwaysOnTop.checked = !enabled;
        ctx.showToast("Could not change window layer", String(error), true);
      }
    });

    el.clock24h.addEventListener("change", () => {
      state.config.clock.use_24h = el.clock24h.checked;
      touch({ render: true });
    });
    el.clockSeconds.addEventListener("change", () => {
      state.config.clock.show_seconds = el.clockSeconds.checked;
      touch({ render: true });
    });
    el.clockDate.addEventListener("change", () => {
      state.config.clock.show_date = el.clockDate.checked;
      touch({ render: true });
    });

    const countdownChanged = () => {
      const seconds =
        integer(el.countdownH.value) * 3600 +
        integer(el.countdownM.value) * 60 +
        integer(el.countdownS.value);
      state.config.countdown.duration_seconds = clamp(seconds || 1, 1, 24 * 3600);
      touch();
    };
    for (const input of [el.countdownH, el.countdownM, el.countdownS]) {
      input.addEventListener("input", countdownChanged);
      input.addEventListener("change", countdownChanged);
    }
    el.countdownSound.addEventListener("change", () => {
      state.config.countdown.sound = el.countdownSound.checked;
      touch();
    });
    el.countdownNotify.addEventListener("change", () => {
      state.config.countdown.notify = el.countdownNotify.checked;
      touch();
    });

    const pomodoroChanged = () => {
      const p = state.config.pomodoro;
      p.work_minutes = clamp(integer(el.pomoWork.value, 25), 1, 180);
      p.short_break_minutes = clamp(integer(el.pomoShort.value, 5), 1, 180);
      p.long_break_minutes = clamp(integer(el.pomoLong.value, 15), 1, 180);
      p.long_break_every = clamp(integer(el.pomoEvery.value, 4), 1, 12);
      touch();
    };
    for (const input of [el.pomoWork, el.pomoShort, el.pomoLong, el.pomoEvery]) {
      input.addEventListener("input", pomodoroChanged);
      input.addEventListener("change", () => {
        pomodoroChanged();
        syncForm();
      });
    }
    el.pomoAuto.addEventListener("change", () => {
      state.config.pomodoro.auto_start_next = el.pomoAuto.checked;
      touch();
    });
    el.pomoSound.addEventListener("change", () => {
      state.config.pomodoro.sound = el.pomoSound.checked;
      touch();
    });
    el.pomoNotify.addEventListener("change", () => {
      state.config.pomodoro.notify = el.pomoNotify.checked;
      touch();
    });

    el.themes.addEventListener("click", (event) => {
      const button = event.target.closest("[data-theme]");
      if (!button) return;
      const theme = THEMES[button.dataset.theme];
      if (!theme) return;
      Object.assign(state.config.appearance, {
        background_color: theme.background,
        text_color: theme.text,
        accent_color: theme.accent,
      });
      el.backgroundColor.value = theme.background;
      el.textColor.value = theme.text;
      el.accentColor.value = theme.accent;
      syncThemeActive();
      touch({ render: true });
    });

    el.sizePresets.addEventListener("click", async (event) => {
      const button = event.target.closest("button[data-width]");
      if (!button) return;
      const width = Number(button.dataset.width);
      const height = Number(button.dataset.height);
      try {
        const config = await invoke("set_window_dimensions", { width, height });
        state.config.window = config.window;
        syncSizeActive();
        setSaveState(isOpen ? "Size saved · applies when closed" : "Size saved");
      } catch (error) {
        ctx.showToast("Could not resize window", String(error), true);
      }
    });

    el.reset.addEventListener("click", async () => {
      try {
        clearTimeout(saveTimer);
        saveTimer = null;
        const view = await invoke("reset_config");
        ctx.applyView(view, { syncForm: true });
        syncForm();
        setSaveState("Defaults restored");
      } catch (error) {
        ctx.showToast("Could not reset settings", String(error), true);
      }
    });
  }

  async function open() {
    if (isOpen) return;
    isOpen = true;
    syncForm();
    setSaveState(lastSavedAt ? "Saved" : "Ready");
    sheet.hidden = false;
    app.classList.add("settings-open");
    try {
      await invoke("set_settings_open", { open: true });
    } catch (error) {
      ctx.showToast("Could not open preferences", String(error), true);
    }
  }

  async function close() {
    if (!isOpen) return;
    await flushNow();
    isOpen = false;
    sheet.hidden = true;
    app.classList.remove("settings-open");
    try {
      await invoke("set_settings_open", { open: false });
    } catch (error) {
      ctx.showToast("Could not resize window", String(error), true);
    }
  }

  bind();

  return {
    open,
    close,
    syncForm,
    flushNow,
    get isOpen() {
      return isOpen;
    },
  };
}
