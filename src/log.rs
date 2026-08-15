use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn init(_appdata_dir: &PathBuf) {
    let temp = std::env::var("TEMP").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(temp).join("LiveWallpaper");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("log init failed: {e}");
        return;
    }
    *LOG_PATH.lock().unwrap() = Some(dir.join("log.txt"));
}

pub fn log(msg: &str) {
    eprintln!("[lw] {msg}");
    let path = LOG_PATH.lock().unwrap().clone();
    let Some(path) = path else { return };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let line = format!("[{now}] {msg}\r\n");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}
