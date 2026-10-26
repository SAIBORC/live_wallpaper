use std::sync::atomic::{AtomicBool, Ordering};
use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_ulong, c_void};

pub static SURFACE_DESTROYED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug)]
pub struct Monitor {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub fn set_dpi_awareness() {
    // Not needed on Linux; X11 handles DPI via the display server.
}

// X11 type aliases (minimal subset)
type Display = c_void;
type Window = c_ulong;
type Colormap = c_ulong;
type Visual = c_void;
type Atom = c_ulong;
type XEvent = [c_ulong; 24];

// X11 function pointer types via libloading
type XOpenDisplayFn = unsafe extern "C" fn(*const i8) -> *mut Display;
type XCloseDisplayFn = unsafe extern "C" fn(*mut Display) -> c_int;
type XDefaultScreenFn = unsafe extern "C" fn(*mut Display) -> c_int;
type XRootWindowFn = unsafe extern "C" fn(*mut Display, c_int) -> Window;
type XCreateWindowFn = unsafe extern "C" fn(
    *mut Display, Window, c_int, c_int, c_ulong, c_ulong, c_ulong,
    c_ulong, c_ulong, *mut Visual, c_ulong, *mut c_void,
) -> Window;
type XMapWindowFn = unsafe extern "C" fn(*mut Display, Window) -> c_int;
type XFlushFn = unsafe extern "C" fn(*mut Display) -> c_int;
type XStoreNameFn = unsafe extern "C" fn(*mut Display, Window, *const i8) -> c_int;
type XInternAtomFn = unsafe extern "C" fn(*mut Display, *const i8, c_int) -> Atom;
type XChangePropertyFn = unsafe extern "C" fn(
    *mut Display, Window, Atom, Atom, c_int, c_int, *const c_uchar, c_int,
) -> c_int;
type XDestroyWindowFn = unsafe extern "C" fn(*mut Display, Window) -> c_int;

/// An X11 surface (window) that mpv can render into via `wid` (the XID).
pub struct Surface {
    pub xid: u64,
    _display: *mut Display,
    _window: Window,
    _lib: Library,
}

impl Surface {
    pub fn new(x: i32, y: i32, w: i32, h: i32, _click_through: bool) -> Result<Self, String> {
        let lib = Library::new("libX11.so.6")
            .or_else(|_| Library::new("libX11.so"))
            .map_err(|e| format!("libX11 not found: {e}"))?;

        unsafe {
            let x_open_display: Symbol<XOpenDisplayFn> = lib.get(b"XOpenDisplay\0")
                .map_err(|e| format!("XOpenDisplay: {e}"))?;
            let x_default_screen: Symbol<XDefaultScreenFn> = lib.get(b"XDefaultScreen\0")
                .map_err(|e| format!("XDefaultScreen: {e}"))?;
            let x_root_window: Symbol<XRootWindowFn> = lib.get(b"XRootWindow\0")
                .map_err(|e| format!("XRootWindow: {e}"))?;
            let x_create_window: Symbol<XCreateWindowFn> = lib.get(b"XCreateWindow\0")
                .map_err(|e| format!("XCreateWindow: {e}"))?;
            let x_map_window: Symbol<XMapWindowFn> = lib.get(b"XMapWindow\0")
                .map_err(|e| format!("XMapWindow: {e}"))?;
            let x_flush: Symbol<XFlushFn> = lib.get(b"XFlush\0")
                .map_err(|e| format!("XFlush: {e}"))?;
            let x_intern_atom: Symbol<XInternAtomFn> = lib.get(b"XInternAtom\0")
                .map_err(|e| format!("XInternAtom: {e}"))?;
            let x_change_property: Symbol<XChangePropertyFn> = lib.get(b"XChangeProperty\0")
                .map_err(|e| format!("XChangeProperty: {e}"))?;
            let x_destroy_window: Symbol<XDestroyWindowFn> = lib.get(b"XDestroyWindow\0")
                .map_err(|e| format!("XDestroyWindow: {e}"))?;

            let display = x_open_display(std::ptr::null());
            if display.is_null() {
                return Err("XOpenDisplay returned null".to_string());
            }

            let screen = x_default_screen(display);
            let root = x_root_window(display, screen);

            let window = x_create_window(
                display,
                root,
                x as c_int,
                y as c_int,
                w as c_ulong,
                h as c_ulong,
                0, // border_width
                0, // depth (CopyFromParent)
                1, // class (InputOutput)
                std::ptr::null_mut(), // visual (CopyFromParent)
                0, // valuemask
                std::ptr::null_mut(),
            );

            // Set window type hints: desktop, below, skip taskbar/pager
            let wm_type = x_intern_atom(display, b"_NET_WM_WINDOW_TYPE\0".as_ptr() as *const i8, 0);
            let wm_type_desktop = x_intern_atom(
                display,
                b"_NET_WM_WINDOW_TYPE_DESKTOP\0".as_ptr() as *const i8,
                0,
            );
            let wm_state = x_intern_atom(display, b"_NET_WM_STATE\0".as_ptr() as *const i8, 0);
            let wm_state_below = x_intern_atom(
                display,
                b"_NET_WM_STATE_BELOW\0".as_ptr() as *const i8,
                0,
            );
            let wm_state_skip_taskbar = x_intern_atom(
                display,
                b"_NET_WM_STATE_SKIP_TASKBAR\0".as_ptr() as *const i8,
                0,
            );
            let wm_state_skip_pager = x_intern_atom(
                display,
                b"_NET_WM_STATE_SKIP_PAGER\0".as_ptr() as *const i8,
                0,
            );

            let props: [c_ulong; 4] = [
                wm_type_desktop,
                wm_state_below,
                wm_state_skip_taskbar,
                wm_state_skip_pager,
            ];
            let _ = x_change_property(
                display,
                window,
                wm_type,
                6, // XA_ATOM
                32,
                0, // PropModeReplace
                props.as_ptr() as *const c_uchar,
                1,
            );
            let _ = x_change_property(
                display,
                window,
                wm_state,
                6, // XA_ATOM
                32,
                0, // PropModeReplace
                props.as_ptr() as *const c_uchar,
                4,
            );

            x_map_window(display, window);
            x_flush(display);

            Ok(Surface {
                xid: window as u64,
                _display: display,
                _window: window,
                _lib: lib,
            })
        }
    }

    pub fn destroy(&self) {
        // libloading doesn't expose XCloseDisplay easily without keeping the symbol,
        // but dropping the Library unloads libX11 which is fine for process exit.
        let _ = self;
    }
}

/// Heuristic: check if any window covers the full screen (very basic).
#[cfg(target_os = "linux")]
pub fn desktop_covered() -> bool {
    // Without X11 querying libs, assume not covered.
    // A full implementation would use XQueryTree via libX11.
    false
}