// ClaudeWakeup — a cross-platform tray / menu-bar app that (1) keeps the Claude
// usage window warm with cheap periodic pings and (2) runs pre-written overnight
// tasks, each backed by an OS scheduled job that wakes the machine (Windows Task
// Scheduler on Windows, launchd + pmset on macOS). GUI built on egui/eframe.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod config;
mod i18n;
mod platform;
mod runner;
mod task;
mod util;

use std::path::PathBuf;
use std::time::Duration;

use config::Config;

fn main() {
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = data_dir(&exe);

    // Headless task execution: `ClaudeWakeup --run-task <id>` (invoked by the
    // per-task scheduled job). Runs one task and exits — no tray, no window.
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--run-task") {
        if let Some(id) = args.get(pos + 1) {
            runner::run_task_by_id(&dir, id);
        }
        return;
    }
    // Headless keep-warm ping, fired by the daily wake-job. One ping, then exit.
    // Print the outcome so launchd/console logs show what happened.
    if args.iter().any(|a| a == "--keep-warm") {
        runner::run_keep_warm(&Config::load(&dir));
        println!("{}", runner::warm_result());
        return;
    }

    // Daytime keep-warm pinger on a background thread. (Only relevant while the
    // machine is awake and in use; overnight tasks use the OS scheduler instead.)
    spawn_keep_warm(dir.clone());

    app::run(dir, exe);
}

/// Where config/tasks/logs live. On Windows, next to the executable. On macOS,
/// `~/.claudewakeup` — a plain hidden folder in $HOME. This avoids two TCC
/// prompts that would otherwise fire on every launch of an unsigned app:
/// `~/Library/Application Support` triggers the "access other app data" prompt,
/// and writing next to a bundle that lives in an iCloud-synced Documents folder
/// triggers the iCloud prompt. Both the GUI and the launchd-invoked headless
/// runner resolve this the same way, so they always agree on the data location.
fn data_dir(exe: &std::path::Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let dir = PathBuf::from(home).join(".claudewakeup");
            let _ = std::fs::create_dir_all(&dir);
            return dir;
        }
    }
    exe.parent().map(|p| p.to_path_buf()).unwrap_or_default()
}

/// In-app keep-warm timer — handles INTERVAL mode only (while the machine is
/// awake). Daily mode is handled by a wake-the-machine scheduled job instead, so
/// it also fires while the machine is asleep / you're away.
fn spawn_keep_warm(dir: PathBuf) {
    std::thread::spawn(move || loop {
        let cfg = Config::load(&dir);
        if cfg.warm_enabled && !cfg.is_daily() {
            std::thread::sleep(Duration::from_secs(cfg.warm_interval_minutes.max(1) * 60));
            let cfg = Config::load(&dir);
            if cfg.warm_enabled && !cfg.is_daily() {
                runner::run_keep_warm(&cfg);
            }
        } else {
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}
