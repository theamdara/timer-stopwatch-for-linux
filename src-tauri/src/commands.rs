//! Tauri commands exposed to the small vanilla-JS frontend.

use crate::config::Config;
use crate::timer::{Mode, Snapshot, TimerAction};
use crate::window_ops;
use crate::{AppState, SharedState};
use serde::Serialize;
use std::sync::MutexGuard;
use std::time::Instant;
use tauri::{AppHandle, Manager, State, WebviewWindow};

#[derive(Serialize)]
pub struct Bootstrap {
    pub config: Config,
    pub state: Snapshot,
    pub version: String,
    pub config_path: String,
}

#[derive(Serialize)]
pub struct AppView {
    pub config: Config,
    pub state: Snapshot,
}

fn guard<'a>(state: &'a State<'_, SharedState>) -> Result<MutexGuard<'a, AppState>, String> {
    state
        .lock()
        .map_err(|_| "Application state is unavailable.".to_string())
}

fn view(state: &mut AppState) -> AppView {
    AppView {
        config: state.config.clone(),
        state: state.timer.snapshot(Instant::now()),
    }
}

#[tauri::command]
pub fn get_bootstrap(state: State<'_, SharedState>) -> Result<Bootstrap, String> {
    let state = guard(&state)?;
    Ok(Bootstrap {
        config: state.config.clone(),
        state: state.timer.snapshot(Instant::now()),
        version: env!("CARGO_PKG_VERSION").to_string(),
        config_path: state.config_path.display().to_string(),
    })
}

#[tauri::command]
pub fn save_config(
    app: AppHandle,
    state: State<'_, SharedState>,
    mut config: Config,
) -> Result<AppView, String> {
    config.validate();
    let mut state = guard(&state)?;
    let old = state.config.clone();

    // Window geometry is owned by the Rust side so a settings save can never
    // clobber a resize/move that happened at the same moment.
    config.window = state.config.window.clone();
    state.timer.sync_config(&old, &config);
    state.config = config;
    state.persist();

    let result = view(&mut state);
    let always_on_top = result.config.appearance.always_on_top;
    drop(state);

    if old.appearance.always_on_top != always_on_top {
        if let Some(window) = app.get_webview_window("main") {
            window_ops::apply_always_on_top(&window, always_on_top);
        }
    }
    Ok(result)
}

#[tauri::command]
pub fn reset_config(app: AppHandle, state: State<'_, SharedState>) -> Result<AppView, String> {
    let mut state = guard(&state)?;
    let old = state.config.clone();
    let mut config = Config::default();
    config.validate();
    config.window = old.window.clone();
    state.timer.sync_config(&old, &config);
    state.config = config;
    state.persist();

    let result = view(&mut state);
    drop(state);

    if let Some(window) = app.get_webview_window("main") {
        window_ops::apply_always_on_top(&window, result.config.appearance.always_on_top);
    }
    Ok(result)
}

#[tauri::command]
pub fn set_mode(state: State<'_, SharedState>, mode: Mode) -> Result<AppView, String> {
    let mut state = guard(&state)?;
    state.timer.set_mode(mode, Instant::now());
    Ok(view(&mut state))
}

#[tauri::command]
pub fn timer_action(state: State<'_, SharedState>, action: TimerAction) -> Result<AppView, String> {
    let mut state = guard(&state)?;
    state.timer.perform(action, Instant::now());
    Ok(view(&mut state))
}

#[tauri::command]
pub fn set_countdown(
    state: State<'_, SharedState>,
    seconds: u64,
) -> Result<AppView, String> {
    let mut state = guard(&state)?;
    let seconds = seconds.clamp(1, 24 * 60 * 60);
    state.config.countdown.duration_seconds = seconds;
    state.timer.set_countdown_duration(seconds);
    state.persist();
    Ok(view(&mut state))
}

#[tauri::command]
pub fn set_always_on_top(
    app: AppHandle,
    state: State<'_, SharedState>,
    enabled: bool,
) -> Result<AppView, String> {
    let mut state = guard(&state)?;
    state.config.appearance.always_on_top = enabled;
    state.persist();
    let result = view(&mut state);
    drop(state);

    if let Some(window) = app.get_webview_window("main") {
        window_ops::apply_always_on_top(&window, enabled);
    }
    Ok(result)
}

/// Change the normal (non-settings) window size. While the settings sheet is
/// open we only persist the new base size; it is applied when settings close.
#[tauri::command]
pub fn set_window_dimensions(
    state: State<'_, SharedState>,
    window: WebviewWindow,
    width: f64,
    height: f64,
) -> Result<Config, String> {
    let width = width.clamp(300.0, 980.0);
    let height = height.clamp(180.0, 720.0);

    let mut state = guard(&state)?;
    state.config.window.width = width;
    state.config.window.height = height;
    state.persist();

    if !state.expanded {
        window
            .set_size(tauri::LogicalSize::new(width, height))
            .map_err(|error| error.to_string())?;
        state.geometry_dirty_at = None;
    }

    Ok(state.config.clone())
}

#[tauri::command]
pub fn set_settings_open(
    state: State<'_, SharedState>,
    window: WebviewWindow,
    open: bool,
) -> Result<(), String> {
    let mut state = guard(&state)?;
    if state.expanded == open {
        return Ok(());
    }
    state.expanded = open;
    let config = state.config.clone();
    drop(state);

    window_ops::resize_for_settings(&window, &config, open);
    Ok(())
}

#[tauri::command]
pub fn start_dragging(window: WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn start_resize(window: tauri::Window) -> Result<(), String> {
    window
        .start_resize_dragging(tauri_runtime::ResizeDirection::SouthEast)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn minimize_window(window: WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn close_window(window: WebviewWindow) -> Result<(), String> {
    window.close().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn quit_app(app: AppHandle, state: State<'_, SharedState>) {
    if let Ok(state) = state.lock() {
        state.persist();
    }
    app.exit(0);
}
