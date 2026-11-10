// Catime — a lightweight floating clock, countdown, stopwatch and pomodoro.
// No tray icon, no background service: closing the window exits the process.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod runtime;
mod timer;
mod window_ops;

use config::Config;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{Manager, WindowEvent};
use timer::Timer;

pub struct AppState {
    pub config: Config,
    pub timer: Timer,
    pub config_path: PathBuf,
    /// True while the preferences sheet (taller window) is visible.
    pub expanded: bool,
    /// Debounced window-geometry save marker.
    pub geometry_dirty_at: Option<Instant>,
}

impl AppState {
    /// Persist the current config file, logging rather than crashing if the
    /// disk is read-only.
    pub fn persist(&self) {
        if let Err(error) = self.config.save(&self.config_path) {
            eprintln!("[catime] could not save config: {error}");
        }
    }
}

pub type SharedState = Mutex<AppState>;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let config_path = Config::default_path();
            let config = Config::load(&config_path);
            // Create the file on first launch so it is discoverable/editable.
            let _ = config.save(&config_path);

            let timer = Timer::new(&config);
            let state = AppState {
                config: config.clone(),
                timer,
                config_path,
                expanded: false,
                geometry_dirty_at: None,
            };
            app.manage(Mutex::new(state));

            if let Some(window) = app.get_webview_window("main") {
                window_ops::apply_initial(&window, &config);
                // The window starts hidden to avoid a white/transparent flash.
                window.show()?;
                window.set_focus().ok();
            }

            runtime::spawn_ticker(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::save_config,
            commands::reset_config,
            commands::set_mode,
            commands::timer_action,
            commands::set_countdown,
            commands::set_always_on_top,
            commands::set_window_dimensions,
            commands::set_settings_open,
            commands::start_dragging,
            commands::start_resize,
            commands::minimize_window,
            commands::close_window,
            commands::quit_app,
        ])
        .on_window_event(handle_window_event)
        .run(tauri::generate_context!())
        .expect("error while running Catime");
}

fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    match event {
        WindowEvent::Resized(_) => {
            let app_state = window.app_handle().state::<SharedState>();
            let Ok(mut state) = app_state.lock() else {
                return;
            };
            if state.expanded {
                return;
            }
            if let Ok(size) = window.inner_size() {
                let scale = window.scale_factor().unwrap_or(1.0);
                let logical = size.to_logical::<f64>(scale);
                if logical.width >= 200.0 && logical.height >= 100.0 {
                    state.config.window.width = logical.width;
                    state.config.window.height = logical.height;
                    state.geometry_dirty_at = Some(Instant::now());
                }
            }
        }
        WindowEvent::Moved(_) => {
            let app_state = window.app_handle().state::<SharedState>();
            let Ok(mut state) = app_state.lock() else {
                return;
            };
            if let Ok(position) = window.outer_position() {
                state.config.window.x = Some(position.x);
                state.config.window.y = Some(position.y);
                state.geometry_dirty_at = Some(Instant::now());
            }
        }
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let app_state = window.app_handle().state::<SharedState>();
            if let Ok(state) = app_state.lock() {
                state.persist();
            }
            // There is no tray process to keep alive: quit immediately.
            window.app_handle().exit(0);
        }
        _ => {}
    }
}
