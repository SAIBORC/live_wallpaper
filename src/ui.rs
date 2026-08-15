use crate::{config, log, wallpaper, AppShared, Cmd};
use std::mem::size_of;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread::JoinHandle;
use windows::core::{w, PCSTR, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
};
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_HIDEREADONLY, OPENFILENAMEW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, MOD_ALT, MOD_CONTROL, VK_W};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconGetRect, Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP,
    NIIF_ERROR, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW, NOTIFYICONIDENTIFIER,
};
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn enable_dark_mode() {
    unsafe {
        if let Ok(uxtheme) = LoadLibraryW(w!("uxtheme.dll")) {
            if let Some(f) = GetProcAddress(uxtheme, PCSTR(135usize as *const u8)) {
                let call: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(f);
                call(2);
            }
            if let Some(f) = GetProcAddress(uxtheme, PCSTR(136usize as *const u8)) {
                let call: unsafe extern "system" fn() = std::mem::transmute(f);
                call();
            }
        }
    }
}

const WM_TRAY: u32 = WM_APP + 1;
const WM_SHOW_MENU: u32 = WM_APP + 2;
const HOTKEY_ID: i32 = 1;

static TRAY_HWND: AtomicIsize = AtomicIsize::new(0);

const ID_PICK: usize = 1;
const ID_AUDIO: usize = 2;
const ID_VOL_UP: usize = 3;
const ID_VOL_DOWN: usize = 4;
const ID_MON_PRIMARY: usize = 5;
const ID_MON_ALL: usize = 6;
const ID_CLICK: usize = 7;
const ID_AUTOSTART: usize = 8;
const ID_QUIT: usize = 9;
const ID_FIT_FILL: usize = 10;
const ID_FIT_FIT: usize = 11;
const ID_FIT_CENTER: usize = 12;
const ID_FIT_STRETCH: usize = 13;
const ID_REND_D3D11: usize = 14;
const ID_REND_VULKAN: usize = 15;
const ID_LITE: usize = 16;
const ID_SMART: usize = 17;
const ID_REND_ANGLE: usize = 19;
const ID_DOWN_OFF: usize = 20;
const ID_DOWN_720: usize = 21;
const ID_DOWN_540: usize = 22;
const ID_VSYNC: usize = 23;
const ID_HW_AUTO: usize = 24;
const ID_HW_D3D11VA: usize = 25;
const ID_HW_DXVA2: usize = 26;
const ID_HW_OFF: usize = 27;
const ID_DIM: usize = 28;
const ID_BATTERY: usize = 29;

const MF_GRAYED: MENU_ITEM_FLAGS = MENU_ITEM_FLAGS(0x0000_0001);
const MF_DISABLED: MENU_ITEM_FLAGS = MENU_ITEM_FLAGS(0x0000_0002);
const MF_SEPARATOR: MENU_ITEM_FLAGS = MENU_ITEM_FLAGS(0x0000_0800);
const MF_POPUP: MENU_ITEM_FLAGS = MENU_ITEM_FLAGS(0x0000_0010);
const TPM_NONOTIFY: TRACK_POPUP_MENU_FLAGS = TRACK_POPUP_MENU_FLAGS(0x0000_0080);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn video_title(cfg: &config::Config) -> String {
    cfg.video
        .as_deref()
        .and_then(|p| std::path::Path::new(p).file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "No video selected".to_string())
}

pub fn app_icon() -> HICON {
    let hmod = unsafe { GetModuleHandleW(None) }.unwrap_or_default();
    let hid = unsafe { LoadIconW(HINSTANCE(hmod.0), PCWSTR(1usize as *const u16)) };
    match hid {
        Ok(i) if !i.0.is_null() => i,
        _ => match unsafe { LoadIconW(None, IDI_APPLICATION) } {
            Ok(i) => i,
            Err(_) => HICON::default(),
        },
    }
}

fn set_tip(nid: &mut NOTIFYICONDATAW, s: &str) {
    let used = s.encode_utf16().count().min(127);
    for (i, c) in s.encode_utf16().take(127).enumerate() {
        nid.szTip[i] = c;
    }
    nid.szTip[used] = 0;
}

static TIP_TEXT: Mutex<String> = Mutex::new(String::new());

