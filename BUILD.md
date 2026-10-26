# Building Live Wallpaper from Source

Live Wallpaper supports **Windows**, **Linux**, and **macOS** (Big Sur 11+).

Because each OS uses different system libraries, **there is no single binary that runs everywhere**. Each user builds from source on their own machine — just like most open-source software.

Windows users get a portable `.exe`. Linux and macOS users compile it themselves using the instructions below.

---

## Prerequisites (All Platforms)

Install **Rust** (stable): https://rustup.rs/

Verify:
```bash
cargo --version
rustc --version
```

---

## Windows

### Build

```powershell
cargo build --release
```

### Run (Portable)

The `.exe` loads `libmpv-2.dll` at runtime. Keep these files next to `live-wallpaper.exe`:

| File | Location |
|------|----------|
| `live-wallpaper.exe` | `target\release\` |
| `libmpv-2.dll` | `dist\` |
| `VCRUNTIME140.dll` | `dist\` |
| `libEGL.dll` | `dist\` |
| `libGLESv2.dll` | `dist\` |
| `vulkan-1.dll` | `dist\` |

Config: `%APPDATA%\LiveWallpaper\config.json`

---

## Linux

### Step 1: Install System Dependencies

**Debian / Ubuntu / Mint / Pop!_OS:**
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libmpv-dev \
  libx11-dev libxcb-render0-dev libxcb-xfixes0-dev libxcb-icccm4-dev \
  libayatana-appindicator3-dev libgtk-4-dev \
  libwayland-dev wayland-protocols
```

**Fedora / RHEL / AlmaLinux / Rocky:**
```bash
sudo dnf install -y gcc gcc-c++ make pkgconf mpv-devel \
  libX11-devel xcb-util-devel xcb-util-renderutil-devel xcb-util-wm-devel \
  libayatana-appindicator3-devel gtk4-devel \
  wayland-devel wayland-protocols-devel
```

**Arch / Manjaro:**
```bash
sudo pacman -Sy --needed base-devel pkgconf mpv \
  libx11 xcb-util xcb-util-renderutil xcb-util-wm \
  libayatana-appindicator gtk4 \
  wayland wayland-protocols
```

**openSUSE:**
```bash
sudo zypper install -y gcc gcc-c++ make pkg-config mpv-devel \
  libX11-devel xcb-util-devel xcb-util-renderutil-devel xcb-util-wm-devel \
  libayatana-appindicator3-devel gtk4-devel \
  wayland-devel wayland-protocols
```

### Step 2: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Step 3: Build

```bash
cp Cargo.toml.linux Cargo.toml
cargo build --release
```

### Step 4: Run

```bash
./target/release/live-wallpaper
```

Config: `~/.config/live-wallpaper/config.json`

---

## macOS (Big Sur 11+)

### Step 1: Install System Dependencies

Install [Homebrew](https://brew.sh/) if needed:
```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Then:
```bash
brew install mpv pkg-config
xcode-select --install
```

### Step 2: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Step 3: Build

```bash
cp Cargo.toml.macos Cargo.toml
cargo build --release
```

### Step 4: Run

```bash
./target/release/live-wallpaper
```

Config: `~/Library/Application Support/LiveWallpaper/config.json`

---

## Platform Comparison

| Feature | Windows | Linux | macOS |
|---------|---------|-------|-------|
| Build command | `cargo build --release` | `cp Cargo.toml.linux Cargo.toml && cargo build --release` | `cp Cargo.toml.macos Cargo.toml && cargo build --release` |
| Config dir | `%APPDATA%\LiveWallpaper\` | `~/.config/live-wallpaper/` | `~/Library/Application Support/LiveWallpaper/` |
| Log dir | `%TEMP%\LiveWallpaper\` | `~/.cache/live-wallpaper/` | `~/Library/Caches/live-wallpaper/` |
| Wallpaper method | WorkerW (Win32) | X11 desktop window | NSWindow (desktop level) |
| Tray icon | Shell_NotifyIcon | GTK4 appindicator | NSStatusItem |
| Renderers | D3D11, Vulkan, ANGLE | OpenGL, Vulkan | OpenGL, Vulkan |
| libmpv file | `libmpv-2.dll` | `libmpv.so` (system) | `libmpv.dylib` (Homebrew) |

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `cargo: command not found` | Run `source $HOME/.cargo/env` (Linux/macOS) or restart terminal (Windows) |
| `libmpv-2.dll not found` (Windows) | Copy from `dist/` next to the exe |
| `libmpv.so not found` (Linux) | Install `libmpv-dev` / `mpv-devel` / `mpv` |
| `libmpv.dylib not found` (macOS) | Run `brew install mpv` |
| `gtk4 not found` (Linux) | Install `libgtk-4-dev` / `gtk4-devel` / `gtk4` |
| `cocoa` crate build fails (macOS) | Run `xcode-select --install` |
| Build fails on old Linux distro | Use a newer distro (Ubuntu 22.04+, Fedora 38+, Arch) |
| Tray icon missing (macOS) | Grant Accessibility & Screen Recording in System Settings |
| Dubious ownership error (Git) | `git config --global --add safe.directory "<your-project-path>"` |

---

## Source Layout

```
src/main.rs        Entry point, player loop, single-instance guard (all platforms)
src/config.rs      Config load/save (OS-specific paths)
src/log.rs         Logging (OS-specific log dirs)
src/mpv.rs         libmpv FFI bindings (.dll / .so / .dylib)

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