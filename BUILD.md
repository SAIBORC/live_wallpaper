# Building Live Wallpaper from Source

Live Wallpaper is written in Rust and supports **Windows**, **Linux**, and **macOS** (Big Sur 11+).

Because Linux distros use different C library versions (glibc, GTK, etc.), there is no single binary that runs on all of them. Instead, **each user builds it on their own machine** — just like most open-source Linux software.

---

## Prerequisites (all platforms)

- **Rust** (stable): https://rustup.rs/
- **git** (to clone the repo)

---

## Windows

### Build

```powershell
cargo build --release
```

### Run (portable)

The `.exe` loads `libmpv-2.dll` at runtime. Keep these files next to `live-wallpaper.exe`:

| File | Source |
|------|--------|
| `live-wallpaper.exe` | `target/release/` |
| `libmpv-2.dll` | `dist/` |
| `VCRUNTIME140.dll` | `dist/` |
| `libEGL.dll` | `dist/` |
| `libGLESv2.dll` | `dist/` |
| `vulkan-1.dll` | `dist/` |

Config is saved to `%APPDATA%\LiveWallpaper\config.json`.

---

## Linux

### Step 1: Install system dependencies

Pick your distro:

#### Debian / Ubuntu / Linux Mint / Pop!_OS

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libmpv-dev \
  libx11-dev libxcb-render0-dev libxcb-xfixes0-dev libxcb-icccm4-dev \
  libayatana-appindicator3-dev libgtk-4-dev \
  libwayland-dev wayland-protocols
```

#### Fedora / RHEL / AlmaLinux / Rocky Linux

```bash
sudo dnf install -y gcc gcc-c++ make pkgconf mpv-devel \
  libX11-devel xcb-util-devel xcb-util-renderutil-devel xcb-util-wm-devel \
  libayatana-appindicator3-devel gtk4-devel \
  wayland-devel wayland-protocols-devel
```

#### Arch Linux / Manjaro

```bash
sudo pacman -Sy --needed base-devel pkgconf mpv \
  libx11 xcb-util xcb-util-renderutil xcb-util-wm \
  libayatana-appindicator gtk4 \
  wayland wayland-protocols
```

#### openSUSE

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

Use the Linux-specific `Cargo.toml`:

```bash
cp Cargo.toml.linux Cargo.toml
cargo build --release
```

### Step 4: Run

```bash
./target/release/live-wallpaper
```

The binary loads `libmpv.so` from the system (installed by `libmpv-dev` / `mpv-devel` / `mpv`).

Config is saved to `~/.config/live-wallpaper/config.json` (XDG standard).

### Step 5: Make portable (optional)

To create a portable folder you can move around:

```bash
mkdir -p live-wallpaper-portable
cp target/release/live-wallpaper live-wallpaper-portable/
# Bundle libmpv.so if you want to avoid system dependency:
ldd target/release/live-wallpaper | grep libmpv
# Copy the .so file it points to into live-wallpaper-portable/
```

Linux users who receive your folder run:

```bash
chmod +x live-wallpaper-portable/live-wallpaper
./live-wallpaper-portable/live-wallpaper
```

---

## macOS (Big Sur 11+)

### Step 1: Install system dependencies

Install [Homebrew](https://brew.sh/) if you don't have it, then:

```bash
brew install mpv pkg-config
```

You also need Xcode command line tools:

```bash
xcode-select --install
```

### Step 2: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Step 3: Build

Use the macOS-specific `Cargo.toml`:

```bash
cp Cargo.toml.macos Cargo.toml
cargo build --release
```

### Step 4: Run

```bash
./target/release/live-wallpaper
```

The binary loads `libmpv.dylib` from Homebrew (`/opt/homebrew/lib` or `/usr/local/lib`).

Config is saved to `~/Library/Application Support/LiveWallpaper/config.json`.

### Step 5: Make portable (optional)

```bash
mkdir -p live-wallpaper-portable
cp target/release/live-wallpaper live-wallpaper-portable/
```

To make it fully portable, you'd need to bundle `libmpv.dylib` using `install_name_tool` or use `cargo-bundle` to create a `.app` bundle.

---

**Note:** macOS wallpaper integration uses `NSWindow` at desktop level (`kCGDesktopWindowLevel`). On newer macOS versions (Ventura 13+), you may need to grant Accessibility and Screen Recording permissions in System Settings for the wallpaper to appear behind desktop icons.

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `cargo: command not found` | Run `source $HOME/.cargo/env` (Linux/macOS) or restart your terminal |
| `libmpv.so not found` | Install `libmpv-dev` (Debian/Ubuntu) or `mpv-devel` (Fedora) or `mpv` (Arch) |
| `gtk4 not found` | Install `libgtk-4-dev` / `gtk4-devel` / `gtk4` |
| `ayatana-appindicator3 not found` | Install `libayatana-appindicator3-dev` / `libayatana-appindicator3-devel` / `libayatana-appindicator` |
| Build fails on old distro | Use a newer distro (Ubuntu 22.04+, Fedora 38+, Arch) for newer glibc and GTK |
| `libmpv.dylib not found` (macOS) | Run `brew install mpv` and ensure `/opt/homebrew/lib` or `/usr/local/lib` is in your library path |
| `cocoa` crate build fails | Ensure Xcode command line tools are installed: `xcode-select --install` |

---

## Project Layout

```
src/main.rs        Entry point, player loop, single-instance guard
src/config.rs      Config load/save (Windows %APPDATA%, Linux XDG)
src/log.rs         Logging (Windows %TEMP%, Linux ~/.cache)
src/mpv.rs         libmpv FFI (Windows .dll, Linux .so)
src/ui.rs          Windows: tray icon, menu, hotkey
src/wallpaper.rs   Windows: WorkerW window behind desktop
src/ui_linux.rs    Linux: GTK4 appindicator tray + file picker
src/wallpaper_linux.rs  Linux: X11 window behind desktop
src/battery_linux.rs    Linux: /sys/class/power_supply battery check
build.rs           Windows: embeds app.ico via rc.exe
Cargo.toml         Windows build
Cargo.toml.linux   Linux build (copy to Cargo.toml to build on Linux)
app.rc, app.ico    Windows app icon
```

---

## License

MIT