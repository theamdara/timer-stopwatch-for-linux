//! All timer math lives here. Every operation is driven by `std::time::Instant`
//! rather than a counter that increments once per tick, so the display never
//! drifts, even if the UI thread is briefly busy or the compositor throttles.

use crate::config::{Config, PomodoroConfig};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Clock,
    Countdown,
    Stopwatch,
    Pomodoro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PomodoroPhase {
    Work,
    ShortBreak,
    LongBreak,
}

impl PomodoroPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::ShortBreak => "short_break",
            Self::LongBreak => "long_break",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimerAction {
    Start,
    Pause,
    Toggle,
    Reset,
    Skip,
}

#[derive(Debug, Clone, Serialize)]
pub struct CountdownSnapshot {
    pub duration_ms: u64,
    pub remaining_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StopwatchSnapshot {
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PomodoroSnapshot {
    pub phase: PomodoroPhase,
    pub remaining_ms: u64,
    pub completed_work: u32,
    pub cycle_position: u32,
    pub long_break_every: u32,
    pub auto_start_next: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub mode: Mode,
    pub running: bool,
    pub now_ms: u64,
    pub countdown: CountdownSnapshot,
    pub stopwatch: StopwatchSnapshot,
    pub pomodoro: PomodoroSnapshot,
}

/// Emitted once when a countdown or pomodoro phase ends.
#[derive(Debug, Clone, Serialize)]
pub struct FinishedEvent {
    pub kind: &'static str,
    pub title: String,
    pub body: String,
    pub sound: bool,
    pub notify: bool,
    pub completed_phase: Option<&'static str>,
    pub next_phase: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct Timer {
    mode: Mode,
    countdown_duration: Duration,
    countdown_remaining: Duration,
    countdown_deadline: Option<Instant>,
    stopwatch_elapsed: Duration,
    stopwatch_started: Option<Instant>,
    pomodoro_phase: PomodoroPhase,
    pomodoro_remaining: Duration,
    pomodoro_deadline: Option<Instant>,
    pomodoro_completed_work: u32,
    pomodoro_config: PomodoroConfig,
}

impl Timer {
    pub fn new(config: &Config) -> Self {
        let countdown_duration = Duration::from_secs(config.countdown.duration_seconds);
        let pomodoro_config = config.pomodoro.clone();
        Self {
            mode: Mode::Clock,
            countdown_duration,
            countdown_remaining: countdown_duration,
            countdown_deadline: None,
            stopwatch_elapsed: Duration::ZERO,
            stopwatch_started: None,
            pomodoro_phase: PomodoroPhase::Work,
            pomodoro_remaining: Duration::from_secs(pomodoro_config.work_minutes * 60),
            pomodoro_deadline: None,
            pomodoro_completed_work: 0,
            pomodoro_config,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn is_running(&self) -> bool {
        match self.mode {
            Mode::Countdown => self.countdown_deadline.is_some(),
            Mode::Stopwatch => self.stopwatch_started.is_some(),
            Mode::Pomodoro => self.pomodoro_deadline.is_some(),
            Mode::Clock => false,
        }
    }

    /// Applies a full config change, but only touches timer state when the
    /// relevant duration settings actually changed.
    pub fn sync_config(&mut self, old: &Config, new: &Config) {
        if old.countdown.duration_seconds != new.countdown.duration_seconds {
            self.countdown_duration = Duration::from_secs(new.countdown.duration_seconds);
            if self.countdown_deadline.is_none() {
                self.countdown_remaining = self.countdown_duration;
            }
        }

        let pomodoro_changed = old.pomodoro.work_minutes != new.pomodoro.work_minutes
            || old.pomodoro.short_break_minutes != new.pomodoro.short_break_minutes
            || old.pomodoro.long_break_minutes != new.pomodoro.long_break_minutes
            || old.pomodoro.long_break_every != new.pomodoro.long_break_every;

        if pomodoro_changed {
            self.pomodoro_config = new.pomodoro.clone();
            if self.pomodoro_deadline.is_none() {
                self.pomodoro_remaining = self.pomodoro_phase_duration();
            }
        } else {
            // Keep booleans (auto start, sound, notify) in sync.
            self.pomodoro_config = new.pomodoro.clone();
        }
    }

    pub fn set_mode(&mut self, mode: Mode, now: Instant) {
        if self.mode == mode {
            return;
        }
        self.pause(now);
        self.mode = mode;
    }

    pub fn set_countdown_duration(&mut self, seconds: u64) {
        self.countdown_duration = Duration::from_secs(seconds.max(1));
        self.countdown_remaining = self.countdown_duration;
        self.countdown_deadline = None;
    }

    pub fn set_stopwatch_zero(&mut self) {
        self.stopwatch_elapsed = Duration::ZERO;
        self.stopwatch_started = None;
    }

    pub fn start(&mut self, now: Instant) {
        match self.mode {
            Mode::Countdown => {
                if self.countdown_deadline.is_none() {
                    if self.countdown_remaining.is_zero() {
                        self.countdown_remaining = self.countdown_duration;
                    }
                    self.countdown_deadline = Some(now + self.countdown_remaining);
                }
            }
            Mode::Stopwatch => {
                if self.stopwatch_started.is_none() {
                    self.stopwatch_started = Some(now);
                }
            }
            Mode::Pomodoro => {
                if self.pomodoro_deadline.is_none() {
                    if self.pomodoro_remaining.is_zero() {
                        self.pomodoro_remaining = self.pomodoro_phase_duration();
                    }
                    self.pomodoro_deadline = Some(now + self.pomodoro_remaining);
                }
            }
            Mode::Clock => {}
        }
    }

    pub fn pause(&mut self, now: Instant) {
        match self.mode {
            Mode::Countdown => {
                if let Some(deadline) = self.countdown_deadline.take() {
                    self.countdown_remaining = deadline.saturating_duration_since(now);
                }
            }
            Mode::Stopwatch => {
                if let Some(started) = self.stopwatch_started.take() {
                    self.stopwatch_elapsed += now.saturating_duration_since(started);
                }
            }
            Mode::Pomodoro => {
                if let Some(deadline) = self.pomodoro_deadline.take() {
                    self.pomodoro_remaining = deadline.saturating_duration_since(now);
                }
            }
            Mode::Clock => {}
        }
    }

    pub fn toggle(&mut self, now: Instant) {
        if self.is_running() {
            self.pause(now);
        } else {
            self.start(now);
        }
    }

    pub fn reset(&mut self, _now: Instant) {
        match self.mode {
            Mode::Countdown => {
                self.countdown_deadline = None;
                self.countdown_remaining = self.countdown_duration;
            }
            Mode::Stopwatch => self.set_stopwatch_zero(),
            Mode::Pomodoro => {
                self.pomodoro_deadline = None;
                self.pomodoro_phase = PomodoroPhase::Work;
                self.pomodoro_remaining = self.pomodoro_phase_duration();
                self.pomodoro_completed_work = 0;
            }
            Mode::Clock => {}
        }
    }

    /// Pomodoro-only: move to the next phase without firing an alert.
    /// A skipped work phase still counts toward the long-break cadence.
    pub fn skip(&mut self, now: Instant) {
        if self.mode != Mode::Pomodoro {
            return;
        }
        let was_running = self.pomodoro_deadline.is_some();
        self.advance_pomodoro(now, was_running);
    }

    pub fn perform(&mut self, action: TimerAction, now: Instant) {
        match action {
            TimerAction::Start => self.start(now),
            TimerAction::Pause => self.pause(now),
            TimerAction::Toggle => self.toggle(now),
            TimerAction::Reset => self.reset(now),
            TimerAction::Skip => self.skip(now),
        }
    }

    /// Advance real time. Returns an event exactly once when a timer finishes.
    pub fn tick(&mut self, now: Instant, config: &Config) -> Option<FinishedEvent> {
        match self.mode {
            Mode::Countdown => {
                let deadline = self.countdown_deadline?;
                if now < deadline {
                    return None;
                }
                self.countdown_deadline = None;
                self.countdown_remaining = Duration::ZERO;
                Some(FinishedEvent {
                    kind: "countdown",
                    title: "Countdown complete".to_string(),
                    body: "Time is up.".to_string(),
                    sound: config.countdown.sound,
                    notify: config.countdown.notify,
                    completed_phase: None,
                    next_phase: None,
                })
            }
            Mode::Pomodoro => {
                let deadline = self.pomodoro_deadline?;
                if now < deadline {
                    return None;
                }
                self.pomodoro_deadline = None;
                let completed = self.pomodoro_phase;
                let auto_start = self.pomodoro_config.auto_start_next;
                self.advance_pomodoro(now, auto_start);
                let next = self.pomodoro_phase;

                let (title, body) = match (completed, next) {
                    (PomodoroPhase::Work, PomodoroPhase::LongBreak) => (
                        "Focus session complete",
                        format!(
                            "Great work. Enjoy a {} minute long break.",
                            self.pomodoro_config.long_break_minutes
                        ),
                    ),
                    (PomodoroPhase::Work, _) => (
                        "Focus session complete",
                        format!(
                            "Nice work. Take a {} minute break.",
                            self.pomodoro_config.short_break_minutes
                        ),
                    ),
                    (PomodoroPhase::ShortBreak, _) => (
                        "Break over",
                        "Ready for the next focus session?".to_string(),
                    ),
                    (PomodoroPhase::LongBreak, _) => (
                        "Long break over",
                        "New cycle, fresh focus.".to_string(),
                    ),
                };

                Some(FinishedEvent {
                    kind: "pomodoro",
                    title: title.to_string(),
                    body,
                    sound: self.pomodoro_config.sound,
                    notify: self.pomodoro_config.notify,
                    completed_phase: Some(completed.as_str()),
                    next_phase: Some(next.as_str()),
                })
            }
            Mode::Clock | Mode::Stopwatch => None,
        }
    }

    pub fn snapshot(&self, now: Instant) -> Snapshot {
        let countdown_remaining = match self.countdown_deadline {
            Some(deadline) => deadline.saturating_duration_since(now),
            None => self.countdown_remaining,
        };
        let stopwatch_elapsed = match self.stopwatch_started {
            Some(started) => self.stopwatch_elapsed + now.saturating_duration_since(started),
            None => self.stopwatch_elapsed,
        };
        let pomodoro_remaining = match self.pomodoro_deadline {
            Some(deadline) => deadline.saturating_duration_since(now),
            None => self.pomodoro_remaining,
        };
        let every = self.pomodoro_config.long_break_every.max(1);
        let cycle_position = if self.pomodoro_phase == PomodoroPhase::LongBreak {
            every
        } else {
            self.pomodoro_completed_work % every
        };

        Snapshot {
            mode: self.mode,
            running: self.is_running(),
            now_ms: unix_millis(),
            countdown: CountdownSnapshot {
                duration_ms: duration_millis(self.countdown_duration),
                remaining_ms: duration_millis(countdown_remaining),
            },
            stopwatch: StopwatchSnapshot {
                elapsed_ms: duration_millis(stopwatch_elapsed),
            },
            pomodoro: PomodoroSnapshot {
                phase: self.pomodoro_phase,
                remaining_ms: duration_millis(pomodoro_remaining),
                completed_work: self.pomodoro_completed_work,
                cycle_position,
                long_break_every: every,
                auto_start_next: self.pomodoro_config.auto_start_next,
            },
        }
    }

    /// Advance to the next pomodoro phase, optionally starting it immediately.
    fn advance_pomodoro(&mut self, now: Instant, auto_start: bool) {
        match self.pomodoro_phase {
            PomodoroPhase::Work => {
                self.pomodoro_completed_work = self.pomodoro_completed_work.saturating_add(1);
                let every = self.pomodoro_config.long_break_every.max(1);
                self.pomodoro_phase = if self.pomodoro_completed_work % every == 0 {
                    PomodoroPhase::LongBreak
                } else {
                    PomodoroPhase::ShortBreak
                };
            }
            PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => {
                self.pomodoro_phase = PomodoroPhase::Work;
            }
        }

        self.pomodoro_remaining = self.pomodoro_phase_duration();
        self.pomodoro_deadline = if auto_start {
            Some(now + self.pomodoro_remaining)
        } else {
            None
        };
    }

    fn pomodoro_phase_duration(&self) -> Duration {
        let minutes = match self.pomodoro_phase {
            PomodoroPhase::Work => self.pomodoro_config.work_minutes,
            PomodoroPhase::ShortBreak => self.pomodoro_config.short_break_minutes,
            PomodoroPhase::LongBreak => self.pomodoro_config.long_break_minutes,
        };
        Duration::from_secs(minutes.max(1) * 60)
    }
}

fn duration_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(duration_millis)
        .unwrap_or(0)
}

/// Short, stable signature used by the ticker to avoid emitting redundant IPC
/// messages while the display would not visibly change.
pub fn snapshot_signature(snapshot: &Snapshot) -> String {
    match snapshot.mode {
        Mode::Clock => format!("clock:{}", snapshot.now_ms / 1000),
        Mode::Countdown => format!(
            "countdown:{}:{}",
            snapshot.countdown.remaining_ms / 1000,
            snapshot.running
        ),
        Mode::Stopwatch => format!(
            "stopwatch:{}",
            snapshot.stopwatch.elapsed_ms / 10
        ),
        Mode::Pomodoro => format!(
            "pomodoro:{:?}:{}:{}:{}",
            snapshot.pomodoro.phase,
            snapshot.pomodoro.remaining_ms / 1000,
            snapshot.pomodoro.cycle_position,
            snapshot.running
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config::default()
    }

    #[test]
    fn countdown_completes_once_and_can_restart() {
        let config = config();
        let mut timer = Timer::new(&config);
        timer.set_mode(Mode::Countdown, Instant::now());
        timer.set_countdown_duration(2);
        let t0 = Instant::now();
        timer.start(t0);

        assert!(timer.tick(t0 + Duration::from_secs(1), &config).is_none());
        let finished = timer.tick(t0 + Duration::from_secs(2), &config).unwrap();
        assert_eq!(finished.kind, "countdown");
        assert!(timer.tick(t0 + Duration::from_secs(3), &config).is_none());
        assert_eq!(timer.snapshot(t0 + Duration::from_secs(3)).countdown.remaining_ms, 0);

        timer.start(t0 + Duration::from_secs(4));
        assert!(timer.is_running());
        assert_eq!(
            timer.snapshot(t0 + Duration::from_secs(4)).countdown.remaining_ms,
            2000
        );
    }

    #[test]
    fn stopwatch_accumulates_across_pauses() {
        let mut timer = Timer::new(&config());
        timer.set_mode(Mode::Stopwatch, Instant::now());
        let t0 = Instant::now();
        timer.start(t0);
        timer.pause(t0 + Duration::from_millis(1500));
        timer.start(t0 + Duration::from_millis(2500));
        let snap = timer.snapshot(t0 + Duration::from_millis(4000));
        assert_eq!(snap.stopwatch.elapsed_ms, 3000);
        timer.reset(t0 + Duration::from_millis(5000));
        assert_eq!(timer.snapshot(t0 + Duration::from_millis(5000)).stopwatch.elapsed_ms, 0);
    }

    #[test]
    fn pomodoro_cycles_and_auto_starts_break() {
        let mut config = config();
        config.pomodoro.work_minutes = 1;
        config.pomodoro.short_break_minutes = 1;
        config.pomodoro.long_break_every = 2;
        config.pomodoro.auto_start_next = true;
        let mut timer = Timer::new(&config);
        timer.set_mode(Mode::Pomodoro, Instant::now());
        let t0 = Instant::now();
        timer.start(t0);

        let first = timer.tick(t0 + Duration::from_secs(60), &config).unwrap();
        assert_eq!(first.completed_phase, Some("work"));
        assert_eq!(first.next_phase, Some("short_break"));
        assert!(timer.is_running());

        let second = timer.tick(t0 + Duration::from_secs(120), &config).unwrap();
        assert_eq!(second.next_phase, Some("work"));

        timer.reset(t0 + Duration::from_secs(130));
        assert_eq!(
            timer.snapshot(t0).pomodoro.phase,
            PomodoroPhase::Work
        );
        assert_eq!(timer.snapshot(t0).pomodoro.completed_work, 0);
    }

    #[test]
    fn signature_changes_when_clock_second_changes() {
        let timer = Timer::new(&config());
        let t0 = Instant::now();
        let mut first = timer.snapshot(t0);
        first.now_ms = 1_000;
        let mut second = first.clone();
        second.now_ms = 2_000;
        assert_ne!(snapshot_signature(&first), snapshot_signature(&second));
    }
}
