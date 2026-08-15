# Live Wallpaper

A lightweight MP4 video live wallpaper for Windows 11, built in Rust.

It plays a video on the desktop wallpaper itself (behind your desktop icons) using
[libmpv](https://mpv.io/) embedded into a native background window, and is controlled
from a system-tray icon. It is designed to be light on CPU/GPU: it smartly pauses when
the desktop is hidden (fullscreen games, maximized windows), supports hardware decode,
optional decode downscaling, and a frosted-glass blur while the desktop is covered.

## Features

- Desktop-native playback behind desktop icons (works on Windows 10 and 11, including
  the new 24H2/26H1 desktop layout)
- Tray icon controller: pick a video, fit mode, renderer, downscale, hardware decode,
  volume, audio, monitor mode (primary / all monitors), click-through, and more
- Smart pause: video pauses when a fullscreen app or maximized window covers the desktop
  and resumes when the desktop is shown again
- Optional smooth blur ("frosted glass") while the desktop is hidden
- Pause on battery power (laptops)
- Multiple renderers: Direct3D 11 (default), Vulkan, ANGLE (OpenGL over D3D11)
- Single instance: launching a second copy shows a message box instead of running twice
- Runtime error reporting: unsupported/corrupt videos, audio-only files and stuck
  playback are logged and surfaced as a tray error balloon
- Per-video resume: playback position is saved and restored on relaunch
- Custom exe/tray/window icon embedded at build time

## Requirements (to build)

- Windows 10 or 11
- [Rust](https://rustup.rs/) (stable, edition 2021)
- Windows SDK (optional): only needed so `rc.exe` can embed the app icon. If `rc.exe`
  is not found the build still succeeds, just without the custom icon.
- `app.ico` / `app.rc` in the project root (both are committed).

## Build

```sh
cargo build --release
```

The binary is written to `target\release\live-wallpaper.exe`.

To debug locally, `cargo run` also works.

## Run

The exe is useless on its own: it loads libmpv at runtime. Keep the following files
next to `live-wallpaper.exe`:

| File                | Required | Purpose                                |
| ------------------- | -------- | -------------------------------------- |
| `libmpv-2.dll`      | Yes      | mpv playback engine                    |
| `VCRUNTIME140.dll`  | Yes      | Microsoft C/C++ runtime for mpv        |
| `libEGL.dll`        | No       | ANGLE renderer (if you pick ANGLE)     |
| `libGLESv2.dll`     | No       | ANGLE renderer (if you pick ANGLE)     |
| `vulkan-1.dll`      | No       | Vulkan renderer (if you pick Vulkan)   |

Pick up a prebuilt `libmpv-2.dll` from the [mpv builds](https://sourceforge.net/projects/mpv-player-windows/files/)
or build libmpv yourself and copy the DLLs.

### First run

Right-click the tray icon and choose **Pick video...**, or set the `video` field in the
config file (below), then restart the app.

## Supported video formats

Playback is done by mpv (libmpv), so anything FFmpeg can decode works. Common examples:

| Container        | Extensions                 | Typical codecs                       |
| ---------------- | -------------------------- | ------------------------------------ |
| MP4              | `.mp4`, `.m4v`             | H.264, H.265/HEVC, AV1               |
| MKV              | `.mkv`                     | H.264, H.265/HEVC, VP9, AV1          |
| WebM             | `.webm`                    | VP8, VP9, AV1                        |
| AVI              | `.avi`                     | MPEG-4, MJPEG                        |
| MOV              | `.mov`                     | H.264, ProRes, H.265                 |
| WMV              | `.wmv`, `.asf`             | WMV2/3, VC-1                         |
| MPEG             | `.mpg`, `.mpeg`, `.ts`, `.m2ts` | MPEG-1/2, H.264, HEVC           |
| FLV              | `.flv`                     | H.264, VP6, Sorenson                 |
| GIF              | `.gif`                     | (animated GIF)                       |

The app does not restrict the file type: the picker shows common video files, and if you
name a file directly in `config.json` it is passed straight to mpv. Anything that fails
to load (unsupported, corrupt, or audio-only with no video stream) is logged to
`%TEMP%\LiveWallpaper\log.txt` and reported with a tray error balloon.

For live wallpaper use, MP4/H.264 or HEVC at the monitor's native resolution is the
sweet spot (small files, hardware-accelerated decode, smooth looping).

## Tray menu

Right-click the tray icon for:

- Current video / **Pick video...**
- **Play audio**, **Volume +10 / -10**
- **Video fit**: Fill (crop to screen), Fit (whole video visible), Center (original size),
  Stretch (fill and distort)
- **Renderer / API**: Direct3D 11, Vulkan, ANGLE
- **Decode downscale**: Off, Max 720p, Max 540p
- **Hardware decode**: Auto, d3d11va-copy, dxva2-copy, Off
- **Lite mode** (low GPU/RAM), **V-sync**
- **Pause when desktop hidden** (fullscreen/maximized)
- **Blur when hidden** (smooth fade)
- **Pause on battery power**
- **Monitor mode**: Primary monitor / All monitors
- **Click through desktop**, **Start with Windows**, **Quit**

Hotkey: **Ctrl+Alt+W** opens the video picker from anywhere.

## Configuration

Config lives in `%APPDATA%\LiveWallpaper\config.json` and is recreated with defaults
if missing or invalid.

| Key                | Type    | Default       | Meaning                                  |
| ------------------ | ------- | ------------- | ---------------------------------------- |
| `video`            | string? | `null`        | Path to the video file                   |
| `audio`            | bool    | `true`        | Play the video's audio                   |
| `volume`           | int     | `100`         | Volume 0-100                             |
| `monitor_mode`     | string  | `"primary"`   | `"primary"` or `"all"`                   |
| `autostart`        | bool    | `false`       | Start with Windows (registry Run key)    |
| `click_through`    | bool    | `true`        | Clicks pass through the wallpaper        |
| `fit`              | string  | `"fill"`      | `"fill"`/`"fit"`/`"center"`/`"stretch"`  |
| `renderer`         | string  | `"d3d11"`     | `"d3d11"`/`"vulkan"`/`"angle"`           |
| `lite`             | bool    | `false`       | Low GPU/RAM mode (30 fps cap)            |
| `smart_pause`      | bool    | `true`        | Pause when desktop is covered            |
| `dim_when_hidden`  | bool    | `true`        | Blur wallpaper while desktop is hidden   |
| `pause_on_battery` | bool    | `true`        | Pause on battery power (laptops)         |
| `downscale`        | string  | `"off"`       | `"off"`/`"720p"`/`"540p"`                |
| `vsync`            | bool    | `true`        | Sync playback to the display refresh     |
| `hwdec`            | string  | `"auto"`      | `"auto"`/`"d3d11va-copy"`/`"dxva2-copy"`/`"off"` |

Playback position is stored in `%APPDATA%\LiveWallpaper\position.json` so a relaunch
resumes where you left off.

## Log

A runtime log is written to `%TEMP%\LiveWallpaper\log.txt` (the same folder also holds
the single-instance lock file `instance.lock`). Errors such as an unsupported video,
an audio-only file, or stuck playback appear there and as a tray balloon.

## Project layout

```
src/main.rs       Entry point, session loop, single-instance guard, error detection
src/mpv.rs        libmpv FFI bindings, player setup, blur filter, fit modes
src/ui.rs         Tray icon, menu, file picker, hotkey, dark mode, error balloons
src/wallpaper.rs  Desktop integration (WorkerW), wallpaper surfaces, visibility detection
src/config.rs     Configuration load/save, playback position
src/log.rs        Logging to %TEMP%\LiveWallpaper\log.txt
build.rs          Embeds app.ico into the exe using rc.exe
app.rc, app.ico   App icon (replace and rebuild to change)
```

## Notes / known quirks

- The blur uses a software scale-down/up scale chain rather than a `boxblur` filter
  (lavfi's boxblur produces corrupted frames on the D3D11 output). It requires the
  `-copy` hardware-decode paths (`d3d11va-copy` / `dxva2-copy`), so enabling the blur
  restarts playback with one of those decode modes.
- The `start` position is set as an mpv player option before `loadfile`, because this
  mpv build rejects `start=...` passed as a loadfile argument.

## License

MIT
