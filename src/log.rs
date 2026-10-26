use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

#[cfg(target_os = "windows")]
pub fn init(_appdata_dir: &PathBuf) {
    let temp = std::env::var("TEMP").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(temp).join("LiveWallpaper");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("log init failed: {e}");
        return;
    }
    *LOG_PATH.lock().unwrap() = Some(dir.join("log.txt"));
}

#[cfg(target_os = "linux")]
pub fn init(_appdata_dir: &PathBuf) {
    let cache_dir = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache").join("live-wallpaper")
    } else {
        PathBuf::from(".").join("live-wallpaper")
    };
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        eprintln!("log init failed: {e}");
        return;
    }
    *LOG_PATH.lock().unwrap() = Some(cache_dir.join("log.txt"));
}

#[cfg(target_os = "macos")]
pub fn init(_appdata_dir: &PathBuf) {
    let cache_dir = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join("Library").join("Caches").join("live-wallpaper")
    } else {
        PathBuf::from(".").join("live-wallpaper")
    };
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        eprintln!("log init failed: {e}");
        return;
    }
    *LOG_PATH.lock().unwrap() = Some(cache_dir.join("log.txt"));
}

pub fn log(msg: &str) {
    eprintln!("[lw] {msg}");
    let path = LOG_PATH.lock().unwrap().clone();
    let Some(path) = path else { return };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let line = format!("[{now}] {msg}\n");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}