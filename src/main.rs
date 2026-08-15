#![windows_subsystem = "windows"]

mod config;
mod log;
mod mpv;
mod ui;
mod wallpaper;

use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use windows::core::{w, PCWSTR};
use windows::Win32::UI::WindowsAndMessaging::MessageBoxW;

pub enum Cmd {
    Load { path: String },
    SetAudio { on: bool },
    SetVolume { v: i64 },
    SetFit { mode: String },
    SetRenderer { name: String },
    SetLite { on: bool },
    SetVsync { on: bool },
    SetDownscale { mode: String },
    SetHwdec { mode: String },
    Restart,
    Quit,
}

pub struct AppShared {
    pub tx: Sender<Cmd>,
    pub cfg: Arc<Mutex<config::Config>>,
}

enum SessionResult {
    Quit,
    Retry,
}

fn acquire_single_instance() -> Option<std::fs::File> {
    let temp = std::env::var("TEMP").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::Path::new(&temp).join("LiveWallpaper");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("instance.lock");
    let f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&path)
        .ok()?;
    match f.try_lock() {
        Ok(()) => Some(f),
        Err(_) => None,
    }
}

fn main() {
    let _single_instance = acquire_single_instance();
    if _single_instance.is_none() {
        let text = "Live Wallpaper is already running.";
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = unsafe {
            MessageBoxW(
                None,
                PCWSTR(wide.as_ptr()),
                w!("Live Wallpaper"),
                windows::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
            )
        };
        std::process::exit(0);
    }

    let dir = config::data_dir();
    log::init(&dir);

    wallpaper::set_dpi_awareness();
    ui::enable_dark_mode();

    let mut cfg = config::load();
    if let Some(on) = ui::autostart_enabled() {
        cfg.autostart = on;
        let _ = config::save(&cfg);
    }

    let cfg = Arc::new(Mutex::new(cfg));
    let (tx, rx) = channel::<Cmd>();

    let shared = Arc::new(AppShared {
        tx,
        cfg: Arc::clone(&cfg),
    });

    let player_cfg = Arc::clone(&cfg);
    let player_thread = thread::spawn(move || player_loop(rx, player_cfg));

    let code = ui::run(Arc::clone(&shared), player_thread);
    std::process::exit(code);
}

