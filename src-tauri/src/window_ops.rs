//! Window helpers: initial placement, always-on-top, and the settings sheet
//! resize. Keeping these in one module makes the command layer easier to read.

use crate::config::Config;
use tauri::{LogicalSize, PhysicalPosition, WebviewWindow};

const SETTINGS_MIN_WIDTH: f64 = 400.0;
const SETTINGS_MAX_HEIGHT: f64 = 660.0;
const SCREEN_MARGIN: f64 = 16.0;

pub fn apply_initial(window: &WebviewWindow, config: &Config) {
    apply_always_on_top(window, config.appearance.always_on_top);

    let _ = window.set_size(LogicalSize::new(
        config.window.width,
        config.window.height,
    ));

    if let (Some(x), Some(y)) = (config.window.x, config.window.y) {
        if position_is_visible(window, x, y) {
            let _ = window.set_position(PhysicalPosition::new(x, y));
        } else {
            let _ = window.center();
        }
    }
}

pub fn apply_always_on_top(window: &WebviewWindow, enabled: bool) {
    if let Err(error) = window.set_always_on_top(enabled) {
        eprintln!("[catime] could not set always-on-top: {error}");
    }
}

/// Grow the window into a comfortable preferences sheet and return to the
/// user's chosen compact size when done.
pub fn resize_for_settings(window: &WebviewWindow, config: &Config, open: bool) {
    if !open {
        let _ = window.set_size(LogicalSize::new(
            config.window.width,
            config.window.height,
        ));
        return;
    }

    let mut width = config.window.width.max(SETTINGS_MIN_WIDTH);
    let mut height = SETTINGS_MAX_HEIGHT;

    if let Ok(Some(monitor)) = window.current_monitor() {
        let scale = monitor.scale_factor();
        let monitor_size = monitor.size().to_logical::<f64>(scale);
        let monitor_pos = monitor.position();

        width = width.min((monitor_size.width - SCREEN_MARGIN * 2.0).max(320.0));
        height = height.min(
            (monitor_size.height - SCREEN_MARGIN * 2.0 - 40.0).max(320.0),
        );

        let _ = window.set_size(LogicalSize::new(width, height));

        if let Ok(position) = window.outer_position() {
            let physical_width = (width * scale).round() as i32;
            let physical_height = (height * scale).round() as i32;
            let max_x = monitor_pos.x + monitor_size.width as i32 - physical_width
                - (SCREEN_MARGIN * scale) as i32;
            let max_y = monitor_pos.y + monitor_size.height as i32 - physical_height
                - (SCREEN_MARGIN * scale) as i32;
            let min_x = monitor_pos.x + (SCREEN_MARGIN * scale) as i32;
            let min_y = monitor_pos.y + (SCREEN_MARGIN * scale) as i32;

            let x = position.x.clamp(min_x, max_x.max(min_x));
            let y = position.y.clamp(min_y, max_y.max(min_y));
            if x != position.x || y != position.y {
                let _ = window.set_position(PhysicalPosition::new(x, y));
            }
        }
    } else {
        let _ = window.set_size(LogicalSize::new(width, height));
    }
}

fn position_is_visible(window: &WebviewWindow, x: i32, y: i32) -> bool {
    match window.available_monitors() {
        Ok(monitors) => monitors.iter().any(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            let right = position.x + size.width as i32;
            let bottom = position.y + size.height as i32;
            x + 40 >= position.x && x + 40 < right && y + 40 >= position.y && y + 40 < bottom
        }),
        // If the window manager cannot enumerate monitors, trust the position.
        Err(_) => true,
    }
}
