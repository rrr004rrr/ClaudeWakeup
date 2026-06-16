// ClaudeWakeup — a Windows tray app that (1) keeps the Claude usage window warm
// with cheap periodic pings and (2) runs pre-written overnight tasks, each
// backed by a wake-the-PC Task Scheduler job. GUI built on native-windows-gui.
#![windows_subsystem = "windows"]

mod config;
mod i18n;
mod runner;
mod scheduler;
mod task;
mod ui;
mod util;

use std::path::PathBuf;
use std::time::Duration;

use config::Config;

fn main() {
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_default();

    // Headless task execution: `ClaudeWakeup.exe --run-task <id>` (invoked by the
    // per-task Task Scheduler job). Runs one task and exits — no tray, no window.
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--run-task") {
        if let Some(id) = args.get(pos + 1) {
            runner::run_task_by_id(&dir, id);
        }
        return;
    }
    // Headless keep-warm ping, fired by the daily wake-job (Task Scheduler wakes
    // the PC at each configured time). One ping, then exit.
    if args.iter().any(|a| a == "--keep-warm") {
        runner::run_keep_warm(&Config::load(&dir));
        return;
    }

    // Daytime keep-warm pinger on a background thread. (Only relevant while the
    // machine is awake and in use; overnight tasks use Task Scheduler instead.)
    spawn_keep_warm(dir.clone());

    ui::run(dir, exe);
}

/// In-app keep-warm timer — handles INTERVAL mode only (while the PC is awake).
/// Daily mode is handled by a wake-the-PC Task Scheduler job instead, so it also
/// fires while the machine is asleep / you're away.
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
