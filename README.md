# Catime

A lightweight floating clock, countdown timer, stopwatch and Pomodoro timer for Linux.
Frameless, transparent, always-on-top (optional), fully customizable — and entirely local.

Built with **Rust + Tauri v2** on the backend and plain **HTML/CSS/vanilla JS** on the frontend.
No bundler, no Node.js build step, no tray icon, no background service: closing the window exits the process.

> Inspired by [Catime](https://github.com/vladelaina/Catime).

---

## Features

| Mode | What it does |
| --- | --- |
| **Clock** | Digital clock with 12h/24h toggle, optional seconds and date line. |
| **Countdown** | Custom duration (H/M/S input + presets), sound chime and desktop notification on completion. |
| **Stopwatch** | Start, pause, resume, reset. Drift-free timing. |
| **Pomodoro** | Configurable work / short break / long break lengths, long-break interval, auto-start next phase, cycle dots. |

**Window & look**

- Frameless, borderless, transparent window with smooth dragging and a resize grip.
- Always-on-top toggle (per-session and persisted).
- Custom font family, size, weight, text/background/accent colors, background opacity.
- Window size and position are remembered between launches.
- Right-click context menu; in-app preferences sheet.

**Performance**

- All timer math is `Instant`-based, so the display never drifts.
- A 30 ms ticker thread only emits an IPC event when the visible text actually changes — one event per second in clock mode, zero events when idle.
- Release profile tuned for size: `opt-level = "s"`, LTO, single codegen unit, stripped symbols.

---

## Project layout

```
.
├── prompt.md                  # Original feature brief
├── ui/                        # Frontend (static assets, no bundler)
│   ├── index.html             # Markup + inline SVG icon sprite
│   ├── styles.css             # Theme, layout, animations
│   └── js/
│       ├── main.js            # Bootstrap, rendering, input, shortcuts
│       ├── settings.js        # Preferences sheet
│       ├── format.js          # Display formatting helpers
│       ├── audio.js           # WebAudio chime
│       └── api.js             # Thin __TAURI__ invoke/listen bridge
└── src-tauri/                 # Rust backend (Tauri v2)
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json        # Window, CSP, bundling (deb + AppImage)
    ├── capabilities/default.json
    ├── icons/
    └── src/
        ├── main.rs            # Entry point, state, window events
        ├── commands.rs        # Commands exposed to the UI
        ├── timer.rs           # Clock/countdown/stopwatch/pomodoro logic
        ├── config.rs          # JSON config load/save/validation
        ├── runtime.rs         # Ticker thread + notify-send helper
        └── window_ops.rs      # Placement, always-on-top, settings resize
```

---

## Requirements

- **Rust** 1.77 or newer ([rustup](https://rustup.rs)) — `edition 2021`, `rust-version = "1.77"`.
- The Tauri v2 Linux system dependencies: WebKitGTK 4.1, GTK 3, libsoup 3, JavaScriptCore, plus a linker and `openssl`/`pkg-config`.
- Optional: `libnotify` (`notify-send`) for desktop notifications — the in-app chime/toast works without it.

### Ubuntu / Debian

```bash
sudo apt update
sudo apt install -y build-essential curl pkg-config libssl-dev \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev

# Optional: desktop notifications
sudo apt install -y libnotify-bin
```

### Fedora

```bash
sudo dnf install -y gcc make pkg-config openssl-devel \
  webkit2gtk4.1-devel gtk3-devel librsvg2-devel \
  libsoup3-devel javascriptcoregtk-4.1-devel

# Optional: desktop notifications
sudo dnf install -y libnotify
```

### Arch

```bash
sudo pacman -S --needed base-devel rust webkit2gtk-4.1 gtk3 \
  librsvg pkgconf openssl libnotify
```

Install Rust if you don't have it yet:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version   # must be >= 1.77
```

Sanity-check the native dependencies before building:

```bash
pkg-config --modversion webkit2gtk-4.1 gtk+-3.0 libsoup-3.0 javascriptcoregtk-4.1
```

---

## Build & run

All Cargo commands run from the `src-tauri/` directory (that's where `Cargo.toml` lives):

```bash
cd src-tauri
```

### Development (hot reload)

```bash
cargo run
```

Tauri serves `../ui` as static frontend assets, so there is no npm install and no bundler step — edit a file in `ui/` and reload the window. The first compile pulls in the Tauri dependency tree and takes several minutes; later builds are incremental and fast.

### Release binary

```bash
cargo build --release --locked
```

The executable lands at:

```
src-tauri/target/release/catime
```

Run it directly:

```bash
./target/release/catime
```

`--locked` uses the committed `Cargo.lock` for a reproducible dependency set. Drop it if you intentionally want to update dependencies.

### Install system-wide

```bash
sudo install -Dm755 target/release/catime /usr/local/bin/catime
```

### Unit tests

```bash
cargo test
```

### Lint / format (optional)

```bash
rustup component add clippy rustfmt
cargo clippy --all-targets
cargo fmt --check
```

---

## Packaging installers (deb / AppImage)

Uses the [Tauri CLI](https://v2.tauri.app/start/prerequisites/). It needs Node.js ≥ 18 on PATH, but only to run the CLI — the app itself has no JavaScript dependencies.

```bash
npm install --save-dev @tauri-apps/cli     # or: cargo install tauri-cli --version "^2"

npx tauri build                            # from repo root; or `cargo tauri build` in src-tauri/
```

Output appears under `src-tauri/target/release/bundle/`:

- `deb/catime_0.1.0_amd64.deb` — install with `sudo apt install ./…​.deb`
- `appimage/Catime_0.1.0_x86_64.AppImage` — `chmod +x` and run

Bundle targets are declared in [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json) (`"targets": ["deb", "appimage"]`). Building an AppImage additionally needs `linuxdeploy`'s tools; the CLI downloads them on first use (network access required).

To package only one format:

```bash
npx tauri build --bundles deb
npx tauri build --bundles appimage
```

---

## Configuration

Settings are stored as human-editable JSON at:

```
~/.config/catime/config.json
```

The file is created on first launch and rewritten atomically (temp file + rename). Window size/position are saved automatically after you move or resize. Every value is clamped to a safe range on load, so hand-editing can never leave the window invisible; if the file is invalid it is renamed to `config.json.invalid` and defaults are used.

```jsonc
{
  "version": 1,
  "appearance": {
    "font_family": "Cantarell",   // any font installed on the system
    "font_size": 62.0,            // 20 – 200
    "font_weight": 300,           // 300 | 400 | 500 | 600 | 700
    "text_color": "#F5F7FF",
    "background_color": "#101320",
    "accent_color": "#8AB4FF",
    "background_opacity": 0.72,   // 0.0 transparent – 1.0 opaque
    "always_on_top": true
  },
  "clock":     { "use_24h": true, "show_seconds": true, "show_date": true },
  "countdown": { "duration_seconds": 300, "sound": true, "notify": true },
  "pomodoro":  {
    "work_minutes": 25,
    "short_break_minutes": 5,
    "long_break_minutes": 15,
    "long_break_every": 4,
    "auto_start_next": true,
    "sound": true,
    "notify": true
  },
  "window": { "width": 440.0, "height": 240.0, "x": null, "y": null }
}
```

Changes made to the file are picked up on the next launch; the in-app preferences sheet writes them while running.

---

## Using Catime

**Controls** — mode buttons along the bar, play/pause and reset in the header, right-click anywhere for the context menu (always-on-top, reset config, settings, quit), drag the body to move the window, drag the corner grip to resize.

| Key | Action |
| --- | --- |
| `Space` | Start / pause the active timer |
| `R` | Reset |
| `S` | Open settings |
| `T` | Toggle always-on-top |
| `1` `2` `3` `4` | Switch to Clock / Countdown / Stopwatch / Pomodoro |
| `Esc` | Close menu, popover, or settings |
| `Ctrl`+`Q` | Quit |

There is no tray icon by design — quit closes the process.

---

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `Package webkit2gtk-4.1 was not found` | Install the WebKitGTK/GTK dev packages for your distro above, then re-run `pkg-config --modversion webkit2gtk-4.1`. On Ubuntu 22.04 and older, only the `-4.0` variant exists — upgrade or point Tauri at it. |
| Blank/transparent window on some compositors | A compositor without transparency support is the usual cause; raise `background_opacity` (e.g. `1.0`) in the config. On X11, enable the compositor (`picom`, KWin compositing, XFWM compositor). |
| No desktop notification | Install `libnotify` / `libnotify-bin` and make sure a notification daemon is running; the sound chime and in-app toast still fire. Wayland: notifications go through `notify-send`, not the Tauri notification plugin. |
| Window invisible after editing config | Values are clamped, but if you set opacity to `0`, restore it: `sed -i 's/"background_opacity": 0[^,]*/"background_opacity": 0.72/' ~/.config/catime/config.json` |
| Build fails on `openssl-sys` | Install the OpenSSL dev package (`libssl-dev` / `openssl-devel`) and `pkg-config`. |
| AppImage won't start | `chmod +x Catime_*.AppImage`; on Fedora/RHEL, FUSE issues are solved with `./Catime_*.AppImage --appimage-extract-and-run`. |
| First build seems stuck | It's compiling the Tauri/WebKit bindings — normal. Subsequent builds are incremental; `cargo build --release` reuses the cache. |

---

## License

MIT — see the `license` field in [`src-tauri/Cargo.toml`](src-tauri/Cargo.toml).
