use crate::{config, log, AppShared, Cmd};
use std::sync::Arc;
use std::thread;

#[cfg(target_os = "linux")]
pub fn run(shared: Arc<AppShared>, _player_thread: thread::JoinHandle<()>) -> i32 {
    let app = gtk4::Application::builder()
        .application_id("com.saiborc.live-wallpaper")
        .build();

    let shared_clone = Arc::clone(&shared);
    app.connect_activate(move |_app| {
        build_tray(shared_clone);
    });

    app.run();

    0
}

#[cfg(target_os = "linux")]
fn build_tray(shared: Arc<AppShared>) {
    use gtk4::prelude::*;
    use ayatana_appindicator3::{AppIndicator, AppIndicatorCategory, IndicatorStatus};

    let indicator = match AppIndicator::new(
        "live-wallpaper",
        "live-wallpaper",
        AppIndicatorCategory::ApplicationStatus,
    ) {
        Ok(i) => i,
        Err(e) => {
            log::log(&format!("appindicator failed: {e}"));
            return;
        }
    };
    indicator.set_status(IndicatorStatus::Active);

    let menu = gtk4::Menu::new();

    // ---- Video title (read-only) ----
    let title_item = gtk4::MenuItem::new_with_label("No video selected");
    title_item.set_sensitive(false);
    menu.append(&title_item);

    let sep = gtk4::SeparatorMenuItem::new();
    menu.append(&sep);

    // ---- Pick video ----
    let pick_item = gtk4::MenuItem::new_with_label("Pick video...");
    let tx_pick = shared.tx.clone();
    pick_item.connect_activate(move |_| {
        if let Some(path) = pick_video() {
            {
                let mut cfg = shared.cfg.lock().unwrap();
                cfg.video = Some(path.clone());
                let _ = config::save(&cfg);
            }
            let _ = tx_pick.send(Cmd::Load { path });
        }
    });
    menu.append(&pick_item);

    let sep = gtk4::SeparatorMenuItem::new();
    menu.append(&sep);

    // ---- Play audio ----
    let audio_item = gtk4::CheckMenuItem::new_with_label("Play audio");
    {
        let cfg = shared.cfg.lock().unwrap();
        audio_item.set_active(cfg.audio);
    }
    let tx_audio = shared.tx.clone();
    audio_item.connect_toggled(move |item| {
        let on = item.is_active();
        let _ = tx_audio.send(Cmd::SetAudio { on });
    });
    menu.append(&audio_item);

    // ---- Volume +10 ----
    let vol_up = gtk4::MenuItem::new_with_label("Volume +10");
    let tx_vol_up = shared.tx.clone();
    let cfg_up = Arc::clone(&shared.cfg);
    vol_up.connect_activate(move |_| {
        let v = {
            let mut cfg = cfg_up.lock().unwrap();
            cfg.volume = (cfg.volume + 10).clamp(0, 150);
            let v = cfg.volume;
            let _ = config::save(&cfg);
            v
        };
        let _ = tx_vol_up.send(Cmd::SetVolume { v });
    });
    menu.append(&vol_up);

    // ---- Volume -10 ----
    let vol_down = gtk4::MenuItem::new_with_label("Volume -10");
    let tx_vol_down = shared.tx.clone();
    let cfg_down = Arc::clone(&shared.cfg);
    vol_down.connect_activate(move |_| {
        let v = {
            let mut cfg = cfg_down.lock().unwrap();
            cfg.volume = (cfg.volume - 10).clamp(0, 150);
            let v = cfg.volume;
            let _ = config::save(&cfg);
            v
        };
        let _ = tx_vol_down.send(Cmd::SetVolume { v });
    });
    menu.append(&vol_down);

    let sep = gtk4::SeparatorMenuItem::new();
    menu.append(&sep);

    // ---- Video fit submenu ----
    let fit_menu = gtk4::Menu::new();
    let fit_item = gtk4::MenuItem::new_with_label("Video fit");
    fit_item.set_submenu(Some(&fit_menu));

    for (mode, label) in [
        ("fill", "Fill (crop to screen)"),
        ("fit", "Fit (whole video visible)"),
        ("center", "Center (original size)"),
        ("stretch", "Stretch (fill and distort)"),
    ] {
        let mi = gtk4::MenuItem::new_with_label(label);
        let tx_fit = shared.tx.clone();
        let cfg_fit = Arc::clone(&shared.cfg);
        let mode_owned = mode.to_string();
        mi.connect_activate(move |_| {
            {
                let mut cfg = cfg_fit.lock().unwrap();
                cfg.fit = mode_owned.clone();
                let _ = config::save(&cfg);
            }
            let _ = tx_fit.send(Cmd::SetFit { mode: mode_owned.clone() });
        });
        fit_menu.append(&mi);
    }
    fit_menu.show_all();
    menu.append(&fit_item);

    let sep = gtk4::SeparatorMenuItem::new();
    menu.append(&sep);

    // ---- Lite mode ----
    let lite_item = gtk4::CheckMenuItem::new_with_label("Lite mode (low GPU/RAM)");
    {
        let cfg = shared.cfg.lock().unwrap();
        lite_item.set_active(cfg.lite);
    }
    let tx_lite = shared.tx.clone();
    lite_item.connect_toggled(move |item| {
        let on = item.is_active();
        let _ = tx_lite.send(Cmd::SetLite { on });
    });
    menu.append(&lite_item);

    // ---- V-sync ----
    let vsync_item = gtk4::CheckMenuItem::new_with_label("V-sync (smooth playback)");
    {
        let cfg = shared.cfg.lock().unwrap();
        vsync_item.set_active(cfg.vsync);
    }
    let tx_vsync = shared.tx.clone();
    vsync_item.connect_toggled(move |item| {
        let on = item.is_active();
        let _ = tx_vsync.send(Cmd::SetVsync { on });
    });
    menu.append(&vsync_item);

    let sep = gtk4::SeparatorMenuItem::new();
    menu.append(&sep);

    // ---- Quit ----
    let quit_item = gtk4::MenuItem::new_with_label("Quit");
    let tx_quit = shared.tx.clone();
    quit_item.connect_activate(move |_| {
        let _ = tx_quit.send(Cmd::Quit);
        gtk4::Application::quit();
    });
    menu.append(&quit_item);

    menu.show_all();
    indicator.set_menu(Some(&menu));
}

