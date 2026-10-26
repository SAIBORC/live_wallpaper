use crate::{config, log, AppShared, Cmd};
use cocoa::appkit::{NSApplication, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use cocoa::objc::{msg_send, sel, sel_impl};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;

// Global pointer to AppShared, set by main() before starting the UI.
// AppShared contains Sender<Cmd> (cheap to clone) and Arc<Mutex<Config>>.
static SHARED_PTR: std::sync::atomic::AtomicPtr<c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

pub const APP_NAME: &str = "Live Wallpaper";

/// Call once from main() before ui_macos::run().
pub fn set_shared_state(shared: &Arc<AppShared>) {
    let ptr = Arc::as_ptr(shared) as *mut c_void;
    SHARED_PTR.store(ptr, Ordering::SeqCst);
}

fn with_shared<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&AppShared) -> R,
{
    let ptr = SHARED_PTR.load(Ordering::SeqCst);
    if ptr.is_null() {
        return None;
    }
    // SAFETY: The owning Arc lives in main() for the entire process lifetime.
    // We only borrow, never drop or clone the Arc here.
    let shared = unsafe { &*ptr };
    Some(f(shared))
}

pub fn run(_shared: Arc<AppShared>, _player_thread: thread::JoinHandle<()>) -> i32 {
    // The shared state is set by main() before this is called.
    unsafe {
        let pool = NSAutoreleasePool::new(nil);

        let app = NSApplication::sharedApplication(nil);

        // Status bar item.
        let status_bar = NSStatusBar::systemStatusBar(nil);
        let status_item: id = msg_send![status_bar, statusItemWithLength: cocoa::appkit::NSVariableStatusItemLength];

        let button: id = msg_send![status_item, button];
        let title = NSString::alloc(nil).init_str(APP_NAME);
        let _: () = msg_send![button, setTitle: title];

        // Menu.
        let menu = NSMenu::alloc(nil).initWithTitle_(NSString::alloc(nil).init_str(APP_NAME));

        // ---- Video title (read-only) ----
        let title_str = with_shared(|s| {
            let cfg = s.cfg.lock().unwrap();
            cfg.video
                .as_deref()
                .and_then(|p| std::path::Path::new(p).file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "No video selected".to_string())
        }).unwrap_or_else(|| "No video selected".to_string());
        let title_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            NSString::alloc(nil).init_str(&title_str),
            None,
            NSString::alloc(nil).init_str(""),
        );
        let _: () = msg_send![title_item, setEnabled: false];
        menu.addItem_(title_item);

        let sep = NSMenuItem::separatorItem(nil);
        menu.addItem_(sep);

        // ---- Pick video ----
        let pick_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            NSString::alloc(nil).init_str("Pick video..."),
            sel!(pickVideo:),
            NSString::alloc(nil).init_str(""),
        );
        let _: () = msg_send![pick_item, setTarget: status_item];
        menu.addItem_(pick_item);

        let sep = NSMenuItem::separatorItem(nil);
        menu.addItem_(sep);

        // ---- Play audio ----
        let audio_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            NSString::alloc(nil).init_str("Play audio"),
            sel!(toggleAudio:),
            NSString::alloc(nil).init_str(""),
        );
        {
            let on = with_shared(|s| {
                let cfg = s.cfg.lock().unwrap();
                cfg.audio
            }).unwrap_or(true);
            let _: () = msg_send![audio_item, setState: if on { 1 } else { 0 }];
        }
        let _: () = msg_send![audio_item, setTarget: status_item];
        menu.addItem_(audio_item);

        // ---- Volume +10 ----
        let vol_up = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            NSString::alloc(nil).init_str("Volume +10"),
            sel!(volumeUp:),
            NSString::alloc(nil).init_str(""),
        );
        let _: () = msg_send![vol_up, setTarget: status_item];
        menu.addItem_(vol_up);

        // ---- Volume -10 ----
        let vol_down = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            NSString::alloc(nil).init_str("Volume -10"),
            sel!(volumeDown:),
            NSString::alloc(nil).init_str(""),
        );
        let _: () = msg_send![vol_down, setTarget: status_item];
        menu.addItem_(vol_down);

        let sep = NSMenuItem::separatorItem(nil);
        menu.addItem_(sep);

        // ---- Video fit ----
        let fit_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            NSString::alloc(nil).init_str("Video fit"),
            None,
            NSString::alloc(nil).init_str(""),
        );
        let fit_menu = NSMenu::alloc(nil).initWithTitle_(NSString::alloc(nil).init_str("Video fit"));
        for (mode, label) in [
            ("fill", "Fill (crop to screen)"),
            ("fit", "Fit (whole video visible)"),
            ("center", "Center (original size)"),
            ("stretch", "Stretch (fill and distort)"),
        ] {
            let mi = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                NSString::alloc(nil).init_str(label),
                sel!(setFit:),
                NSString::alloc(nil).init_str(""),
            );
            let mode_ns = NSString::alloc(nil).init_str(mode);
            let _: () = msg_send![mi, setRepresentedObject: mode_ns];
            let _: () = msg_send![mi, setTarget: status_item];
            fit_menu.addItem_(mi);
        }
        let _: () = msg_send![fit_item, setSubmenu: fit_menu];
        menu.addItem_(fit_item);

        let sep = NSMenuItem::separatorItem(nil);
        menu.addItem_(sep);

        // ---- Lite mode ----
        let lite_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            NSString::alloc(nil).init_str("Lite mode (low GPU/RAM)"),
            sel!(toggleLite:),
            NSString::alloc(nil).init_str(""),
        );
        {
            let on = with_shared(|s| {
                let cfg = s.cfg.lock().unwrap();
                cfg.lite
            }).unwrap_or(false);
            let _: () = msg_send![lite_item, setState: if on { 1 } else { 0 }];
        }
        let _: () = msg_send![lite_item, setTarget: status_item];
        menu.addItem_(lite_item);

        // ---- V-sync ----
        let vsync_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            NSString::alloc(nil).init_str("V-sync (smooth playback)"),
            sel!(toggleVsync:),
            NSString::alloc(nil).init_str(""),
        );
        {
            let on = with_shared(|s| {
                let cfg = s.cfg.lock().unwrap();
                cfg.vsync
            }).unwrap_or(true);
            let _: () = msg_send![vsync_item, setState: if on { 1 } else { 0 }];
        }
        let _: () = msg_send![vsync_item, setTarget: status_item];
        menu.addItem_(vsync_item);

        let sep = NSMenuItem::separatorItem(nil);
        menu.addItem_(sep);

        // ---- Smart pause ----
        let smart_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            NSString::alloc(nil).init_str("Pause when desktop hidden"),
            sel!(toggleSmart:),
            NSString::alloc(nil).init_str(""),
        );
        {
            let on = with_shared(|s| {
                let cfg = s.cfg.lock().unwrap();
                cfg.smart_pause
            }).unwrap_or(true);
            let _: () = msg_send![smart_item, setState: if on { 1 } else { 0 }];
        }
        let _: () = msg_send![smart_item, setTarget: status_item];
        menu.addItem_(smart_item);

        let sep = NSMenuItem::separatorItem(nil);
        menu.addItem_(sep);

        // ---- Quit ----
        let quit_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            NSString::alloc(nil).init_str("Quit"),
            sel!(quitApp:),
            NSString::alloc(nil).init_str(""),
        );
        let _: () = msg_send![quit_item, setTarget: status_item];
        menu.addItem_(quit_item);

        let _: () = msg_send![status_item, setMenu: menu];

        // Accessory policy: no dock icon.
        app.setActivationPolicy_(cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);

        app.run();

        pool.drain();
    }

    0
}

