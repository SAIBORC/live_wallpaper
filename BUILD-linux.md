# Building Live Wallpaper on Linux

Live Wallpaper runs on most Linux desktop distributions (Ubuntu, Debian, Fedora, Arch, Mint, Pop!_OS, openSUSE, etc.).

Because Linux distros use different C library versions (glibc, GTK, etc.), **there is no single prebuilt binary that runs on all of them**. Each user builds from source on their own machine — just like most open-source Linux software.

---

## Step 1: Install System Dependencies

Pick your distro:

### Debian / Ubuntu / Linux Mint / Pop!_OS

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libmpv-dev \
  libx11-dev libxcb-render0-dev libxcb-xfixes0-dev libxcb-icccm4-dev \
  libayatana-appindicator3-dev libgtk-4-dev \
  libwayland-dev wayland-protocols
```

### Fedora / RHEL / AlmaLinux / Rocky Linux

```bash
sudo dnf install -y gcc gcc-c++ make pkgconf mpv-devel \
  libX11-devel xcb-util-devel xcb-util-renderutil-devel xcb-util-wm-devel \
  libayatana-appindicator3-devel gtk4-devel \
  wayland-devel wayland-protocols-devel
```

### Arch Linux / Manjaro

```bash
sudo pacman -Sy --needed base-devel pkgconf mpv \
  libx11 xcb-util xcb-util-renderutil xcb-util-wm \
  libayatana-appindicator gtk4 \
  wayland wayland-protocols
```

### openSUSE

```bash
sudo zypper install -y gcc gcc-c++ make pkg-config mpv-devel \
  libX11-devel xcb-util-devel xcb-util-renderutil-devel xcb-util-wm-devel \
  libayatana-appindicator3-devel gtk4-devel \
  wayland-devel wayland-protocols
```

---

## Step 2: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Log out and back in (or run `source $HOME/.cargo/env`) so `cargo` is in your PATH.

Verify:
```bash
cargo --version
rustc --version
```

---

## Step 3: Build

Use the Linux-specific Cargo.toml:

```bash
cp Cargo.toml.linux Cargo.toml
cargo build --release
```

The binary is written to `target/release/live-wallpaper`.

---

## Step 4: Run

```bash
./target/release/live-wallpaper
```

The binary loads `libmpv.so` from your system (installed by `libmpv-dev` / `mpv-devel` / `mpv`).

Config is saved to `~/.config/live-wallpaper/config.json` (XDG standard).

Logs are written to `~/.cache/live-wallpaper/log.txt`.

---

## Step 5: Make Portable (Optional)

To create a portable folder you can move around or share:

```bash
mkdir -p live-wallpaper-portable
cp target/release/live-wallpaper live-wallpaper-portable/

# Find where libmpv.so is installed and copy it too:
ldd target/release/live-wallpaper | grep libmpv
# Copy the .so file path shown above into live-wallpaper-portable/
```

Linux users who receive your folder run:

```bash
chmod +x live-wallpaper-portable/live-wallpaper
./live-wallpaper-portable/live-wallpaper
```

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `cargo: command not found` | Run `source $HOME/.cargo/env` |
| `libmpv.so not found` | Install `libmpv-dev` (Debian/Ubuntu), `mpv-devel` (Fedora), or `mpv` (Arch) |
| `gtk4 not found` | Install `libgtk-4-dev` / `gtk4-devel` / `gtk4` |
| `ayatana-appindicator3 not found` | Install `libayatana-appindicator3-dev` / `libayatana-appindicator3-devel` / `libayatana-appindicator` |
| Build fails on old distro | Use a newer distro (Ubuntu 22.04+, Fedora 38+, Arch) for newer glibc and GTK |
| Tray icon doesn't show | Make sure you're using X11 or a Wayland compositor that supports appindicator (GNOME, KDE, XFCE) |

---

## What Works on Linux

- Video playback via libmpv (OpenGL / Vulkan renderers)
- System tray icon via GTK4 + libayatana-appindicator
- File picker dialog
- Click-through wallpaper window (X11)
- Smart pause on fullscreen/maximized windows
- Pause on battery power (reads `/sys/class/power_supply/`)
- Config in `~/.config/live-wallpaper/`
- Per-video resume position

---

## What's Different from Windows

| Feature | Windows | Linux |
|---------|---------|-------|
| Config dir | `%APPDATA%\LiveWallpaper\` | `~/.config/live-wallpaper/` |
| Log dir | `%TEMP%\LiveWallpaper\` | `~/.cache/live-wallpaper/` |
| Wallpaper method | WorkerW Win32 window | X11 desktop-level window |
| Tray | Shell_NotifyIcon (Win32) | libayatana-appindicator (GTK4) |
| Autostart | Registry Run key | `.desktop` file (manual setup) |
| Renderers | D3D11, Vulkan, ANGLE | OpenGL, Vulkan |
| libmpv file | `libmpv-2.dll` | `libmpv.so` (system) |