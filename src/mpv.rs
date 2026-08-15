#![allow(dead_code)]

use crate::log;
use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

pub const MPV_FORMAT_STRING: c_int = 1;
pub const MPV_FORMAT_FLAG: c_int = 3;
pub const MPV_FORMAT_INT64: c_int = 4;
pub const MPV_FORMAT_DOUBLE: c_int = 5;

pub const MPV_EVENT_SHUTDOWN: c_int = 1;
pub const MPV_EVENT_LOG_MESSAGE: c_int = 2;
pub const MPV_EVENT_END_FILE: c_int = 7;
pub const MPV_EVENT_FILE_LOADED: c_int = 8;
pub const MPV_EVENT_IDLE: c_int = 11;

pub const MPV_END_FILE_REASON_EOF: c_int = 0;
pub const MPV_END_FILE_REASON_STOP: c_int = 2;
pub const MPV_END_FILE_REASON_QUIT: c_int = 3;
pub const MPV_END_FILE_REASON_ERROR: c_int = 4;

type MpvHandle = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MpvEvent {
    pub event_id: c_int,
    pub error: c_int,
    pub reply_userdata: u64,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct MpvEventLogMessage {
    pub prefix: *const c_char,
    pub level: *const c_char,
    pub text: *const c_char,
    pub log_level: c_int,
}

pub struct LogMsg {
    pub level: String,
    pub text: String,
}

pub fn decode_log(ev: &MpvEvent) -> Option<LogMsg> {
    if ev.event_id != MPV_EVENT_LOG_MESSAGE || ev.data.is_null() {
        return None;
    }
    let lm = unsafe { &*(ev.data as *const MpvEventLogMessage) };
    let s = |p: *const c_char| {
        if p.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() }
        }
    };
    Some(LogMsg {
        level: s(lm.level),
        text: s(lm.text),
    })
}

pub fn end_file_reason(ev: &MpvEvent) -> c_int {
    if ev.event_id == MPV_EVENT_END_FILE && !ev.data.is_null() {
        unsafe { *(ev.data as *const c_int) }
    } else {
        -1
    }
}

pub struct Mpv {
    lib: Library,
    handle: MpvHandle,
}

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap_or_default()
}

fn find_dll() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_default();

    let mut dirs = vec![exe_dir.clone()];
    if let Ok(cwd) = std::env::current_dir() {
        if !dirs.contains(&cwd) {
            dirs.push(cwd);
        }
    }
    for dir in &dirs {
        for name in ["libmpv-2.dll", "mpv-2.dll", "mpv-1.dll"] {
            let p = dir.join(name);
            if p.exists() {
                return Some(p);
            }
        }
    }
    let _ = exe_dir;
    None
}

unsafe fn load<'a, T>(lib: &'a Library, name: &str) -> Result<Symbol<'a, T>, String> {
    lib.get(name.as_bytes())
        .map_err(|e| format!("libmpv missing export `{name}`: {e}"))
}

impl Mpv {
    pub fn new() -> Result<Self, String> {
        let dll = find_dll().ok_or_else(|| {
            "libmpv dll not found. Keep live-wallpaper.exe with libmpv-2.dll (and \
             VCRUNTIME140.dll) in the same folder, or extract the whole zip first."
                .to_string()
        })?;
        let lib = unsafe {
            Library::new(&dll).map_err(|e| format!("cannot load mpv dll ({}): {e}", dll.display()))?
        };
        let handle = unsafe {
            let f: Symbol<unsafe extern "C" fn() -> MpvHandle> = load(&lib, "mpv_create")?;
            f()
        };
        Ok(Mpv { lib, handle })
    }

    fn err_str(&self, code: c_int) -> String {
        unsafe {
            match load::<unsafe extern "C" fn(c_int) -> *const c_char>(&self.lib, "mpv_error_string")
            {
                Ok(f) => {
                    let p = f(code);
                    if p.is_null() {
                        format!("mpv error {code}")
                    } else {
                        CStr::from_ptr(p).to_string_lossy().into_owned()
                    }
                }
                Err(e) => format!("mpv error {code} ({e})"),
            }
        }
    }