// ----- Menu action callbacks -----

#[allow(non_snake_case)]
unsafe extern "C" fn pickVideo(_sender: id) {
    if let Some(shared) = with_shared(|s| s.clone()) {
        if let Some(path) = pick_video_macos() {
            {
                let mut cfg = shared.cfg.lock().unwrap();
                cfg.video = Some(path.clone());
                let _ = config::save(&cfg);
            }
            let _ = shared.tx.send(Cmd::Load { path });
        }
    }
}

#[allow(non_snake_case)]
unsafe extern "C" fn toggleAudio(_sender: id) {
    if let Some(shared) = with_shared(|s| s.clone()) {
        let on = {
            let mut cfg = shared.cfg.lock().unwrap();
            cfg.audio = !cfg.audio;
            let on = cfg.audio;
            let _ = config::save(&cfg);
            on
        };
        let _ = shared.tx.send(Cmd::SetAudio { on });
    }
}

#[allow(non_snake_case)]
unsafe extern "C" fn volumeUp(_sender: id) {
    if let Some(shared) = with_shared(|s| s.clone()) {
        let v = {
            let mut cfg = shared.cfg.lock().unwrap();
            cfg.volume = (cfg.volume + 10).clamp(0, 150);
            let v = cfg.volume;
            let _ = config::save(&cfg);
            v
        };
        let _ = shared.tx.send(Cmd::SetVolume { v });
    }
}