/// Open a GTK file chooser dialog and return the selected path.
#[cfg(target_os = "linux")]
fn pick_video() -> Option<String> {
    use gtk4::prelude::*;
    use gtk4::FileChooserAction;
    use gtk4::FileFilter;

    let dialog = gtk4::FileChooserNative::new(
        Some("Pick video"),
        None::<&gtk4::ApplicationWindow>,
        FileChooserAction::Open,
        "Open",
        "Cancel",
    );

    let filter = FileFilter::new();
    filter.set_name("Video files");
    for pat in &[
        "*.mp4", "*.mkv", "*.avi", "*.webm", "*.mov", "*.flv", "*.wmv",
        "*.MP4", "*.MKV", "*.AVI", "*.WEBM", "*.MOV", "*.FLV", "*.WMV",
    ] {
        filter.add_pattern(pat);
    }
    dialog.add_filter(&filter);

    let filter_all = FileFilter::new();
    filter_all.set_name("All files");
    filter_all.add_pattern("*");
    dialog.add_filter(&filter_all);

    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    dialog.connect_response(move |dialog, response| {
        let path = if response == gtk4::ResponseType::Accept {
            dialog.file().and_then(|f| f.path())
        } else {
            None
        };
        let _ = tx.send(path.map(|p| p.to_string_lossy().into_owned()));
        dialog.close();
    });
    dialog.show();

    rx.recv().ok().flatten()
}

#[cfg(not(target_os = "linux"))]
pub fn run(_shared: Arc<AppShared>, _player_thread: thread::JoinHandle<()>) -> i32 {
    0
}