// ClaudeWakeup — a tiny Windows system-tray app that periodically pings the
// Claude CLI (`claude -p "hi"`) to keep your token / usage window warm.
//
// Pure Win32 (the `windows` crate) so the binary stays small. No window is
// shown; everything lives in the notification-area (tray) icon + its menu.
#![windows_subsystem = "windows"]

use core::mem::size_of;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

mod i18n;
use i18n::Lang;

// ---- constants -------------------------------------------------------------

const WM_APP_TRAY: u32 = WM_APP + 1;
const WM_APP_REFRESH: u32 = WM_APP + 2;
const TRAY_UID: u32 = 0x4357; // 'C''W'
const TIMER_ID: usize = 1;
const TIMER_MS: u32 = 15_000; // scheduler granularity
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const ID_STATUS: usize = 10;
const ID_LASTRUN: usize = 11;
const ID_TOGGLE: usize = 12;
const ID_RUNNOW: usize = 13;
const ID_CONFIG: usize = 14;
const ID_LOG: usize = 15;
const ID_RELOAD: usize = 16;
const ID_QUIT: usize = 17;

// ---- state -----------------------------------------------------------------

#[derive(Clone, PartialEq)]
enum Mode {
    Interval,
    Daily,
}

struct ConfigData {
    language: Lang,
    enabled: bool,
    mode: Mode,
    interval_minutes: u64,
    daily_times: Vec<(u8, u8)>,
    claude_path: String,
    model: String,
    prompt: String,
    extra_args: Vec<String>,
}

struct AppState {
    cfg: ConfigData,
    next_due: Option<Instant>, // interval mode
    last_fired_key: u64,       // daily mode: YYYYMMDDHHMM already fired
    last_run: String,
    config_path: PathBuf,
    log_path: PathBuf,
}

static STATE: OnceLock<Mutex<AppState>> = OnceLock::new();
static HWND_RAW: AtomicIsize = AtomicIsize::new(0);
static RUNNING: AtomicBool = AtomicBool::new(false);

fn state() -> &'static Mutex<AppState> {
    STATE.get().expect("state initialized in main")
}

fn default_config_data() -> ConfigData {
    ConfigData {
        language: Lang::En,
        enabled: true,
        mode: Mode::Interval,
        interval_minutes: 300,
        daily_times: vec![(9, 0), (14, 0), (19, 0)],
        claude_path: "claude".to_string(),
        model: "haiku".to_string(),
        prompt: "hi".to_string(),
        extra_args: Vec::new(),
    }
}

// ---- config (de)serialization ---------------------------------------------

fn parse_times(v: &str) -> Vec<(u8, u8)> {
    v.split(',')
        .filter_map(|t| {
            let t = t.trim();
            let (h, m) = t.split_once(':')?;
            Some((h.trim().parse().ok()?, m.trim().parse().ok()?))
        })
        .collect()
}

fn parse_config(text: &str) -> ConfigData {
    let mut cd = default_config_data();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim().to_lowercase();
            let v = v.trim().to_string();
            match k.as_str() {
                "language" => cd.language = Lang::parse(&v),
                "enabled" => cd.enabled = v.eq_ignore_ascii_case("true") || v == "1",
                "mode" => {
                    cd.mode = if v.eq_ignore_ascii_case("daily") {
                        Mode::Daily
                    } else {
                        Mode::Interval
                    }
                }
                "interval_minutes" => {
                    if let Ok(n) = v.parse::<u64>() {
                        cd.interval_minutes = n.max(1);
                    }
                }
                "daily_times" => cd.daily_times = parse_times(&v),
                "claude_path" => {
                    if !v.is_empty() {
                        cd.claude_path = v;
                    }
                }
                "model" => cd.model = v,
                "prompt" => {
                    if !v.is_empty() {
                        cd.prompt = v;
                    }
                }
                "extra_args" => {
                    cd.extra_args = v.split_whitespace().map(|s| s.to_string()).collect()
                }
                _ => {}
            }
        }
    }
    cd
}