#[allow(non_snake_case)]
unsafe extern "C" fn volumeDown(_sender: id) {
    if let Some(shared) = with_shared(|s| s.clone()) {
        let v = {
            let mut cfg = shared.cfg.lock().unwrap();
            cfg.volume = (cfg.volume - 10).clamp(0, 150);
            let v = cfg.volume;
            let _ = config::save(&cfg);
            v
        };
        let _ = shared.tx.send(Cmd::SetVolume { v });
    }
}

#[allow(non_snake_case)]
unsafe extern "C" fn setFit(_sender: id) {
    if let Some(shared) = with_shared(|s| s.clone()) {
        let obj: id = msg_send![_sender, representedObject];
        let ns_str: id = obj;
        // Get UTF8 string from NSString.
        let c_str: id = msg_send![ns_str, UTF8String];
        let bytes = c_str as *const u8;
        let mode_str = if !bytes.is_null() {
            let len = (0..).position(|i| *bytes.add(i) == 0).unwrap_or(0);
            std::str::from_utf8(std::slice::from_raw_parts(bytes, len))
                .unwrap_or("fill")
                .to_string()
        } else {
            "fill".to_string()
        };
        {
            let mut cfg = shared.cfg.lock().unwrap();
            cfg.fit = mode_str.clone();
            let _ = config::save(&cfg);
        }
        let _ = shared.tx.send(Cmd::SetFit { mode: mode_str });
    }
}

#[allow(non_snake_case)]
unsafe extern "C" fn toggleLite(_sender: id) {
    if let Some(shared) = with_shared(|s| s.clone()) {
        let on = {
            let mut cfg = shared.cfg.lock().unwrap();
            cfg.lite = !cfg.lite;
            let on = cfg.lite;
            let _ = config::save(&cfg);
            on
        };
        let _ = shared.tx.send(Cmd::SetLite { on });
    }
}

#[allow(non_snake_case)]
unsafe extern "C" fn toggleVsync(_sender: id) {
    if let Some(shared) = with_shared(|s| s.clone()) {
        let on = {
            let mut cfg = shared.cfg.lock().unwrap();
            cfg.vsync = !cfg.vsync;
            let on = cfg.vsync;
            let _ = config::save(&cfg);
            on
        };
        let _ = shared.tx.send(Cmd::SetVsync { on });
    }
}

#[allow(non_snake_case)]
unsafe extern "C" fn toggleSmart(_sender: id) {
    if let Some(shared) = with_shared(|s| s.clone()) {
        let on = {
            let mut cfg = shared.cfg.lock().unwrap();
            cfg.smart_pause = !cfg.smart_pause;
            let _ = config::save(&cfg);
            cfg.smart_pause
        };
        let _ = shared.tx.send(Cmd::SetLite { on });
    }
}

#[allow(non_snake_case)]
unsafe extern "C" fn quitApp(_sender: id) {
    if let Some(shared) = with_shared(|s| s.clone()) {
        let _ = shared.tx.send(Cmd::Quit);
    }
    NSApplication::sharedApplication(nil).terminate_(nil);
}

/// Open a macOS file picker and return the selected path.
fn pick_video_macos() -> Option<String> {
    use cocoa::appkit::NSOpenPanel;
    use cocoa::foundation::NSArray;

    unsafe {
        let panel = NSOpenPanel::openPanel(nil);
        panel.setCanChooseFiles_(true);
        panel.setCanChooseDirectories_(false);
        panel.setAllowsMultipleSelection_(false);

        let filter = NSArray::arrayWithObject_(nil, NSString::alloc(nil).init_str("public.video"));
        panel.setAllowedFileTypes_(filter);

        let result = panel.runModal();
        if result == cocoa::appkit::NSFileHandlingPanelOKButton {
            let url: id = msg_send![panel, URL];
            if !url.is_null() {
                let path: id = msg_send![url, path];
                let bytes = path.UTF8String() as *const u8;
                if bytes.is_null() {
                    return None;
                }
                let len = (0..).position(|i| *bytes.add(i) == 0).unwrap_or(0);
                let s = std::str::from_utf8(std::slice::from_raw_parts(bytes, len)).ok()?;
                return Some(s.to_string());
            }
        }
        None
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_shared_state(_shared: &Arc<AppShared>) {}

#[cfg(not(target_os = "macos"))]
pub fn run(_shared: Arc<AppShared>, _player_thread: thread::JoinHandle<()>) -> i32 {
    0
}