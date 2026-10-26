use std::sync::atomic::{AtomicBool, Ordering};
use std::os::raw::{c_void, CGFloat};

pub static SURFACE_DESTROYED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug)]
pub struct Monitor {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub fn set_dpi_awareness() {
    // Not needed on macOS; the display server handles scaling.
}

// Re-export cocoa types used by other modules.
pub use cocoa::appkit::{NSWindow, NSWindowLevel};
pub use cocoa::base::{id, nil};
pub use cocoa::foundation::{NSPoint, NSRect, NSSize};

/// Constants for macOS window levels.
pub const kCGDesktopWindowLevel: NSWindowLevel = NSWindowLevel(3); // kCGDesktopWindowLevel = 3

/// Create a borderless desktop-level window for mpv rendering.
/// Returns the window pointer (`id`) that can be passed to mpv as `wid`.
pub fn create_desktop_window(x: i32, y: i32, w: i32, h: i32) -> Option<id> {
    unsafe {
        let app = cocoa::appkit::NSApp();

        // Create a borderless window at desktop level.
        let rect = NSRect::new(
            NSPoint::new(x as CGFloat, y as CGFloat),
            NSSize::new(w as CGFloat, h as CGFloat),
        );
        let mask = cocoa::appkit::NSBorderlessWindowMask;
        let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            rect,
            mask,
            cocoa::appkit::NSBackingStoreBuffered,
            false,
        );

        // Set window level to desktop (behind icons).
        window.setLevel_(kCGDesktopWindowLevel);

        // Make it transparent and ignore mouse events (click-through).
        window.setOpaque_(false);
        window.setBackgroundColor_(cocoa::appkit::NSColor::clearColor(nil));
        window.setIgnoresMouseEvents_(true);

        // Disable titlebar and shadows.
        window.setTitlebarAppearsTransparent_(true);
        window.setTitleVisibility_(cocoa::appkit::NSWindowTitleVisibility::NSWindowTitleHidden);
        window.setHasShadow_(false);

        // Collect the window so it stays alive.
        let _: () = msg_send![window, retain];

        Some(window)
    }
}

/// Destroy a desktop window created by `create_desktop_window`.
pub unsafe fn destroy_desktop_window(window: id) {
    let _: () = msg_send![window, release];
}

/// Get the list of displays (monitors) on macOS.
pub fn monitor_list() -> Vec<Monitor> {
    unsafe {
        let mut monitors: Vec<Monitor> = Vec::new();

        let display_count: u32 = 0;
        let mut active_displays: Vec<core_graphics::display::CGDirectDisplayID> = Vec::new();

        // Get number of displays first.
        let mut count = 0u32;
        let err = core_graphics::display::CGGetActiveDisplayList(
            0,
            std::ptr::null_mut(),
            &mut count,
        );
        if err != core_graphics::error::CGError::success {
            return monitors;
        }

        active_displays.resize(count as usize, 0);
        let err = core_graphics::display::CGGetActiveDisplayList(
            count,
            active_displays.as_mut_ptr(),
            &mut count,
        );
        if err != core_graphics::error::CGError::success {
            return monitors;
        }

        for &display_id in active_displays.iter().take(count as usize) {
            let bounds = core_graphics::display::CGDisplayBounds(display_id);
            monitors.push(Monitor {
                x: bounds.origin.x as i32,
                y: bounds.origin.y as i32,
                w: bounds.size.width as i32,
                h: bounds.size.height as i32,
            });
        }

        monitors
    }
}

/// Check if any window covers the full screen (heuristic).
/// On macOS this is limited without Accessibility API permissions.
pub fn desktop_covered() -> bool {
    // Without Accessibility API permissions, we cannot reliably detect
    // fullscreen windows on macOS. Return false so the wallpaper keeps playing.
    // Users can grant Accessibility permissions in System Settings for future improvements.
    false
}