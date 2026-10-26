# Live Wallpaper

A lightweight MP4 video live wallpaper for **Windows** and **Linux**, built in Rust.

It plays a video on the desktop wallpaper itself (behind your desktop icons) using
[libmpv](https://mpv.io/) embedded into a native background window, and is controlled
from a system-tray icon. It is designed to be light on CPU/GPU: it smartly pauses when
the desktop is hidden (fullscreen games, maximized windows), supports hardware decode,
optional decode downscaling, and a frosted-glass blur while the desktop is covered.

## Features

- Desktop-native playback behind desktop icons
- Tray icon controller: pick a video, fit mode, renderer, downscale, hardware decode,
  volume, audio, monitor mode (primary / all monitors), click-through, and more
- Smart pause: video pauses when a fullscreen app or maximized window covers the desktop
  and resumes when the desktop is shown again
- Optional smooth blur ("frosted glass") while the desktop is hidden
- Pause on battery power (laptops)
- Multiple renderers: Direct3D 11 (Windows) / OpenGL (Linux)
- Single instance: launching a second copy shows a message instead of running twice
- Runtime error reporting: unsupported/corrupt videos, audio-only files and stuck
  playback are logged and surfaced as a tray error
- Per-video resume: playback position is saved and restored on relaunch

## Supported Platforms

| Platform | Status | Build Method |
|----------|--------|-------------|
| Windows 10 / 11 | ✅ Supported | `cargo build --release` |
| Linux (most distros) | ✅ Supported | `cargo build --release` (see BUILD.md) |
| macOS Big Sur 11+ | ✅ Supported | `cargo build --release` (see BUILD.md) |

## Windows Build

```powershell
cargo build --release
```

Keep these DLLs next to `live-wallpaper.exe` (from `dist/`):

- `libmpv-2.dll` — required
- `VCRUNTIME140.dll` — required
- `libEGL.dll`, `libGLESv2.dll` — ANGLE renderer
- `vulkan-1.dll` — Vulkan renderer

See the original README features list below for full Windows usage details.

## Linux Build

Linux distros use different C library versions, so **there is no single prebuilt binary
that runs on all of them**. Each user builds from source on their own machine.

See **`BUILD.md`** for step-by-step instructions for:

- Debian / Ubuntu / Mint / Pop!_OS
- Fedora / RHEL / AlmaLinux / Rocky Linux
- Arch Linux / Manjaro
- openSUSE

After building, run:

```bash
./target/release/live-wallpaper
```

Config is saved to `~/.config/live-wallpaper/config.json` (XDG standard).

## Configuration

### Windows
Config lives in `%APPDATA%\LiveWallpaper\config.json`.

### Linux
Config lives in `~/.config/live-wallpaper/config.json`.

### macOS
Config lives in `~/Library/Application Support/LiveWallpaper/config.json`.

| Key                | Type    | Default       | Meaning                                  |
|--------------------|---------|---------------|------------------------------------------|
| `video`            | string? | `null`        | Path to the video file                   |
| `audio`            | bool    | `true`        | Play the video's audio                   |
| `volume`           | int     | `100`         | Volume 0-100                             |
| `monitor_mode`     | string  | `"primary"`   | `"primary"` or `"all"`                   |
| `autostart`        | bool    | `false`       | Start with Windows / Linux desktop       |
| `click_through`    | bool    | `true`        | Clicks pass through the wallpaper        |
| `fit`              | string  | `"fill"`      | `"fill"`/`"fit"`/`"center"`/`"stretch"`  |
| `renderer`         | string  | `"d3d11"` (Win) / `"gpu"` (Linux) | `"d3d11"`/`"vulkan"`/`"angle"` / `"gpu"`/`"vulkan"` |
| `lite`             | bool    | `false`       | Low GPU/RAM mode (30 fps cap)            |
| `smart_pause`      | bool    | `true`        | Pause when desktop is covered            |
| `dim_when_hidden`  | bool    | `true`        | Blur wallpaper while desktop is hidden   |
| `pause_on_battery` | bool    | `true`        | Pause on battery power (laptops)         |
| `downscale`        | string  | `"off"`       | `"off"`/`"720p"`/`"540p"`                |
| `vsync`            | bool    | `true`        | Sync playback to the display refresh     |
| `hwdec`            | string  | `"auto"`      | Hardware decode mode                     |

Playback position is stored in `position.json` next to `config.json` so a relaunch
resumes where you left off.

## Supported Video Formats

Playback is done by mpv (libmpv), so anything FFmpeg can decode works. Common examples:

| Container        | Extensions                 | Typical codecs                       |
|------------------|----------------------------|--------------------------------------|
| MP4              | `.mp4`, `.m4v`             | H.264, H.265/HEVC, AV1               |
| MKV              | `.mkv`                     | H.264, H.265/HEVC, VP9, AV1          |
| WebM             | `.webm`                    | VP8, VP9, AV1                        |
| AVI              | `.avi`                     | MPEG-4, MJPEG                        |
| MOV              | `.mov`                     | H.264, ProRes, H.265                 |
| WMV              | `.wmv`, `.asf`             | WMV2/3, VC-1                         |
| MPEG             | `.mpg`, `.mpeg`, `.ts`, `.m2ts` | MPEG-1/2, H.264, HEVC           |
| FLV              | `.flv`                     | H.264, VP6, Sorenson                 |
| GIF              | `.gif`                     | (animated GIF)                       |

For live wallpaper use, MP4/H.264 or HEVC at the monitor's native resolution is the
sweet spot (small files, hardware-accelerated decode, smooth looping).

## License

MIT