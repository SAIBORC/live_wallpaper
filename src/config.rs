use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub video: Option<String>,
    #[serde(default = "d_audio")]
    pub audio: bool,
    #[serde(default = "d_volume")]
    pub volume: i64,
    #[serde(default = "d_monitor")]
    pub monitor_mode: String,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default = "d_click")]
    pub click_through: bool,
    #[serde(default = "d_fit")]
    pub fit: String,
    #[serde(default = "d_renderer")]
    pub renderer: String,
    #[serde(default)]
    pub lite: bool,
    #[serde(default = "d_smart")]
    pub smart_pause: bool,
    #[serde(default = "d_dim")]
    pub dim_when_hidden: bool,
    #[serde(default = "d_battery")]
    pub pause_on_battery: bool,
    #[serde(default = "d_downscale")]
    pub downscale: String,
    #[serde(default = "d_vsync")]
    pub vsync: bool,
    #[serde(default = "d_hwdec")]
    pub hwdec: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            video: None,
            audio: true,
            volume: 100,
            monitor_mode: "primary".to_string(),
            autostart: false,
            click_through: true,
            fit: "fill".to_string(),
            renderer: "d3d11".to_string(),
            lite: false,
            smart_pause: true,
            dim_when_hidden: true,
            pause_on_battery: true,
            downscale: "off".to_string(),
            vsync: true,
            hwdec: "auto".to_string(),
        }
    }
}

fn d_audio() -> bool {
    true
}
fn d_volume() -> i64 {
    100
}
fn d_monitor() -> String {
    "primary".to_string()
}
fn d_click() -> bool {
    true
}
fn d_fit() -> String {
    "fill".to_string()
}
fn d_renderer() -> String {
    "d3d11".to_string()
}
fn d_smart() -> bool {
    true
}
fn d_dim() -> bool {
    true
}
fn d_battery() -> bool {
    true
}
fn d_downscale() -> String {
    "off".to_string()
}
fn d_vsync() -> bool {
    true
}
fn d_hwdec() -> String {
    "auto".to_string()
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("LiveWallpaper")
}

pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

pub fn pos_path() -> PathBuf {
    data_dir().join("position.json")
}

pub fn load_pos(path: &str) -> f64 {
    let p = pos_path();
    if let Ok(text) = fs::read_to_string(&p) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if v.get("path").and_then(|x| x.as_str()) == Some(path) {
                if let Some(pos) = v.get("pos").and_then(|x| x.as_f64()) {
                    return pos;
                }
            }
        }
    }
    0.0
}

pub fn save_pos(path: &str, pos: f64) {
    let v = serde_json::json!({ "path": path, "pos": pos });
    if let Ok(text) = serde_json::to_string(&v) {
        let _ = fs::write(pos_path(), text);
    }
}

pub fn load() -> Config {
    let p = config_path();
    match fs::read_to_string(&p) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| {
            let c = Config::default();
            let _ = save(&c);
            c
        }),
        Err(_) => Config::default(),
    }
}

pub fn save(cfg: &Config) -> Result<(), String> {
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    let text = serde_json::to_string_pretty(cfg).map_err(|e| format!("serialize: {e}"))?;
    fs::write(config_path(), text).map_err(|e| format!("write config: {e}"))
}
