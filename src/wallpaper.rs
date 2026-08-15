use std::os::raw::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, GetStockObject, BLACK_BRUSH, HBRUSH, HDC, HMONITOR,
    MONITORINFO, MONITOR_DEFAULTTONEAREST, MonitorFromPoint,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::*;

pub static SURFACE_DESTROYED: AtomicBool = AtomicBool::new(false);
static CLICK_THROUGH: AtomicBool = AtomicBool::new(true);

pub fn set_click_through(on: bool) {
    CLICK_THROUGH.store(on, Ordering::Relaxed);
}

const WM_SPAWN_WORKERW: u32 = 0x052C;

#[derive(Clone, Copy, Debug)]
pub struct Monitor {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub fn set_dpi_awareness() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn monitor_list() -> Vec<Monitor> {
    let mut out: Vec<Monitor> = Vec::new();
    unsafe extern "system" fn cb(hmon: HMONITOR, _hdc: HDC, _rc: *mut RECT, lparam: LPARAM) -> BOOL {
        let mut mi = MONITORINFO::default();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            let r = mi.rcMonitor;
            let list = lparam.0 as *mut Vec<Monitor>;
            (*list).push(Monitor {
                x: r.left,
                y: r.top,
                w: r.right - r.left,
                h: r.bottom - r.top,
            });
        }
        BOOL::from(true)
    }
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(cb),
            LPARAM(&mut out as *mut Vec<Monitor> as isize),
        );
    }
    out
}

fn find_shell_parent() -> Option<HWND> {
    let mut found: isize = 0;
    unsafe extern "system" fn cb(h: HWND, lparam: LPARAM) -> BOOL {
        let has_icons = FindWindowExW(h, None, w!("SHELLDLL_DefView"), PCWSTR::null())
            .map(|x| !x.0.is_null())
            .unwrap_or(false);
        if has_icons {
            unsafe {
                *(lparam.0 as *mut isize) = h.0 as isize;
            }
        }
        BOOL::from(true)
    }
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut found as *mut isize as isize));
        if found != 0 {
            Some(HWND(found as *mut c_void))
        } else {
            None
        }
    }
}

fn find_workerw_child_of(progman: HWND) -> Option<HWND> {
    unsafe {
        let mut child = GetWindow(progman, GW_CHILD).ok()?;
        loop {
            let mut buf = [0u16; 256];
            let n = GetClassNameW(child, &mut buf);
            if n > 0 {
                let name = String::from_utf16_lossy(&buf[..n as usize]);
                if name == "WorkerW" && IsWindowVisible(child).as_bool() {
                    let has_icons = FindWindowExW(child, None, w!("SHELLDLL_DefView"), PCWSTR::null())
                        .map(|x| !x.0.is_null())
                        .unwrap_or(false);
                    if !has_icons {
                        return Some(child);
                    }
                }
            }
            match GetWindow(child, GW_HWNDNEXT) {
                Ok(next) if !next.0.is_null() => child = next,
                _ => return None,
            }
        }
    }
}

pub fn find_workerw() -> Result<HWND, String> {
    unsafe {
        let progman = FindWindowW(w!("Progman"), PCWSTR::null())
            .map_err(|_| "Progman not found (interactive desktop not available?)".to_string())?;
        if progman.0.is_null() {
            return Err("Progman not found".into());
        }

        let mut result: usize = 0;
        SendMessageTimeoutW(
            progman,
            WM_SPAWN_WORKERW,
            WPARAM(0),
            LPARAM(0),
            SMTO_NORMAL,
            1000,
            Some(&mut result),
        );

        if let Some(shell_parent) = find_shell_parent() {
            let ww = FindWindowExW(None, shell_parent, w!("WorkerW"), PCWSTR::null())
                .map(|x| if x.0.is_null() { None } else { Some(x) })
                .ok()
                .flatten();
            if let Some(ww) = ww {
                return Ok(ww);
            }
        }

        if let Some(ww) = find_workerw_child_of(progman) {
            return Ok(ww);
        }

        Ok(progman)
    }
}

