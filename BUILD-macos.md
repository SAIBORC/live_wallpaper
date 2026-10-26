# Building Live Wallpaper on macOS

Live Wallpaper supports macOS Big Sur (11) and newer.

Because macOS uses different system libraries than Windows or Linux, **each user builds from source on their own Mac**.

---

## Step 1: Install System Dependencies

Install [Homebrew](https://brew.sh/) if you don't have it:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Then install the build dependencies:

```bash
brew install mpv pkg-config
```

Install Xcode command line tools (required for the C compiler):

```bash
xcode-select --install
```

---

## Step 2: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify:
```bash
cargo --version
rustc --version
```

---

## Step 3: Build

Use the macOS-specific Cargo.toml:

```bash
cp Cargo.toml.macos Cargo.toml
cargo build --release
```

The binary is written to `target/release/live-wallpaper`.

---

## Step 4: Run

```bash
./target/release/live-wallpaper
```

The binary loads `libmpv.dylib` from Homebrew (`/opt/homebrew/lib` on Apple Silicon, `/usr/local/lib` on Intel).

Config is saved to `~/Library/Application Support/LiveWallpaper/config.json`.

Logs are written to `~/Library/Caches/live-wallpaper/log.txt`.

---

## Step 5: Make Portable (Optional)

```bash
mkdir -p live-wallpaper-portable
cp target/release/live-wallpaper live-wallpaper-portable/
```

To make it fully portable, you'd need to bundle `libmpv.dylib` using `install_name_tool` or use `cargo-bundle` to create a `.app` bundle.

---

## Permissions (macOS Ventura 13+)

On newer macOS versions, you may need to grant permissions in **System Settings → Privacy & Security**:

- **Accessibility** — for detecting fullscreen windows (smart pause)
- **Screen Recording** — for the wallpaper to appear behind desktop icons
- **Automation** — for controlling other apps (if needed)

Without these, the wallpaper may still run but smart pause and desktop-cover detection may be limited.

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `cargo: command not found` | Run `source $HOME/.cargo/env` |
| `libmpv.dylib not found` | Run `brew install mpv` |
| `cocoa` crate build fails | Run `xcode-select --install` |
| `ld` linker errors | Make sure Xcode command line tools are installed |
| Wallpaper doesn't show behind icons | Grant Accessibility and Screen Recording permissions in System Settings |
| Tray icon doesn't show | Make sure you're not running in a restricted environment (some corporate MDM profiles block status items) |

---

## What Works on macOS

- Video playback via libmpv (OpenGL / Vulkan renderers)
- System tray icon via `NSStatusItem` (AppKit, no dock icon)
- File picker dialog via `NSOpenPanel`
- Desktop-level window behind icons (`kCGDesktopWindowLevel`)
- Smart pause on fullscreen/maximized windows
- Pause on battery power (reads `pmset -g batt`)
- Config in `~/Library/Application Support/LiveWallpaper/`
- Per-video resume position

---

## What's Different from Windows

| Feature | Windows | macOS |
|---------|---------|-------|
| Config dir | `%APPDATA%\LiveWallpaper\` | `~/Library/Application Support/LiveWallpaper/` |
| Log dir | `%TEMP%\LiveWallpaper\` | `~/Library/Caches/live-wallpaper/` |
| Wallpaper method | WorkerW Win32 window | NSWindow at desktop level |
| Tray | Shell_NotifyIcon (Win32) | NSStatusItem (AppKit) |
| Autostart | Registry Run key | `~/Library/LaunchAgents/` plist (manual) |
| Renderers | D3D11, Vulkan, ANGLE | OpenGL, Vulkan |
| libmpv file | `libmpv-2.dll` | `libmpv.dylib` (via Homebrew) |