pub fn update_tip(text: &str) {
    {
        let mut cur = TIP_TEXT.lock().unwrap();
        if *cur == text {
            return;
        }
        *cur = text.to_string();
    }
    let raw = TRAY_HWND.load(Ordering::SeqCst);
    if raw == 0 {
        return;
    }
    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = HWND(raw as *mut std::ffi::c_void);
    nid.uID = 1;
    nid.uFlags = NIF_TIP;
    set_tip(&mut nid, text);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

pub fn notify_error(msg: &str) {
    let raw = TRAY_HWND.load(Ordering::SeqCst);
    if raw == 0 {
        return;
    }
    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = HWND(raw as *mut std::ffi::c_void);
    nid.uID = 1;
    nid.uFlags = NIF_INFO;
    nid.dwInfoFlags = NIIF_ERROR;
    let title = "Live Wallpaper error";
    for (i, c) in title.encode_utf16().take(63).enumerate() {
        nid.szInfoTitle[i] = c;
    }
    nid.szInfoTitle[63] = 0;
    for (i, c) in msg.encode_utf16().take(255).enumerate() {
        nid.szInfo[i] = c;
    }
    nid.szInfo[255] = 0;
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

unsafe extern "system" fn tray_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_DESTROY {
        PostQuitMessage(0);
        return LRESULT(0);
    }
    if msg == WM_CLOSE {
        let _ = DestroyWindow(hwnd);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn tray_ll_mouse(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam.0 as u32 == WM_RBUTTONUP {
        let raw = TRAY_HWND.load(Ordering::SeqCst);
        if raw != 0 {
            let hwnd = HWND(raw as *mut std::ffi::c_void);
            let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let ident = NOTIFYICONIDENTIFIER {
                cbSize: size_of::<NOTIFYICONIDENTIFIER>() as u32,
                hWnd: hwnd,
                uID: 1,
                guidItem: windows::core::GUID::default(),
            };
            if let Ok(r) = Shell_NotifyIconGetRect(&ident) {
                const TOL: i32 = 8;
                let inside = ms.pt.x >= r.left - TOL
                    && ms.pt.x <= r.right + TOL
                    && ms.pt.y >= r.top - TOL
                    && ms.pt.y <= r.bottom + TOL;
                if inside {
                    let _ = PostMessageW(hwnd, WM_SHOW_MENU, WPARAM(0), LPARAM(0));
                }
            }
        }
    }
    CallNextHookEx(HHOOK(std::ptr::null_mut()), code, wparam, lparam)
}

pub fn autostart_enabled() -> Option<bool> {
    let name = w!("LiveWallpaper");
    unsafe {
        let mut key = HKEY::default();
        let rc = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        );
        if rc != ERROR_SUCCESS {
            return None;
        }
        let mut size: u32 = 0;
        let qrc = RegQueryValueExW(key, name, None, None, None, Some(&mut size as *mut u32));
        if qrc == ERROR_FILE_NOT_FOUND {
            let _ = RegCloseKey(key);
            return None;
        }
        if qrc != ERROR_SUCCESS || size == 0 {
            let _ = RegCloseKey(key);
            return Some(false);
        }
        let mut buf = vec![0u8; size as usize];
        let qrc2 = RegQueryValueExW(
            key,
            name,
            None,
            None,
            Some(buf.as_mut_ptr()),
            Some(&mut size as *mut u32),
        );
        let _ = RegCloseKey(key);
        if qrc2 != ERROR_SUCCESS {
            return Some(false);
        }
        let ws: Vec<u16> = buf[..(size as usize)]
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let end = ws.iter().position(|&c| c == 0).unwrap_or(ws.len());
        let stored = String::from_utf16_lossy(&ws[..end]);
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        Some(stored == exe)
    }
}

fn set_autostart(on: bool) -> Result<(), String> {
    let name = w!("LiveWallpaper");
    unsafe {
        let mut key = HKEY::default();
        let rc = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            0,
            KEY_SET_VALUE,
            &mut key,
        );
        if rc != ERROR_SUCCESS {
            return Err(format!("open Run key: {rc:?}"));
        }
        let res = if on {
            let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
            let mut w: Vec<u16> = exe.to_string_lossy().encode_utf16().collect();
            w.push(0);
            let mut bytes = Vec::with_capacity(w.len() * 2);
            for c in &w {
                bytes.extend_from_slice(&c.to_le_bytes());
            }
            RegSetValueExW(key, name, 0, REG_SZ, Some(&bytes))
        } else {
            RegDeleteValueW(key, name)
        };
        let _ = RegCloseKey(key);
        if res == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("registry write: {res:?}"))
        }
    }
}