    pub fn initialize(&self) -> Result<(), String> {
        let rc = unsafe {
            let f = load::<unsafe extern "C" fn(MpvHandle) -> c_int>(&self.lib, "mpv_initialize")?;
            f(self.handle)
        };
        if rc < 0 {
            Err(self.err_str(rc))
        } else {
            Ok(())
        }
    }

    pub fn set_option_string(&self, name: &str, value: &str) -> Result<(), String> {
        let n = cstr(name);
        let v = cstr(value);
        let rc = unsafe {
            let f = load::<unsafe extern "C" fn(MpvHandle, *const c_char, *const c_char) -> c_int>(
                &self.lib,
                "mpv_set_option_string",
            )?;
            f(self.handle, n.as_ptr(), v.as_ptr())
        };
        if rc < 0 {
            Err(self.err_str(rc))
        } else {
            Ok(())
        }
    }

    pub fn set_option_int(&self, name: &str, value: i64) -> Result<(), String> {
        let n = cstr(name);
        let rc = unsafe {
            let f = load::<unsafe extern "C" fn(MpvHandle, *const c_char, c_int, *const c_void) -> c_int>(
                &self.lib,
                "mpv_set_option",
            )?;
            f(
                self.handle,
                n.as_ptr(),
                MPV_FORMAT_INT64,
                &value as *const i64 as *const c_void,
            )
        };
        if rc < 0 {
            Err(self.err_str(rc))
        } else {
            Ok(())
        }
    }

    pub fn set_property_flag(&self, name: &str, value: bool) -> Result<(), String> {
        let n = cstr(name);
        let v: c_int = if value { 1 } else { 0 };
        let rc = unsafe {
            let f = load::<unsafe extern "C" fn(MpvHandle, *const c_char, c_int, *const c_void) -> c_int>(
                &self.lib,
                "mpv_set_property",
            )?;
            f(
                self.handle,
                n.as_ptr(),
                MPV_FORMAT_FLAG,
                &v as *const c_int as *const c_void,
            )
        };
        if rc < 0 {
            Err(self.err_str(rc))
        } else {
            Ok(())
        }
    }

    pub fn set_property_int(&self, name: &str, value: i64) -> Result<(), String> {
        let n = cstr(name);
        let rc = unsafe {
            let f = load::<unsafe extern "C" fn(MpvHandle, *const c_char, c_int, *const c_void) -> c_int>(
                &self.lib,
                "mpv_set_property",
            )?;
            f(
                self.handle,
                n.as_ptr(),
                MPV_FORMAT_INT64,
                &value as *const i64 as *const c_void,
            )
        };
        if rc < 0 {
            Err(self.err_str(rc))
        } else {
            Ok(())
        }
    }

    pub fn set_property_string(&self, name: &str, value: &str) -> Result<(), String> {
        let n = cstr(name);
        let v = cstr(value);
        let rc = unsafe {
            let f = load::<unsafe extern "C" fn(MpvHandle, *const c_char, *const c_char) -> c_int>(
                &self.lib,
                "mpv_set_property_string",
            )?;
            f(self.handle, n.as_ptr(), v.as_ptr())
        };
        if rc < 0 {
            Err(self.err_str(rc))
        } else {
            Ok(())
        }
    }

    pub fn get_property_flag(&self, name: &str) -> Result<bool, ()> {
        let n = cstr(name);
        let mut v: c_int = 0;
        let rc = unsafe {
            let f = load::<unsafe extern "C" fn(MpvHandle, *const c_char, c_int, *mut c_void) -> c_int>(
                &self.lib,
                "mpv_get_property",
            )
            .map_err(|_| ())?;
            f(
                self.handle,
                n.as_ptr(),
                MPV_FORMAT_FLAG,
                &mut v as *mut c_int as *mut c_void,
            )
        };
        if rc >= 0 {
            Ok(v != 0)
        } else {
            Err(())
        }
    }