fn player_loop(rx: Receiver<Cmd>, cfg: Arc<Mutex<config::Config>>) {
    loop {
        match run_session(&rx, &cfg) {
            SessionResult::Quit => break,
            SessionResult::Retry => {
                log::log("wallpaper session ended, re-attaching in 1s...");
                thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
    }
    log::log("player thread exiting");
}

fn ram_mb() -> i64 {
    use windows::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    unsafe {
        let mut pmc = PROCESS_MEMORY_COUNTERS::default();
        let h = windows::Win32::System::Threading::GetCurrentProcess();
        if GetProcessMemoryInfo(h, &mut pmc, std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32)
            .is_ok()
        {
            (pmc.WorkingSetSize / (1024 * 1024)) as i64
        } else {
            0
        }
    }
}

fn on_battery() -> bool {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    unsafe {
        let mut s = SYSTEM_POWER_STATUS::default();
        if GetSystemPowerStatus(&mut s).is_ok() {
            s.BatteryFlag != 0x80 && s.ACLineStatus == 0
        } else {
            false
        }
    }
}

fn save_playback_position(players: &[mpv::Player]) {
    if let Some(p) = players.first() {
        let path = p.mpv.get_property_string("path");
        let pos = p.mpv.get_property_double("time-pos");
        let dur = p.mpv.get_property_double("duration");
        if let (Ok(path), Ok(pos), Ok(dur)) = (path, pos, dur) {
            if !path.is_empty() && pos > 3.0 && pos < dur - 10.0 {
                config::save_pos(&path, pos);
            }
        }
    }
}

fn report_error(error_state: &mut Option<String>, msg: String) {
    log::log(&format!("ERROR: {msg}"));
    if error_state.as_deref() != Some(msg.as_str()) {
        ui::notify_error(&msg);
    }
    *error_state = Some(msg);
}

fn run_session(rx: &Receiver<Cmd>, cfg: &Arc<Mutex<config::Config>>) -> SessionResult {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyWindow, DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    };

    wallpaper::SURFACE_DESTROYED.store(false, Ordering::SeqCst);

    let host = match wallpaper::find_workerw() {
        Ok(h) => h,
        Err(e) => {
            log::log(&format!("desktop hook failed: {e}"));
            return SessionResult::Retry;
        }
    };

    let (mode, click_through) = {
        let c = cfg.lock().unwrap();
        (c.monitor_mode.clone(), c.click_through)
    };
    let monitors = wallpaper::monitor_list();
    let selected: Vec<_> = if mode == "all" {
        monitors
    } else {
        monitors.into_iter().take(1).collect()
    };
    log::log(&format!("host={:?} monitors={}", host.0, selected.len()));

    let mut surfaces: Vec<HWND> = Vec::new();
    for m in &selected {
        match wallpaper::spawn_surface(host, m, click_through) {
            Ok(h) => surfaces.push(h),
            Err(e) => log::log(&format!("surface create: {e}")),
        }
    }
    if surfaces.is_empty() {
        log::log("no surfaces could be created");
        return SessionResult::Retry;
    }

    let (audio_enabled, volume, renderer, lite, vsync, downscale, hwdec, blur) = {
        let c = cfg.lock().unwrap();
        (
            c.audio,
            c.volume,
            c.renderer.clone(),
            c.lite,
            c.vsync,
            c.downscale.clone(),
            c.hwdec.clone(),
            c.dim_when_hidden,
        )
    };

    let mut chain: Vec<String> = vec![renderer.clone()];
    for cand in ["d3d11", "angle", "vulkan"] {
        if !chain.iter().any(|c| c == cand) {
            chain.push(cand.to_string());
        }
    }

    let mut effective: Option<String> = None;
    let mut players: Vec<mpv::Player> = Vec::new();
    for cand in &chain {
        match mpv::Player::create(
            surfaces[0].0 as isize,
            audio_enabled,
            volume,
            cand,
            lite,
            vsync,
            &downscale,
            &hwdec,
            blur,
        ) {
            Ok(p) => {
                log::log(&format!("renderer active: {cand}"));
                effective = Some(cand.clone());
                players.push(p);
                break;
            }
            Err(e) => log::log(&format!("renderer {cand} failed: {e}, trying next...")),
        }
    }
    if effective.is_none() {
        log::log("no renderer could initialize");
        for s in &surfaces {
            let _ = unsafe { DestroyWindow(*s) };
        }
        return SessionResult::Retry;
    }
    let effective = effective.unwrap();
    for (i, s) in surfaces.iter().enumerate().skip(1) {
        match mpv::Player::create(
            s.0 as isize,
            audio_enabled && i == 0,
            volume,
            &effective,
            lite,
            vsync,
            &downscale,
            &hwdec,
            blur,
        ) {
            Ok(p) => players.push(p),
            Err(e) => log::log(&format!("mpv init (extra surface): {e}")),
        }
    }
    if players.is_empty() {
        log::log("no mpv players created");
        for s in &surfaces {
            let _ = unsafe { DestroyWindow(*s) };
        }
        return SessionResult::Retry;
    }

    let fit_mode = cfg.lock().unwrap().fit.clone();
    for p in &players {
        match p.set_fit(&fit_mode) {
            Ok(()) => log::log(&format!("video fit set: {fit_mode}")),
            Err(e) => log::log(&format!("set fit {fit_mode}: {e}")),
        }
    }

    let mut current_path: Option<String> = None;

    let initial_video = cfg.lock().unwrap().video.clone();
    if let Some(path) = initial_video {
        if std::path::Path::new(&path).exists() {
            let start = config::load_pos(&path);
            for p in &players {
                match p.load_file(&path, start) {
                    Ok(()) => log::log(&format!(
                        "loadfile issued: {path} (resume={start:.1}s)"
                    )),
                    Err(e) => log::log(&format!("loadfile {path}: {e}")),
                }
            }
            current_path = Some(path);
        } else {
            log::log(&format!("saved video no longer exists: {path}"));
        }
    }

    let mut iter: u64 = 0;
    let mut last_pause_check = std::time::Instant::now();
    let mut pause_state = false;
    let mut blur_state = false;
    let mut last_pos_save = std::time::Instant::now();
    let mut error_state: Option<String> = None;
    let mut last_time: f64 = -1.0;
    let mut stuck_streak: u32 = 0;
    let mut last_file_loaded = false;
    loop {
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                Cmd::Load { path } => {
                    let start = config::load_pos(&path);
                    for p in &players {
                        if let Err(e) = p.load_file(&path, start) {
                            log::log(&format!("loadfile: {e}"));
                        }
                    }
                    current_path = Some(path);
                }
                Cmd::SetAudio { on } => {
                    for (i, p) in players.iter().enumerate() {
                        let _ = p.set_mute(!on || i != 0);
                    }
                }
                Cmd::SetVolume { v } => {
                    for p in &players {
                        let _ = p.set_volume(v);
                    }
                }
                Cmd::SetFit { mode } => {
                    log::log(&format!("applying video fit: {mode}"));
                    for p in &players {
                        if let Err(e) = p.set_fit(&mode) {
                            log::log(&format!("set fit {mode}: {e}"));
                        }
                    }
                }
                Cmd::SetRenderer { name } => {
                    log::log(&format!("renderer changed to {name}, restarting session"));
                    for s in &surfaces {
                        let _ = unsafe { DestroyWindow(*s) };
                    }
                    return SessionResult::Retry;
                }
                Cmd::SetLite { on } => {
                    log::log(&format!("lite mode set to {on}, restarting session"));
                    for s in &surfaces {
                        let _ = unsafe { DestroyWindow(*s) };
                    }
                    return SessionResult::Retry;
                }
                Cmd::SetVsync { on } => {
                    log::log(&format!("vsync set to {on}, restarting session"));
                    for s in &surfaces {
                        let _ = unsafe { DestroyWindow(*s) };
                    }
                    return SessionResult::Retry;
                }
                Cmd::SetDownscale { mode } => {
                    log::log(&format!("decode downscale set to {mode}, restarting session"));
                    for s in &surfaces {
                        let _ = unsafe { DestroyWindow(*s) };
                    }
                    return SessionResult::Retry;
                }
                Cmd::SetHwdec { mode } => {
                    log::log(&format!("hwdec set to {mode}, restarting session"));
                    for s in &surfaces {
                        let _ = unsafe { DestroyWindow(*s) };
                    }
                    return SessionResult::Retry;
                }
                Cmd::Restart => {
                    log::log("restarting session (monitor mode changed)");
                    for s in &surfaces {
                        let _ = unsafe { DestroyWindow(*s) };
                    }
                    return SessionResult::Retry;
                }
                Cmd::Quit => {
                    save_playback_position(&players);
                    for s in &surfaces {
                        let _ = unsafe { DestroyWindow(*s) };
                    }
                    return SessionResult::Quit;
                }
            }
        }

        if let Err(e) = rx.try_recv() {
            if matches!(e, std::sync::mpsc::TryRecvError::Disconnected) {
                log::log("control channel closed (UI gone) - shutting down");
                save_playback_position(&players);
                for s in &surfaces {
                    let _ = unsafe { DestroyWindow(*s) };
                }
                return SessionResult::Quit;
            }
        }

        if wallpaper::SURFACE_DESTROYED.load(Ordering::SeqCst) {
            log::log("desktop surface destroyed (explorer restart?)");
            for s in &surfaces {
                let _ = unsafe { DestroyWindow(*s) };
            }
            return SessionResult::Retry;
        }

        if last_pause_check.elapsed() >= std::time::Duration::from_millis(1000) {
            last_pause_check = std::time::Instant::now();
            let (smart, blur, battery) = {
                let c = cfg.lock().unwrap();
                (c.smart_pause, c.dim_when_hidden, c.pause_on_battery)
            };
            let smart_covered = smart && wallpaper::desktop_covered();
            let battery_paused = battery && on_battery();
            let covered = smart_covered || battery_paused;
            if covered != pause_state {
                pause_state = covered;
                log::log(&format!(
                    "desktop {} (battery_pause={battery_paused}) - video {}",
                    if smart_covered { "hidden" } else { "visible" },
                    if covered { "paused" } else { "resumed" }
                ));
                for p in &players {
                    let _ = p.set_pause(covered);
                }
            }

            if smart_covered != blur_state {
                blur_state = smart_covered;
                for p in &players {
                    let _ = p.set_blur(smart_covered && blur);
                }
            }
        }

        if last_pos_save.elapsed() >= std::time::Duration::from_secs(15) {
            last_pos_save = std::time::Instant::now();
            save_playback_position(&players);
        }

        iter += 1;
        if iter % 60 == 0 {
            if let Some(p) = players.first() {
                let s = p.mpv_status();
                log::log(&format!("status: {s}"));
                let paused = p.mpv.get_property_flag("pause").unwrap_or(false);
                let idle = p.mpv.get_property_flag("idle-active").unwrap_or(false);
                let t = p.mpv.get_property_double("time-pos").unwrap_or(-1.0);

                if last_file_loaded && !paused && !idle && t >= 0.0 {
                    if last_time >= 0.0 && (t - last_time).abs() < 0.01 {
                        stuck_streak += 1;
                        if stuck_streak >= 4 {
                            let path = p
                                .mpv
                                .get_property_string("path")
                                .unwrap_or_else(|_| "?".to_string());
                            report_error(
                                &mut error_state,
                                format!("playback appears stuck (time not advancing): {path}"),
                            );
                            stuck_streak = 0;
                        }
                    } else {
                        stuck_streak = 0;
                    }
                    last_time = t;
                } else {
                    stuck_streak = 0;
                    last_time = -1.0;
                }
                let state = if idle {
                    "idle"
                } else if paused {
                    "paused"
                } else {
                    "playing"
                };
                let fps = p
                    .mpv
                    .get_property_double("estimated-vf-fps")
                    .unwrap_or(0.0);
                let tip = match &error_state {
                    Some(e) => format!("Live Wallpaper | ERROR: {e}"),
                    None => {
                        format!("Live Wallpaper | {state} | {fps:.0} fps | {} MB", ram_mb())
                    }
                };
                ui::update_tip(&tip);
            }
        }

        for p in &players {
            let ev = p.wait_event(0.05);
            match ev.event_id {
                mpv::MPV_EVENT_SHUTDOWN => {
                    log::log("mpv shutdown event");
                    for s in &surfaces {
                        let _ = unsafe { DestroyWindow(*s) };
                    }
                    return SessionResult::Retry;
                }
                mpv::MPV_EVENT_FILE_LOADED => {
                    log::log("mpv: file loaded");
                    last_file_loaded = true;
                    stuck_streak = 0;
                    last_time = -1.0;
                    let vfmt = p.mpv.get_property_string("video-format").unwrap_or_default();
                    if vfmt.is_empty() {
                        let path = p
                            .mpv
                            .get_property_string("path")
                            .ok()
                            .filter(|s| !s.is_empty())
                            .or_else(|| current_path.clone())
                            .unwrap_or_else(|| "?".to_string());
                        report_error(
                            &mut error_state,
                            format!("no video stream found in: {path} (audio-only file?)"),
                        );
                    } else {
                        error_state = None;
                    }
                }
                mpv::MPV_EVENT_END_FILE => {
                    let reason = mpv::end_file_reason(&ev);
                    log::log(&format!(
                        "mpv: end file (reason={reason}){}",
                        if reason == mpv::MPV_END_FILE_REASON_ERROR {
                            " ERROR"
                        } else {
                            ""
                        }
                    ));
                    if reason == mpv::MPV_END_FILE_REASON_ERROR {
                        last_file_loaded = false;
                        let path = p
                            .mpv
                            .get_property_string("path")
                            .ok()
                            .filter(|s| !s.is_empty())
                            .or_else(|| current_path.clone())
                            .unwrap_or_else(|| "?".to_string());
                        report_error(
                            &mut error_state,
                            format!("video failed to load (unsupported or corrupt): {path}"),
                        );
                    }
                }
                mpv::MPV_EVENT_LOG_MESSAGE => {
                    if let Some(m) = mpv::decode_log(&ev) {
                        log::log(&format!("mpv[{}] {}", m.level, m.text.trim_end()));
                    }
                }
                _ => {}
            }
        }

        thread::sleep(std::time::Duration::from_millis(25));
    }
}
