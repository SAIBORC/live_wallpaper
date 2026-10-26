# Building Live Wallpaper from Source

Live Wallpaper supports **Windows**, **Linux**, and **macOS** (Big Sur 11+).

Because each OS uses different system libraries, **there is no single binary that runs everywhere**. Each user builds from source on their own machine — just like most open-source software.

---

## Choose Your Platform

| Platform | Build Guide |
|----------|-------------|
| Windows | See below (or `README.md`) |
| Linux | [BUILD-linux.md](./BUILD-linux.md) |
| macOS (Big Sur 11+) | [BUILD-macos.md](./BUILD-macos.md) |

---

## Windows (Quick)

```powershell
cargo build --release
```

Keep these DLLs next to `live-wallpaper.exe` (from `dist/`):

- `libmpv-2.dll` — required
- `VCRUNTIME140.dll` — required
- `libEGL.dll`, `libGLESv2.dll` — ANGLE renderer
- `vulkan-1.dll` — Vulkan renderer

Config: `%APPDATA%\LiveWallpaper\config.json`

---

## Linux (Quick)

```bash
# Install distro dependencies (see BUILD-linux.md for full list)
# Debian/Ubuntu example:
sudo apt install -y build-essential pkg-config libmpv-dev \
  libx11-dev libxcb-render0-dev libxcb-xfixes0-dev libxcb-icccm4-dev \
  libayatana-appindicator3-dev libgtk-4-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

cp Cargo.toml.linux Cargo.toml
cargo build --release
./target/release/live-wallpaper
```

Config: `~/.config/live-wallpaper/config.json`

Full guide: [BUILD-linux.md](./BUILD-linux.md)

---

## macOS (Quick)

```bash
brew install mpv pkg-config
xcode-select --install

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

cp Cargo.toml.macos Cargo.toml
cargo build --release
./target/release/live-wallpaper
```

Config: `~/Library/Application Support/LiveWallpaper/config.json`

Full guide: [BUILD-macos.md](./BUILD-macos.md)

---

## Shared Source Files

```
src/main.rs        Entry point, player loop, single-instance guard (all platforms)
src/config.rs      Config load/save (OS-specific paths)
src/log.rs         Logging (OS-specific log dirs)
src/mpv.rs         libmpv FFI (Windows .dll, Linux .so, macOS .dylib)

src/ui.rs          Windows: tray icon, menu, hotkey
src/wallpaper.rs   Windows: WorkerW window behind desktop

src/ui_linux.rs    Linux: GTK4 appindicator tray + file picker
src/wallpaper_linux.rs  Linux: X11 window behind desktop
src/battery_linux.rs    Linux: /sys/class/power_supply battery check

src/ui_macos.rs    macOS: NSStatusItem tray + NSOpenPanel picker
src/wallpaper_macos.rs  macOS: NSWindow desktop-level behind icons
src/battery_macos.rs    macOS: pmset battery check
```

---

## License

MIT