    pub fn get_property_double(&self, name: &str) -> Result<f64, ()> {
        let n = cstr(name);
        let mut v: f64 = 0.0;
        let rc = unsafe {
            let f = load::<unsafe extern "C" fn(MpvHandle, *const c_char, c_int, *mut c_void) -> c_int>(
                &self.lib,
                "mpv_get_property",
            )
            .map_err(|_| ())?;
            f(
                self.handle,
                n.as_ptr(),
                MPV_FORMAT_DOUBLE,
                &mut v as *mut f64 as *mut c_void,
            )
        };
        if rc >= 0 {
            Ok(v)
        } else {
            Err(())
        }
    }

    pub fn get_property_string(&self, name: &str) -> Result<String, ()> {        let n = cstr(name);
        let mut ptr: *mut c_char = std::ptr::null_mut();
        let rc = unsafe {
            let f = load::<unsafe extern "C" fn(MpvHandle, *const c_char, c_int, *mut c_void) -> c_int>(
                &self.lib,
                "mpv_get_property",
            )
            .map_err(|_| ())?;
            f(
                self.handle,
                n.as_ptr(),
                MPV_FORMAT_STRING,
                &mut ptr as *mut *mut c_char as *mut c_void,
            )
        };
        if rc < 0 || ptr.is_null() {
            return Err(());
        }
        let s = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
        unsafe {
            if let Ok(f) = load::<unsafe extern "C" fn(*mut c_void)>(&self.lib, "mpv_free") {
                f(ptr as *mut c_void);
            }
        }
        Ok(s)
    }

    pub fn command(&self, args: &[&str]) -> Result<(), String> {
        let owned: Vec<CString> = args.iter().map(|a| cstr(a)).collect();
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        let rc = unsafe {
            let f = load::<unsafe extern "C" fn(MpvHandle, *const *const c_char) -> c_int>(
                &self.lib,
                "mpv_command",
            )?;
            f(self.handle, ptrs.as_ptr())
        };
        if rc < 0 {
            Err(self.err_str(rc))
        } else {
            Ok(())
        }
    }

    pub fn request_log_messages(&self, level: &str) -> Result<(), String> {
        let l = cstr(level);
        let rc = unsafe {
            let f = load::<unsafe extern "C" fn(MpvHandle, *const c_char) -> c_int>(
                &self.lib,
                "mpv_request_log_messages",
            )?;
            f(self.handle, l.as_ptr())
        };
        if rc < 0 {
            Err(self.err_str(rc))
        } else {
            Ok(())
        }
    }

    pub fn wait_event(&self, timeout: f64) -> MpvEvent {
        unsafe {
            match load::<unsafe extern "C" fn(MpvHandle, f64) -> *mut MpvEvent>(
                &self.lib,
                "mpv_wait_event",
            ) {
                Ok(f) => {
                    let ev = f(self.handle, timeout);
                    if ev.is_null() {
                        MpvEvent {
                            event_id: 0,
                            error: 0,
                            reply_userdata: 0,
                            data: std::ptr::null_mut(),
                        }
                    } else {
                        *ev
                    }
                }
                Err(_) => MpvEvent {
                    event_id: 0,
                    error: 0,
                    reply_userdata: 0,
                    data: std::ptr::null_mut(),
                },
            }
        }
    }

