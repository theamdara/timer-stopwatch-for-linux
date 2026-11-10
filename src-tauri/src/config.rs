//! Persistent, human-editable JSON configuration.
//!
//! The file lives at `~/.config/catime/config.json` on Linux. It is written
//! atomically and every value is clamped to a sane range when loaded so a
//! hand-edited file can never leave the window invisible or unusable.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_VERSION: u32 = 1;
pub const DEFAULT_FONT_FAMILY: &str = "Cantarell";
pub const DEFAULT_TEXT_COLOR: &str = "#F5F7FF";
pub const DEFAULT_BACKGROUND_COLOR: &str = "#101320";
pub const DEFAULT_ACCENT_COLOR: &str = "#8AB4FF";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub appearance: Appearance,
    pub clock: ClockConfig,
    pub countdown: CountdownConfig,
    pub pomodoro: PomodoroConfig,
    pub window: WindowConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            appearance: Appearance::default(),
            clock: ClockConfig::default(),
            countdown: CountdownConfig::default(),
            pomodoro: PomodoroConfig::default(),
            window: WindowConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    /// A single CSS font family. Quoted automatically by the UI.
    pub font_family: String,
    pub font_size: f64,
    pub font_weight: u16,
    pub text_color: String,
    pub background_color: String,
    pub accent_color: String,
    /// 0.0 = fully transparent, 1.0 = opaque.
    pub background_opacity: f64,
    pub always_on_top: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            font_family: DEFAULT_FONT_FAMILY.to_string(),
            font_size: 62.0,
            font_weight: 300,
            text_color: DEFAULT_TEXT_COLOR.to_string(),
            background_color: DEFAULT_BACKGROUND_COLOR.to_string(),
            accent_color: DEFAULT_ACCENT_COLOR.to_string(),
            background_opacity: 0.72,
            always_on_top: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockConfig {
    /// `true` for 24-hour time, `false` for 12-hour time with AM/PM.
    pub use_24h: bool,
    pub show_seconds: bool,
    pub show_date: bool,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            use_24h: true,
            show_seconds: true,
            show_date: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CountdownConfig {
    pub duration_seconds: u64,
    pub sound: bool,
    pub notify: bool,
}

impl Default for CountdownConfig {
    fn default() -> Self {
        Self {
            duration_seconds: 5 * 60,
            sound: true,
            notify: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PomodoroConfig {
    pub work_minutes: u64,
    pub short_break_minutes: u64,
    pub long_break_minutes: u64,
    pub long_break_every: u32,
    pub auto_start_next: bool,
    pub sound: bool,
    pub notify: bool,
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self {
            work_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            long_break_every: 4,
            auto_start_next: true,
            sound: true,
            notify: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    /// Logical inner size in CSS pixels.
    pub width: f64,
    pub height: f64,
    /// Physical screen position. `None` means "let the compositor place it".
    pub x: Option<i32>,
    pub y: Option<i32>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 440.0,
            height: 240.0,
            x: None,
            y: None,
        }
    }
}

impl Config {
    /// The canonical configuration location required by Catime:
    /// `~/.config/catime/config.json`.
    pub fn default_path() -> PathBuf {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        home.join(".config").join("catime").join("config.json")
    }

    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(contents) => match serde_json::from_str::<Config>(&contents) {
                Ok(mut config) => {
                    config.validate();
                    config
                }
                Err(error) => {
                    eprintln!("[catime] invalid config at {}: {error}", path.display());
                    // Keep the broken file around so the user can inspect it.
                    let backup = path.with_extension("json.invalid");
                    let _ = fs::rename(path, &backup);
                    let mut config = Config::default();
                    config.validate();
                    config
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut config = Config::default();
                config.validate();
                config
            }
            Err(error) => {
                eprintln!("[catime] could not read config: {error}");
                let mut config = Config::default();
                config.validate();
                config
            }
        }
    }

    /// Write pretty, deterministic JSON via a temporary file + rename.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }

        let mut pretty = serde_json::to_string_pretty(self)
            .map_err(|error| format!("could not serialize config: {error}"))?;
        pretty.push('\n');

        let temp = path.with_extension("json.tmp");
        fs::write(&temp, pretty)
            .map_err(|error| format!("could not write {}: {error}", temp.display()))?;
        fs::rename(&temp, path)
            .map_err(|error| format!("could not replace {}: {error}", path.display()))?;
        Ok(())
    }

    pub fn validate(&mut self) {
        self.version = CONFIG_VERSION;

        let a = &mut self.appearance;
        a.font_family = sanitize_font_family(&a.font_family);
        if a.font_family.is_empty() {
            a.font_family = DEFAULT_FONT_FAMILY.to_string();
        }
        a.font_size = a.font_size.clamp(20.0, 200.0);
        a.font_weight = [300u16, 400, 500, 600, 700]
            .into_iter()
            .min_by_key(|weight| weight.abs_diff(a.font_weight))
            .unwrap_or(400);
        a.text_color = normalize_color(&a.text_color, DEFAULT_TEXT_COLOR);
        a.background_color = normalize_color(&a.background_color, DEFAULT_BACKGROUND_COLOR);
        a.accent_color = normalize_color(&a.accent_color, DEFAULT_ACCENT_COLOR);
        a.background_opacity = a.background_opacity.clamp(0.0, 1.0);

        self.countdown.duration_seconds = self.countdown.duration_seconds.clamp(1, 24 * 60 * 60);

        let p = &mut self.pomodoro;
        p.work_minutes = p.work_minutes.clamp(1, 180);
        p.short_break_minutes = p.short_break_minutes.clamp(1, 180);
        p.long_break_minutes = p.long_break_minutes.clamp(1, 180);
        p.long_break_every = p.long_break_every.clamp(1, 12);

        self.window.width = self.window.width.clamp(300.0, 980.0);
        self.window.height = self.window.height.clamp(180.0, 720.0);
    }
}

fn sanitize_font_family(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .filter(|c| !c.is_control())
        .filter(|c| !matches!(c, '"' | '\'' | ';' | '{' | '}' | '<' | '>' | '\\'))
        .collect();
    cleaned.trim().chars().take(80).collect()
}

fn normalize_color(value: &str, fallback: &str) -> String {
    let candidate = value.trim();
    let valid = candidate.len() == 7
        && candidate.starts_with('#')
        && candidate.chars().skip(1).all(|c| c.is_ascii_hexdigit());

    if valid {
        candidate.to_ascii_uppercase()
    } else {
        fallback.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_dangerous_values() {
        let mut config = Config::default();
        config.appearance.background_opacity = 9.0;
        config.appearance.font_size = 2.0;
        config.countdown.duration_seconds = 0;
        config.window.width = 1.0;
        config.appearance.text_color = "not-a-color".into();
        config.validate();

        assert_eq!(config.appearance.background_opacity, 1.0);
        assert_eq!(config.appearance.font_size, 20.0);
        assert_eq!(config.countdown.duration_seconds, 1);
        assert_eq!(config.window.width, 300.0);
        assert_eq!(config.appearance.text_color, DEFAULT_TEXT_COLOR);
    }

    #[test]
    fn default_path_is_under_dot_config() {
        let path = Config::default_path();
        assert!(path.ends_with(Path::new(".config/catime/config.json")));
    }
}