fn config_text(cd: &ConfigData) -> String {
    let times: Vec<String> = cd
        .daily_times
        .iter()
        .map(|(h, m)| format!("{:02}:{:02}", h, m))
        .collect();
    let mode = if cd.mode == Mode::Daily { "daily" } else { "interval" };
    let c = cd.language.cfg_comments();
    format!(
        "# {header}\n\
         \n\
         # {c_language}\n\
         language = {language}\n\
         \n\
         # {c_enabled}\n\
         enabled = {enabled}\n\
         \n\
         # {c_mode}\n\
         mode = {mode}\n\
         \n\
         # {c_interval}\n\
         interval_minutes = {interval}\n\
         \n\
         # {c_daily}\n\
         daily_times = {times}\n\
         \n\
         # {c_command}\n\
         claude_path = {claude}\n\
         model = {model}\n\
         prompt = {prompt}\n\
         # {c_extra}\n\
         extra_args = {extra}\n",
        header = c.header,
        c_language = c.language,
        language = cd.language.code(),
        c_enabled = c.enabled,
        enabled = cd.enabled,
        c_mode = c.mode,
        mode = mode,
        c_interval = c.interval,
        interval = cd.interval_minutes,
        c_daily = c.daily,
        times = times.join(", "),
        c_command = c.command,
        claude = cd.claude_path,
        model = cd.model,
        prompt = cd.prompt,
        c_extra = c.extra,
        extra = cd.extra_args.join(" "),
    )
}

fn write_config_file(path: &PathBuf, cd: &ConfigData) {
    let _ = std::fs::write(path, config_text(cd));
}

// ---- time helpers ----------------------------------------------------------

fn local_time() -> SYSTEMTIME {
    unsafe { GetLocalTime() }
}

fn local_time_string() -> String {
    let st = local_time();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
    )
}

fn ymdhm_key(st: &SYSTEMTIME) -> u64 {
    (st.wYear as u64) * 1_0000_0000
        + (st.wMonth as u64) * 100_0000
        + (st.wDay as u64) * 1_0000
        + (st.wHour as u64) * 100
        + st.wMinute as u64
}

// ---- the wakeup ping --------------------------------------------------------

fn append_log(path: &PathBuf, line: &str) {
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{}", line);
    }
}

/// Recompute interval `next_due` from now (called on start / reload / toggle).
fn reset_interval(s: &mut AppState) {
    if s.cfg.mode == Mode::Interval {
        s.next_due = Some(Instant::now() + Duration::from_secs(s.cfg.interval_minutes * 60));
    } else {
        s.next_due = None;
    }
}