    pub fn shutdown(&self) {
        unsafe {
            if let Ok(f) = load::<unsafe extern "C" fn(MpvHandle)>(&self.lib, "mpv_terminate_destroy")
            {
                f(self.handle);
            }
        }
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub struct Player {
    pub mpv: Mpv,
    pub surface: isize,
    downscale: String,
    blur_on: std::cell::Cell<bool>,
}

impl Player {
    pub fn create(
        surface: isize,
        audio: bool,
        volume: i64,
        renderer: &str,
        lite: bool,
        vsync: bool,
        downscale: &str,
        hwdec_cfg: &str,
        blur: bool,
    ) -> Result<Player, String> {
        let mpv = Mpv::new()?;
        mpv.set_option_int("wid", surface as i64)?;

        let (vo, gpu_api, gpu_context, def_hwdec) = match renderer {
            "vulkan" => ("gpu", Some("vulkan"), None, "d3d11va-copy"),
            "angle" => ("gpu", Some("opengl"), Some("angle"), "d3d11va-copy"),
            _ => ("gpu", Some("d3d11"), None, "auto"),
        };
        let hwdec = if hwdec_cfg == "auto" {
            def_hwdec
        } else {
            hwdec_cfg
        };

        let hwdec = if blur {
            match hwdec {
                "auto" => "d3d11va-copy",
                "d3d11va" => "d3d11va-copy",
                "dxva2" => "dxva2-copy",
                h => h,
            }
        } else {
            hwdec
        };

        let mut options: Vec<(&str, &str)> = vec![
            ("idle", "yes"),
            ("vo", vo),
            ("hwdec", hwdec),
            ("loop-file", "inf"),
            ("loop", "inf"),
            ("input-default-bindings", "no"),
            ("input-vo-keyboard", "no"),
            ("osc", "no"),
            ("osd-level", "0"),
            ("ytdl", "no"),
            ("cache", "no"),
            ("demuxer-readahead-secs", "1.0"),
            ("vd-lavc-dr", "yes"),
            ("framedrop", "vo"),
            ("keep-open", "yes"),
            ("audio-client-name", "LiveWallpaper"),
            ("msg-level", "all=error"),
            ("terminal", "no"),
        ];
        if let Some(api) = gpu_api {
            options.push(("gpu-api", api));
        }
        if let Some(ctx) = gpu_context {
            options.push(("gpu-context", ctx));
        }
        let sync = if lite {
            "display"
        } else if vsync {
            "display-resample"
        } else {
            "audio"
        };
        options.push(("video-sync", sync));
        if lite {
            options.push(("override-display-fps", "30"));
            options.push(("framedrop", "decoder"));
            options.push(("vd-lavc-skiploopfilter", "nonkey"));
        }
        match downscale {
            "720p" => options.push(("vf", "scale=w=1280:h=-2")),
            "540p" => options.push(("vf", "scale=w=960:h=-2")),
            _ => {}
        }
        log::log(&format!(
            "mpv init: vo={vo} gpu-api={gpu_api:?} gpu-context={gpu_context:?} hwdec={hwdec} lite={lite} vsync={sync} downscale={downscale}"
        ));
        for (k, v) in options {
            let _ = mpv.set_option_string(k, v);
        }
        let _ = mpv.set_option_int("volume", volume);

        mpv.initialize()?;
        let _ = mpv.set_property_flag("mute", !audio);
        let _ = mpv.request_log_messages("info");

        Ok(Player {
            mpv,
            surface,
            downscale: downscale.to_string(),
            blur_on: std::cell::Cell::new(false),
        })
    }

    pub fn load_file(&self, path: &str, start: f64) -> Result<(), String> {
        if start > 2.0 {
            let _ = self.mpv.set_option_string("start", &format!("+{start:.2}"));
        } else {
            let _ = self.mpv.set_option_string("start", "0");
        }
        self.mpv.command(&["loadfile", path, "replace"])
    }

    fn visible_chain(&self) -> &'static str {
        match self.downscale.as_str() {
            "720p" => "scale=w=1280:h=-2",
            "540p" => "scale=w=960:h=-2",
            _ => "scale=w=iw:h=ih",
        }
    }

    fn set_vf(&self, chain: &str) -> Result<(), String> {
        self.mpv.command(&["vf", "set", chain])
    }

    fn blur_chain(&self, mid: u32) -> String {
        match self.downscale.as_str() {
            "720p" => format!("scale=w=1280:h=-2,scale=w={mid}:h=-2,scale=w=1280:h=-2"),
            "540p" => format!("scale=w=960:h=-2,scale=w={mid}:h=-2,scale=w=960:h=-2"),
            _ => format!("scale=w={mid}:h=-2,scale=w=1920:h=-2"),
        }
    }

    pub fn set_blur(&self, on: bool) -> Result<(), String> {
        if on == self.blur_on.get() {
            return Ok(());
        }
        self.blur_on.set(on);
        let widths: [u32; 4] = if on {
            [320, 176, 96, 56]
        } else {
            [56, 96, 176, 320]
        };
        for w in widths {
            let _ = self.set_vf(&self.blur_chain(w));
            std::thread::sleep(std::time::Duration::from_millis(45));
        }
        if !on {
            let _ = self.set_vf(self.visible_chain());
        }
        Ok(())
    }

    pub fn set_mute(&self, mute: bool) -> Result<(), String> {
        self.mpv.set_property_flag("mute", mute)
    }

    pub fn set_pause(&self, on: bool) -> Result<(), String> {
        self.mpv.set_property_flag("pause", on)
    }

    pub fn set_volume(&self, v: i64) -> Result<(), String> {
        self.mpv.set_property_int("volume", v)
    }

    pub fn set_fit(&self, mode: &str) -> Result<(), String> {
        match mode {
            "fit" => {
                self.mpv.set_property_flag("keepaspect", true)?;
                self.mpv.set_property_string("video-unscaled", "no")?;
                self.mpv.set_property_string("panscan", "0")?;
                self.mpv.set_property_string("video-align-x", "0")?;
                self.mpv.set_property_string("video-align-y", "0")?;
            }
            "center" => {
                self.mpv.set_property_flag("keepaspect", true)?;
                self.mpv.set_property_string("video-unscaled", "yes")?;
                self.mpv.set_property_string("video-align-x", "0")?;
                self.mpv.set_property_string("video-align-y", "0")?;
            }
            "stretch" => {
                self.mpv.set_property_flag("keepaspect", false)?;
                self.mpv.set_property_string("video-unscaled", "no")?;
                self.mpv.set_property_string("panscan", "0")?;
            }
            _ => {
                self.mpv.set_property_flag("keepaspect", true)?;
                self.mpv.set_property_string("video-unscaled", "no")?;
                self.mpv.set_property_string("panscan", "1.0")?;
                self.mpv.set_property_string("video-align-x", "0")?;
                self.mpv.set_property_string("video-align-y", "0")?;
            }
        }
        Ok(())
    }

    pub fn wait_event(&self, timeout: f64) -> MpvEvent {
        self.mpv.wait_event(timeout)
    }

    pub fn mpv_status(&self) -> String {
        let f = |n: &str| self.mpv.get_property_string(n).unwrap_or_default();
        let flag = |n: &str| {
            self.mpv
                .get_property_flag(n)
                .map(|b| if b { "on" } else { "off" })
                .unwrap_or("?")
                .to_string()
        };
        let double = |n: &str| {
            self.mpv
                .get_property_double(n)
                .map(|v| format!("{v:.2}"))
                .unwrap_or_default()
        };
        format!(
            "idle={} pause={} mute={} vfmt={} afmt={} acodec={} ao={} adev={} vol={} hwdec={} t={} path={} | vo={} gpu-api={} disp-fps={} out-fps={} | vf={} fit: unscaled={} keepaspect={} panscan={}",
            flag("idle-active"),
            flag("pause"),
            flag("mute"),
            f("video-format"),
            f("audio-format"),
            f("audio-codec"),
            f("ao"),
            f("audio-device"),
            f("volume"),
            f("hwdec-current"),
            double("time-pos"),
            f("path"),
            f("vo"),
            f("gpu-api"),
            f("display-fps"),
            f("estimated-vf-fps"),
            f("vf"),
            f("video-unscaled"),
            flag("keepaspect"),
            double("panscan"),
        )
    }
}