fn pick_video() -> Option<String> {
    let filter = wide(
        "Video files\0*.mp4;*.mkv;*.avi;*.webm;*.mov;*.flv;*.wmv\0All files\0*.*\0\0",
    );
    let mut file_buf = [0u16; 1024];
    let mut ofn = OPENFILENAMEW::default();
    ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = HWND::default();
    ofn.lpstrFilter = PCWSTR(filter.as_ptr());
    ofn.lpstrFile = PWSTR(file_buf.as_mut_ptr());
    ofn.nMaxFile = file_buf.len() as u32;
    ofn.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_HIDEREADONLY;
    let ok = unsafe { GetOpenFileNameW(&mut ofn).as_bool() };
    if !ok {
        return None;
    }
    let s = unsafe { PWSTR(file_buf.as_mut_ptr()).to_string() }.unwrap_or_default();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

unsafe fn build_menu(cfg: &config::Config) -> Result<HMENU, String> {
    let menu = CreatePopupMenu().map_err(|e| format!("CreatePopupMenu: {e}"))?;

    unsafe fn item(
        menu: HMENU,
        flags: MENU_ITEM_FLAGS,
        id: usize,
        text: PCWSTR,
    ) -> Result<(), String> {
        AppendMenuW(menu, flags, id, text).map_err(|e| format!("AppendMenuW: {e}"))
    }

    item(
        menu,
        MF_GRAYED | MF_DISABLED,
        0,
        PCWSTR(wide(&video_title(cfg)).as_ptr()),
    )?;
    item(menu, MF_SEPARATOR, 0, PCWSTR::null())?;

    item(menu, MF_STRING, ID_PICK, w!("Pick video..."))?;
    item(menu, MF_SEPARATOR, 0, PCWSTR::null())?;

    let audio = if cfg.audio {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    item(menu, audio, ID_AUDIO, w!("Play audio"))?;
    item(menu, MF_STRING, ID_VOL_UP, w!("Volume +10"))?;
    item(menu, MF_STRING, ID_VOL_DOWN, w!("Volume -10"))?;
    item(menu, MF_SEPARATOR, 0, PCWSTR::null())?;

    let fit = CreatePopupMenu().map_err(|e| format!("CreatePopupMenu: {e}"))?;
    for (mode, label, id) in [
        ("fill", "Fill (crop to screen)", ID_FIT_FILL),
        ("fit", "Fit (whole video visible)", ID_FIT_FIT),
        ("center", "Center (original size)", ID_FIT_CENTER),
        ("stretch", "Stretch (fill and distort)", ID_FIT_STRETCH),
    ] {
        let f = if cfg.fit == mode {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        AppendMenuW(fit, f, id, PCWSTR(wide(label).as_ptr()))
            .map_err(|e| format!("AppendMenuW: {e}"))?;
    }
    item(menu, MF_POPUP | MF_STRING, fit.0 as usize, w!("Video fit"))?;

    let rend = CreatePopupMenu().map_err(|e| format!("CreatePopupMenu: {e}"))?;
    for (name, label, id) in [
        ("d3d11", "Direct3D 11 (recommended)", ID_REND_D3D11),
        ("vulkan", "Vulkan", ID_REND_VULKAN),
        ("angle", "ANGLE (OpenGL over D3D11)", ID_REND_ANGLE),
    ] {
        let f = if cfg.renderer == name {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        AppendMenuW(rend, f, id, PCWSTR(wide(label).as_ptr()))
            .map_err(|e| format!("AppendMenuW: {e}"))?;
    }
    item(
        menu,
        MF_POPUP | MF_STRING,
        rend.0 as usize,
        w!("Renderer / API"),
    )?;

    let down = CreatePopupMenu().map_err(|e| format!("CreatePopupMenu: {e}"))?;
    for (mode, label, id) in [
        ("off", "Off (full resolution)", ID_DOWN_OFF),
        ("720p", "Max 720p (faster, less RAM)", ID_DOWN_720),
        ("540p", "Max 540p (fastest, least RAM)", ID_DOWN_540),
    ] {
        let f = if cfg.downscale == mode {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        AppendMenuW(down, f, id, PCWSTR(wide(label).as_ptr()))
            .map_err(|e| format!("AppendMenuW: {e}"))?;
    }
    item(
        menu,
        MF_POPUP | MF_STRING,
        down.0 as usize,
        w!("Decode downscale"),
    )?;

    let hw = CreatePopupMenu().map_err(|e| format!("CreatePopupMenu: {e}"))?;
    for (mode, label, id) in [
        ("auto", "Auto (recommended)", ID_HW_AUTO),
        ("d3d11va-copy", "d3d11va-copy", ID_HW_D3D11VA),
        ("dxva2-copy", "dxva2-copy", ID_HW_DXVA2),
        ("off", "Off (software decode)", ID_HW_OFF),
    ] {
        let f = if cfg.hwdec == mode {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        AppendMenuW(hw, f, id, PCWSTR(wide(label).as_ptr()))
            .map_err(|e| format!("AppendMenuW: {e}"))?;
    }
    item(
        menu,
        MF_POPUP | MF_STRING,
        hw.0 as usize,
        w!("Hardware decode"),
    )?;

    let lite = if cfg.lite {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    item(menu, lite, ID_LITE, w!("Lite mode (low GPU/RAM)"))?;

    let vsync = if cfg.vsync {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    item(menu, vsync, ID_VSYNC, w!("V-sync (smooth playback)"))?;

    let smart = if cfg.smart_pause {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    item(
        menu,
        smart,
        ID_SMART,
        w!("Pause when desktop hidden (fullscreen/maximized)"),
    )?;

    let dim = if cfg.dim_when_hidden {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    item(
        menu,
        dim,
        ID_DIM,
        w!("Blur when hidden (smooth fade)"),
    )?;

    let battery = if cfg.pause_on_battery {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    item(
        menu,
        battery,
        ID_BATTERY,
        w!("Pause on battery power"),
    )?;

    let sub = CreatePopupMenu().map_err(|e| format!("CreatePopupMenu: {e}"))?;
    let mp = if cfg.monitor_mode != "all" {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    AppendMenuW(sub, mp, ID_MON_PRIMARY, w!("Primary monitor"))
        .map_err(|e| format!("AppendMenuW: {e}"))?;
    let ma = if cfg.monitor_mode == "all" {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    AppendMenuW(sub, ma, ID_MON_ALL, w!("All monitors"))
        .map_err(|e| format!("AppendMenuW: {e}"))?;
    item(menu, MF_POPUP | MF_STRING, sub.0 as usize, w!("Monitor mode"))?;

    let click = if cfg.click_through {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    item(menu, click, ID_CLICK, w!("Click through desktop"))?;
    let auto = if cfg.autostart {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    item(menu, auto, ID_AUTOSTART, w!("Start with Windows"))?;
    item(menu, MF_SEPARATOR, 0, PCWSTR::null())?;
    item(menu, MF_STRING, ID_QUIT, w!("Quit"))?;

    Ok(menu)
}

fn change_volume(shared: &Arc<AppShared>, delta: i64) {
    let mut cfg = shared.cfg.lock().unwrap();
    cfg.volume = (cfg.volume + delta).clamp(0, 150);
    let v = cfg.volume;
    let _ = config::save(&cfg);
    drop(cfg);
    let _ = shared.tx.send(Cmd::SetVolume { v });
}

fn set_monitor(shared: &Arc<AppShared>, mode: &str) {
    let mut cfg = shared.cfg.lock().unwrap();
    cfg.monitor_mode = mode.to_string();
    let _ = config::save(&cfg);
    drop(cfg);
    let _ = shared.tx.send(Cmd::Restart);
}

fn set_fit(shared: &Arc<AppShared>, mode: &str) {
    let mut cfg = shared.cfg.lock().unwrap();
    cfg.fit = mode.to_string();
    let _ = config::save(&cfg);
    drop(cfg);
    let _ = shared.tx.send(Cmd::SetFit {
        mode: mode.to_string(),
    });
}

fn set_renderer(shared: &Arc<AppShared>, name: &str) {
    let mut cfg = shared.cfg.lock().unwrap();
    cfg.renderer = name.to_string();
    let _ = config::save(&cfg);
    drop(cfg);
    let _ = shared.tx.send(Cmd::SetRenderer {
        name: name.to_string(),
    });
}

fn set_downscale(shared: &Arc<AppShared>, mode: &str) {
    let mut cfg = shared.cfg.lock().unwrap();
    cfg.downscale = mode.to_string();
    let _ = config::save(&cfg);
    drop(cfg);
    let _ = shared.tx.send(Cmd::SetDownscale {
        mode: mode.to_string(),
    });
}

fn set_hwdec(shared: &Arc<AppShared>, mode: &str) {
    let mut cfg = shared.cfg.lock().unwrap();
    cfg.hwdec = mode.to_string();
    let _ = config::save(&cfg);
    drop(cfg);
    let _ = shared.tx.send(Cmd::SetHwdec {
        mode: mode.to_string(),
    });
}

fn handle_menu(shared: &Arc<AppShared>, id: usize) {
    match id {
        ID_PICK => {
            if let Some(path) = pick_video() {
                {
                    let mut cfg = shared.cfg.lock().unwrap();
                    cfg.video = Some(path.clone());
                    let _ = config::save(&cfg);
                }
                let _ = shared.tx.send(Cmd::Load { path });
            }
        }
        ID_AUDIO => {
            let on = {
                let mut cfg = shared.cfg.lock().unwrap();
                cfg.audio = !cfg.audio;
                let _ = config::save(&cfg);
                cfg.audio
            };
            let _ = shared.tx.send(Cmd::SetAudio { on });
        }
        ID_VOL_UP => change_volume(shared, 10),
        ID_VOL_DOWN => change_volume(shared, -10),
        ID_FIT_FILL => set_fit(shared, "fill"),
        ID_FIT_FIT => set_fit(shared, "fit"),
        ID_FIT_CENTER => set_fit(shared, "center"),
        ID_FIT_STRETCH => set_fit(shared, "stretch"),
        ID_REND_D3D11 => set_renderer(shared, "d3d11"),
        ID_REND_VULKAN => set_renderer(shared, "vulkan"),
        ID_REND_ANGLE => set_renderer(shared, "angle"),
        ID_DOWN_OFF => set_downscale(shared, "off"),
        ID_DOWN_720 => set_downscale(shared, "720p"),
        ID_DOWN_540 => set_downscale(shared, "540p"),
        ID_HW_AUTO => set_hwdec(shared, "auto"),
        ID_HW_D3D11VA => set_hwdec(shared, "d3d11va-copy"),
        ID_HW_DXVA2 => set_hwdec(shared, "dxva2-copy"),
        ID_HW_OFF => set_hwdec(shared, "off"),
        ID_LITE => {
            let on = {
                let mut cfg = shared.cfg.lock().unwrap();
                cfg.lite = !cfg.lite;
                let _ = config::save(&cfg);
                cfg.lite
            };
            let _ = shared.tx.send(Cmd::SetLite { on });
        }
        ID_VSYNC => {
            let on = {
                let mut cfg = shared.cfg.lock().unwrap();
                cfg.vsync = !cfg.vsync;
                let _ = config::save(&cfg);
                cfg.vsync
            };
            let _ = shared.tx.send(Cmd::SetVsync { on });
        }
        ID_SMART => {
            let mut cfg = shared.cfg.lock().unwrap();
            cfg.smart_pause = !cfg.smart_pause;
            let _ = config::save(&cfg);
        }
        ID_DIM => {
            {
                let mut cfg = shared.cfg.lock().unwrap();
                cfg.dim_when_hidden = !cfg.dim_when_hidden;
                let _ = config::save(&cfg);
            }

            let _ = shared.tx.send(Cmd::Restart);
        }
        ID_BATTERY => {
            let mut cfg = shared.cfg.lock().unwrap();
            cfg.pause_on_battery = !cfg.pause_on_battery;
            let _ = config::save(&cfg);
        }
        ID_MON_PRIMARY => set_monitor(shared, "primary"),
        ID_MON_ALL => set_monitor(shared, "all"),
        ID_CLICK => {
            let on = {
                let mut cfg = shared.cfg.lock().unwrap();
                cfg.click_through = !cfg.click_through;
                let _ = config::save(&cfg);
                cfg.click_through
            };
            wallpaper::set_click_through(on);
        }
        ID_AUTOSTART => {
            let on = {
                let mut cfg = shared.cfg.lock().unwrap();
                cfg.autostart = !cfg.autostart;
                let _ = config::save(&cfg);
                cfg.autostart
            };
            if let Err(e) = set_autostart(on) {
                log::log(&format!("autostart: {e}"));
            }
        }
        ID_QUIT => {
            let _ = shared.tx.send(Cmd::Quit);
            unsafe { PostQuitMessage(0) };
        }
        _ => {}
    }
}

fn show_menu(shared: &Arc<AppShared>, hwnd: HWND) {
    let cfg = shared.cfg.lock().unwrap().clone();
    let menu = match unsafe { build_menu(&cfg) } {
        Ok(m) => m,
        Err(e) => {
            log::log(&format!("menu build: {e}"));
            return;
        }
    };
    let mut pt = POINT::default();
    let _ = unsafe { GetCursorPos(&mut pt) };
    unsafe {
        let _ = SetForegroundWindow(hwnd);
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
        let r = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
        let id = r.0 as usize;
        let _ = DestroyMenu(menu);
        if id != 0 {
            handle_menu(shared, id);
        }
    }
}

pub fn run(shared: Arc<AppShared>, player_thread: JoinHandle<()>) -> i32 {
    unsafe {
        let hmod = match GetModuleHandleW(None) {
            Ok(m) => m,
            Err(e) => {
                let text = wide(&format!("Failed to init: {e}"));
                MessageBoxW(
                    None,
                    PCWSTR(text.as_ptr()),
                    w!("Live Wallpaper"),
                    MB_OK | MB_ICONERROR,
                );
                return 1;
            }
        };
        let hinst = HINSTANCE(hmod.0);

        let class = w!("LiveWallpaperTray");
        let wc = WNDCLASSW {
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(tray_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: app_icon(),
            hCursor: HCURSOR::default(),
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: class,
        };
        RegisterClassW(&wc);

        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0),
            class,
            PCWSTR::null(),
            WINDOW_STYLE(WS_OVERLAPPED.0),
            0,
            0,
            0,
            0,
            None,
            None,
            hinst,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                let text = wide(&format!("Failed to create window: {e}"));
                MessageBoxW(
                    None,
                    PCWSTR(text.as_ptr()),
                    w!("Live Wallpaper"),
                    MB_OK | MB_ICONERROR,
                );
                return 1;
            }
        };

        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = app_icon();
        set_tip(&mut nid, "Live Wallpaper");

        if !Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
            let _ = MessageBoxW(
                None,
                w!("Failed to add tray icon."),
                w!("Live Wallpaper"),
                MB_OK | MB_ICONERROR,
            );
            let _ = DestroyWindow(hwnd);
            return 1;
        }
        TRAY_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

        match RegisterHotKey(None, HOTKEY_ID, MOD_CONTROL | MOD_ALT, VK_W.0 as u32) {
            Ok(()) => {}
            Err(e) => log::log(&format!("hotkey failed: {e}")),
        }

        let hook = match SetWindowsHookExW(WH_MOUSE_LL, Some(tray_ll_mouse), hinst, 0) {
            Ok(h) => Some(h),
            Err(e) => {
                log::log(&format!("ll mouse hook failed: {e}"));
                None
            }
        };

        let mut msg = MSG::default();
        loop {
            let got = GetMessageW(&mut msg, None, 0, 0);
            if !got.as_bool() {
                break;
            }
            if msg.message == WM_HOTKEY && msg.wParam.0 == HOTKEY_ID as usize {
                show_menu(&shared, hwnd);
            } else if msg.message == WM_SHOW_MENU && msg.hwnd == hwnd {
                show_menu(&shared, hwnd);
            } else if msg.message == WM_TRAY && msg.hwnd == hwnd {
                let lm = msg.lParam.0 as u32;
                if lm == WM_RBUTTONUP || lm == WM_CONTEXTMENU {
                    show_menu(&shared, hwnd);
                }
            } else {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        if let Some(h) = hook {
            let _ = UnhookWindowsHookEx(h);
        }
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        let _ = DestroyWindow(hwnd);
        let _ = player_thread.join();
        0
    }
}