fn trigger_run() {
    // Don't overlap pings.
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let (path, prompt, model, extra, logp, lang) = {
        let s = state().lock().unwrap();
        (
            s.cfg.claude_path.clone(),
            s.cfg.prompt.clone(),
            s.cfg.model.clone(),
            s.cfg.extra_args.clone(),
            s.log_path.clone(),
            s.cfg.language,
        )
    };

    std::thread::spawn(move || {
        let started = local_time_string();
        let mut cmd = Command::new(&path);
        cmd.arg("-p").arg(&prompt);
        if !model.is_empty() {
            cmd.arg("--model").arg(&model);
        }
        for a in &extra {
            cmd.arg(a);
        }
        cmd.creation_flags(CREATE_NO_WINDOW);

        let summary = match cmd.output() {
            Ok(o) if o.status.success() => {
                let reply = String::from_utf8_lossy(&o.stdout);
                let reply: String = reply.trim().replace('\n', " ").chars().take(60).collect();
                lang.run_ok(&started, &reply)
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr);
                let err: String = err.trim().replace('\n', " ").chars().take(60).collect();
                lang.run_failed(&started, o.status.code(), &err)
            }
            Err(e) => lang.run_error(&started, &e.to_string()),
        };

        append_log(&logp, &summary);
        if let Some(s) = STATE.get() {
            s.lock().unwrap().last_run = summary;
        }
        RUNNING.store(false, Ordering::SeqCst);

        let h = HWND_RAW.load(Ordering::SeqCst);
        if h != 0 {
            unsafe {
                let _ = PostMessageW(
                    HWND(h as *mut core::ffi::c_void),
                    WM_APP_REFRESH,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
    });
}

fn scheduler_tick() {
    if RUNNING.load(Ordering::SeqCst) {
        return;
    }
    let mut due = false;
    {
        let mut s = state().lock().unwrap();
        if !s.cfg.enabled {
            return;
        }
        match s.cfg.mode {
            Mode::Interval => {
                let now = Instant::now();
                match s.next_due {
                    Some(nd) if now >= nd => {
                        due = true;
                        s.next_due =
                            Some(now + Duration::from_secs(s.cfg.interval_minutes * 60));
                    }
                    None => reset_interval(&mut s),
                    _ => {}
                }
            }
            Mode::Daily => {
                let st = local_time();
                let key = ymdhm_key(&st);
                let cur = (st.wHour as u8, st.wMinute as u8);
                if s.cfg.daily_times.iter().any(|&t| t == cur) && s.last_fired_key != key {
                    due = true;
                    s.last_fired_key = key;
                }
            }
        }
    }
    if due {
        trigger_run();
    }
}

// ---- tray icon + menu ------------------------------------------------------

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn set_tip(tip: &mut [u16; 128], s: &str) {
    let w: Vec<u16> = s.encode_utf16().take(127).collect();
    for (i, c) in w.iter().enumerate() {
        tip[i] = *c;
    }
    tip[w.len()] = 0;
}

fn status_text(s: &AppState) -> String {
    let lang = s.cfg.language;
    let head = if s.cfg.enabled {
        format!("● {}", lang.running())
    } else {
        format!("○ {}", lang.paused())
    };
    let sched = match s.cfg.mode {
        Mode::Interval => {
            let mins = s
                .next_due
                .map(|nd| nd.saturating_duration_since(Instant::now()).as_secs() / 60)
                .unwrap_or(0);
            lang.sched_interval(s.cfg.interval_minutes, mins)
        }
        Mode::Daily => {
            let times: Vec<String> = s
                .cfg
                .daily_times
                .iter()
                .map(|(h, m)| format!("{:02}:{:02}", h, m))
                .collect();
            lang.sched_daily(&times.join(","))
        }
    };
    format!("{} · {}", head, sched)
}

fn make_nid(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    nid
}

fn add_tray(hwnd: HWND, hicon: HICON) {
    let mut nid = make_nid(hwnd);
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_APP_TRAY;
    nid.hIcon = hicon;
    let tip = state().lock().unwrap().cfg.language.tray_tip();
    set_tip(&mut nid.szTip, tip);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

fn update_tooltip(hwnd: HWND) {
    let tip = {
        let s = state().lock().unwrap();
        let base = status_text(&s);
        if s.last_run.is_empty() {
            base
        } else {
            format!("{}\n{}", base, s.last_run)
        }
    };
    let mut nid = make_nid(hwnd);
    nid.uFlags = NIF_TIP;
    set_tip(&mut nid.szTip, &tip);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

fn remove_tray(hwnd: HWND) {
    let nid = make_nid(hwnd);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

fn show_menu(hwnd: HWND) {
    unsafe {
        let menu = match CreatePopupMenu() {
            Ok(m) => m,
            Err(_) => return,
        };

        let (status, lastrun, enabled, lang) = {
            let s = state().lock().unwrap();
            (status_text(&s), s.last_run.clone(), s.cfg.enabled, s.cfg.language)
        };
        let status_w = wide(&status);
        let _ = AppendMenuW(menu, MF_GRAYED, ID_STATUS, PCWSTR(status_w.as_ptr()));
        if !lastrun.is_empty() {
            let lr_w = wide(&lastrun);
            let _ = AppendMenuW(menu, MF_GRAYED, ID_LASTRUN, PCWSTR(lr_w.as_ptr()));
        }
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

        // AppendMenuW copies the string, so these temporaries can be dropped after.
        let s_enable = wide(lang.menu_enable());
        let s_runnow = wide(lang.menu_run_now());
        let s_config = wide(lang.menu_edit_config());
        let s_log = wide(lang.menu_open_log());
        let s_reload = wide(lang.menu_reload());
        let s_quit = wide(lang.menu_quit());

        let toggle_flags = MF_STRING | if enabled { MF_CHECKED } else { MF_UNCHECKED };
        let _ = AppendMenuW(menu, toggle_flags, ID_TOGGLE, PCWSTR(s_enable.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, ID_RUNNOW, PCWSTR(s_runnow.as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_CONFIG, PCWSTR(s_config.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, ID_LOG, PCWSTR(s_log.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, ID_RELOAD, PCWSTR(s_reload.as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, PCWSTR(s_quit.as_ptr()));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        // Required so the menu dismisses when the user clicks elsewhere.
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
    }
}

fn open_in_notepad(path: &PathBuf) {
    let _ = Command::new("notepad").arg(path).spawn();
}

fn handle_command(hwnd: HWND, id: usize) {
    match id {
        ID_TOGGLE => {
            {
                let mut s = state().lock().unwrap();
                s.cfg.enabled = !s.cfg.enabled;
                reset_interval(&mut s);
                let path = s.config_path.clone();
                write_config_file(&path, &s.cfg);
            }
            update_tooltip(hwnd);
        }
        ID_RUNNOW => trigger_run(),
        ID_CONFIG => {
            let p = state().lock().unwrap().config_path.clone();
            open_in_notepad(&p);
        }
        ID_LOG => {
            let (p, lang) = {
                let s = state().lock().unwrap();
                (s.log_path.clone(), s.cfg.language)
            };
            if !p.exists() {
                append_log(&p, lang.log_created());
            }
            open_in_notepad(&p);
        }
        ID_RELOAD => {
            {
                let mut s = state().lock().unwrap();
                let path = s.config_path.clone();
                if let Ok(text) = std::fs::read_to_string(&path) {
                    s.cfg = parse_config(&text);
                }
                s.last_fired_key = 0;
                reset_interval(&mut s);
            }
            update_tooltip(hwnd);
        }
        ID_QUIT => {
            remove_tray(hwnd);
            unsafe { PostQuitMessage(0) };
        }
        _ => {}
    }
}

// ---- tray icon bitmap (drawn at runtime, no asset file) --------------------

fn create_icon() -> HICON {
    unsafe {
        let fallback = || LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
        let sz: i32 = 32;
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = sz;
        bmi.bmiHeader.biHeight = -sz; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = 0; // BI_RGB

        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbm = match CreateDIBSection(
            None,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        ) {
            Ok(h) if !bits.is_null() => h,
            _ => return fallback(),
        };

        // Draw a filled disc in Claude's terracotta (#D97757), transparent outside.
        let px = bits as *mut u32;
        let c = 15.5f32;
        let r = 14.0f32;
        for y in 0..sz {
            for x in 0..sz {
                let dx = x as f32 - c;
                let dy = y as f32 - c;
                let d = (dx * dx + dy * dy).sqrt();
                // memory is little-endian BGRA -> u32 = A<<24 | R<<16 | G<<8 | B
                let val = if d <= r { 0xFFD9_7757u32 } else { 0x0000_0000u32 };
                *px.add((y * sz + x) as usize) = val;
            }
        }

        let hmask = CreateBitmap(sz, sz, 1, 1, None);
        let ii = ICONINFO {
            fIcon: BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hmask,
            hbmColor: hbm,
        };
        let hicon = CreateIconIndirect(&ii).unwrap_or_else(|_| fallback());
        let _ = DeleteObject(HGDIOBJ(hbm.0));
        let _ = DeleteObject(HGDIOBJ(hmask.0));
        hicon
    }
}

// ---- window procedure ------------------------------------------------------

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_APP_TRAY => {
            let evt = (lparam.0 as u32) & 0xFFFF;
            if evt == WM_RBUTTONUP || evt == WM_LBUTTONUP || evt == WM_CONTEXTMENU {
                show_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_TIMER => {
            scheduler_tick();
            LRESULT(0)
        }
        WM_APP_REFRESH => {
            update_tooltip(hwnd);
            LRESULT(0)
        }
        WM_COMMAND => {
            handle_command(hwnd, (wparam.0 & 0xFFFF) as usize);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ---- main ------------------------------------------------------------------

fn main() -> Result<()> {
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let config_path = dir.join("claude-wakeup.toml");
    let log_path = dir.join("claude-wakeup.log");

    let cfg = match std::fs::read_to_string(&config_path) {
        Ok(text) => parse_config(&text),
        Err(_) => {
            let cd = default_config_data();
            write_config_file(&config_path, &cd);
            cd
        }
    };

    let mut app = AppState {
        cfg,
        next_due: None,
        last_fired_key: 0,
        last_run: String::new(),
        config_path,
        log_path,
    };
    reset_interval(&mut app);
    STATE.set(Mutex::new(app)).ok();

    unsafe {
        let hinst = HINSTANCE(GetModuleHandleW(None)?.0);
        let class = w!("ClaudeWakeupWndClass");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!("ClaudeWakeup"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            hinst,
            None,
        )?;
        HWND_RAW.store(hwnd.0 as isize, Ordering::SeqCst);

        let hicon = create_icon();
        add_tray(hwnd, hicon);
        let _ = SetTimer(hwnd, TIMER_ID, TIMER_MS, None);
        update_tooltip(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}
