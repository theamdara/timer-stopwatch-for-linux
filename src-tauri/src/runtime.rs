//! Background ticker + native notification helper.
//!
//! The thread wakes every 30 ms (cheap) but only emits an IPC event when the
//! visible text would actually change. Clock mode therefore sends one event per
//! second and an idle timer sends nothing at all.

use crate::timer::{snapshot_signature, FinishedEvent};
use crate::SharedState;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub fn spawn_ticker(app: AppHandle) {
    std::thread::Builder::new()
        .name("catime-ticker".into())
        .spawn(move || {
            let mut last_signature = String::new();

            loop {
                std::thread::sleep(Duration::from_millis(30));
                let now = Instant::now();

                let (snapshot, finished) = {
                    let state = app.state::<SharedState>();
                    let mut state = match state.lock() {
                        Ok(state) => state,
                        Err(_) => break,
                    };

                    let finished = if state.timer.is_running() {
                        // A local clone avoids borrowing two fields of the
                        // mutex guard at once; it is only taken while a timer
                        // actually runs.
                        let config = state.config.clone();
                        state.timer.tick(now, &config)
                    } else {
                        None
                    };

                    if state
                        .geometry_dirty_at
                        .map(|changed| changed.elapsed() >= Duration::from_millis(650))
                        .unwrap_or(false)
                    {
                        state.geometry_dirty_at = None;
                        state.persist();
                    }

                    let snapshot = state.timer.snapshot(now);
                    (snapshot, finished)
                };

                if let Some(event) = finished {
                    if event.notify {
                        notify_desktop(&event);
                    }
                    let _ = app.emit("timer-finished", &event);
                }

                let signature = snapshot_signature(&snapshot);
                if signature != last_signature {
                    last_signature = signature;
                    let _ = app.emit("timer-tick", &snapshot);
                }
            }
        })
        .expect("failed to start catime ticker thread");
}

/// Use the desktop's standard notification daemon through `notify-send`
/// (libnotify). If it is not installed the in-app sound/toast still works, so
/// this is deliberately best-effort and never blocks the timer thread.
fn notify_desktop(event: &FinishedEvent) {
    let title = event.title.clone();
    let body = event.body.clone();

    std::thread::spawn(move || {
        let _ = Command::new("notify-send")
            .args([
                "--app-name=Catime",
                "--icon=alarm-symbolic",
                "--urgency=normal",
                "--",
                &title,
                &body,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    });
}