unsafe extern "system" fn surface_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => SURFACE_DESTROYED.store(true, Ordering::SeqCst),
        WM_NCHITTEST if CLICK_THROUGH.load(Ordering::Relaxed) => {
            return LRESULT(HTTRANSPARENT as isize);
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

pub fn spawn_surface(parent: HWND, m: &Monitor, click_through: bool) -> Result<HWND, String> {
    CLICK_THROUGH.store(click_through, Ordering::Relaxed);
    unsafe {
        let hmod = GetModuleHandleW(None).map_err(|e| format!("GetModuleHandleW: {e}"))?;
        let hinst = HINSTANCE(hmod.0);
        let class = w!("LiveWallpaperSurface");

        let wc = WNDCLASSW {
            style: WNDCLASS_STYLES(CS_HREDRAW.0 | CS_VREDRAW.0),
            lpfnWndProc: Some(surface_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: crate::ui::app_icon(),
            hCursor: HCURSOR::default(),
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: class,
        };
        RegisterClassW(&wc);

        let ex_style = WINDOW_EX_STYLE(WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0);

        let hwnd = CreateWindowExW(
            ex_style,
            class,
            PCWSTR::null(),
            WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_CLIPSIBLINGS.0),
            m.x,
            m.y,
            m.w,
            m.h,
            parent,
            None,
            hinst,
            None,
        )
        .map_err(|e| format!("CreateWindowExW surface: {e}"))?;

        let _ = SetWindowPos(
            hwnd,
            HWND_BOTTOM,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        Ok(hwnd)
    }
}

fn class_of(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut buf = [0u16; 256];
        let n = GetClassNameW(hwnd, &mut buf);
        if n > 0 {
            Some(String::from_utf16_lossy(&buf[..n as usize]))
        } else {
            None
        }
    }
}

fn is_background_class(hwnd: HWND) -> bool {
    matches!(
        class_of(hwnd).as_deref(),
        Some(
            "Progman"
                | "WorkerW"
                | "Shell_TrayWnd"
                | "Shell_SecondaryTrayWnd"
                | "LiveWallpaperTray"
                | "LiveWallpaperSurface"
        )
    )
}

fn is_fullscreen_window(hwnd: HWND) -> bool {
    unsafe {
        let mut wr = RECT::default();
        if GetWindowRect(hwnd, &mut wr).is_err() {
            return false;
        }
        let w = wr.right - wr.left;
        let h = wr.bottom - wr.top;
        if w <= 0 || h <= 0 {
            return false;
        }
        let mon = MonitorFromPoint(
            POINT {
                x: wr.left + w / 2,
                y: wr.top + h / 2,
            },
            MONITOR_DEFAULTTONEAREST,
        );
        let mut mi = MONITORINFO::default();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if !GetMonitorInfoW(mon, &mut mi).as_bool() {
            return false;
        }
        let m = mi.rcMonitor;
        const TOL: i32 = 8;
        wr.left <= m.left + TOL
            && wr.top <= m.top + TOL
            && wr.right >= m.right - TOL
            && wr.bottom >= m.bottom - TOL
    }
}

pub fn desktop_covered() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if !fg.0.is_null() && !is_background_class(fg) && is_fullscreen_window(fg) {
            return true;
        }
    }
    let mut covered = false;
    unsafe extern "system" fn cb(h: HWND, lparam: LPARAM) -> BOOL {
        if is_background_class(h) {
            return BOOL::from(true);
        }
        if IsWindowVisible(h).as_bool() && IsZoomed(h).as_bool() {
            unsafe {
                *(lparam.0 as *mut bool) = true;
            }
            return BOOL::from(false);
        }
        BOOL::from(true)
    }
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut covered as *mut bool as isize));
    }
    covered
}
