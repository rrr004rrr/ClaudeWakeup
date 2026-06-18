//! macOS backend.
//!
//!  - Scheduling: a per-job launchd LaunchAgent (`~/Library/LaunchAgents/*.plist`)
//!    with a `StartCalendarInterval`. launchd runs the job at the given clock time
//!    (and, if the Mac was asleep and pmset woke it, right then).
//!  - Waking: `pmset schedule wake "<datetime>"` arms hardware wake-ups for the
//!    next few occurrences of each scheduled time. pmset needs root, so this is
//!    run through an `osascript ... with administrator privileges` prompt — it is
//!    only ever called from the GUI (it re-arms on launch and on save).
//!  - Keep-awake: a `caffeinate -i` child held for the duration of a run.
//!  - "once" tasks: launchd has no native one-shot, so the job uses a daily
//!    calendar trigger and the runner unregisters it after a successful run.

use std::path::Path;
use std::process::{Child, Command};

use chrono::{Duration, Local, Timelike};

use crate::task::Task;

/// No console-hiding needed on macOS.
pub fn configure_command(_cmd: &mut Command) {}

/// launchd runs jobs with a minimal PATH (`/usr/bin:/bin:…`) that excludes where
/// `claude` is usually installed (`~/.local/bin`, Homebrew, npm). Prepend the
/// common locations so the CLI (and the tools it shells out to) can be found.
pub fn augment_path(cmd: &mut Command) {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut dirs = vec![
        format!("{home}/.local/bin"),
        format!("{home}/.bun/bin"),
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".to_string(),
        "/usr/bin".to_string(),
        "/bin".to_string(),
        "/usr/sbin".to_string(),
        "/sbin".to_string(),
    ];
    if let Ok(existing) = std::env::var("PATH") {
        dirs.push(existing);
    }
    cmd.env("PATH", dirs.join(":"));
}

/// RAII guard: holds a `caffeinate -i` child that prevents idle sleep while alive.
pub struct KeepAwake(Option<Child>);

pub fn keep_awake() -> KeepAwake {
    let child = Command::new("/usr/bin/caffeinate").arg("-i").spawn().ok();
    KeepAwake(child)
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        if let Some(mut c) = self.0.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// Open a file or folder in the default app.
pub fn open_path(path: &str) {
    let _ = Command::new("/usr/bin/open").arg(path).spawn();
}

// ---- launchd ---------------------------------------------------------------

fn launch_agents_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    Path::new(&home).join("Library").join("LaunchAgents")
}

fn task_label(id: &str) -> String {
    format!("com.claudewakeup.task.{}", crate::util::sanitize(id))
}

const WARM_LABEL: &str = "com.claudewakeup.warm";

fn plist_path(label: &str) -> std::path::PathBuf {
    launch_agents_dir().join(format!("{label}.plist"))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// One `<dict><key>Hour</key>…<key>Minute</key>…</dict>` calendar entry.
fn cal_entry(h: u8, m: u8) -> String {
    format!(
        "<dict><key>Hour</key><integer>{}</integer>\
         <key>Minute</key><integer>{}</integer></dict>",
        h, m
    )
}

fn program_args_xml(exe: &Path, args: &[&str]) -> String {
    let mut out = String::from("<array>");
    out.push_str(&format!("<string>{}</string>", xml_escape(&exe.display().to_string())));
    for a in args {
        out.push_str(&format!("<string>{}</string>", xml_escape(a)));
    }
    out.push_str("</array>");
    out
}

fn build_plist(label: &str, exe: &Path, workdir: &Path, prog_args: &[&str], calendar_xml: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n\
         <key>Label</key><string>{label}</string>\n\
         <key>ProgramArguments</key>{args}\n\
         <key>WorkingDirectory</key><string>{wd}</string>\n\
         <key>StartCalendarInterval</key>{cal}\n\
         <key>ProcessType</key><string>Background</string>\n\
         </dict>\n</plist>\n",
        label = xml_escape(label),
        args = program_args_xml(exe, prog_args),
        wd = xml_escape(&workdir.display().to_string()),
        cal = calendar_xml,
    )
}

/// Write the plist and (re)load it — but only if its contents actually changed.
/// Re-loading an unchanged agent makes macOS show the "App background activity"
/// notification every launch, so an unchanged plist is left completely alone.
fn apply(label: &str, plist: &str) -> Result<(), String> {
    let path = plist_path(label);
    if std::fs::read_to_string(&path).map(|cur| cur == plist).unwrap_or(false) {
        return Ok(());
    }
    let dir = launch_agents_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(&path, plist).map_err(|e| e.to_string())?;
    launchctl_reload(&path)
}

fn launchctl_reload(plist: &Path) -> Result<(), String> {
    // Unload first (ignore errors) so a re-register picks up changes.
    let _ = Command::new("/bin/launchctl").arg("unload").arg(plist).output();
    let out = Command::new("/bin/launchctl")
        .arg("load")
        .arg("-w")
        .arg(plist)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn launchctl_remove(label: &str) -> Result<(), String> {
    let plist = plist_path(label);
    let _ = Command::new("/bin/launchctl").arg("unload").arg(&plist).output();
    let _ = std::fs::remove_file(&plist);
    Ok(())
}

/// Create (or replace) the launchd job that runs `--run-task <id>`.
pub fn register(exe: &Path, workdir: &Path, task: &Task) -> Result<(), String> {
    let (h, m) = parse_hhmm(&task.time);
    let label = task_label(&task.id);
    let cal = cal_entry(h, m); // daily; "once" is finalized by the runner.
    let plist = build_plist(&label, exe, workdir, &["--run-task", &task.id], &cal);
    apply(&label, &plist)
}

/// Remove the launchd job for a task id.
pub fn unregister(id: &str) -> Result<(), String> {
    launchctl_remove(&task_label(id))
}

/// Register (or replace) the keep-warm job firing `--keep-warm` at each time.
pub fn register_warm(exe: &Path, workdir: &Path, times: &[(u8, u8)]) -> Result<(), String> {
    if times.is_empty() {
        return unregister_warm();
    }
    // StartCalendarInterval may be an array of dicts for multiple daily times.
    let entries: String = times.iter().map(|(h, m)| cal_entry(*h, *m)).collect();
    let cal = format!("<array>{entries}</array>");
    let plist = build_plist(WARM_LABEL, exe, workdir, &["--keep-warm"], &cal);
    apply(WARM_LABEL, &plist)
}

pub fn unregister_warm() -> Result<(), String> {
    launchctl_remove(WARM_LABEL)
}

// ---- pmset wake ------------------------------------------------------------

/// Ask macOS to wake the machine for each scheduled time. Arms the next few
/// occurrences of every distinct time via `pmset schedule wake`, run through one
/// admin prompt. Passing an empty slice just clears our scheduled wakes.
///
/// Note: `pmset schedule cancelall` clears *all* one-time scheduled power events,
/// not just ours — acceptable for an app whose job is to manage wake-ups.
pub fn sync_wake_schedule(times: &[(u8, u8)]) -> Result<(), String> {
    let mut cmds = vec!["/usr/bin/pmset schedule cancelall".to_string()];
    for dt in next_occurrences(times, 3) {
        // Inner double quotes are escaped for AppleScript (\\\" -> \").
        cmds.push(format!("/usr/bin/pmset schedule wake \\\"{}\\\"", dt));
    }
    let shell = cmds.join("; ");
    let script = format!("do shell script \"{shell}\" with administrator privileges");

    let out = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// The next `days` daily occurrences of each (h, m), formatted for `pmset`
/// ("MM/dd/yy HH:MM:SS"), skipping any already in the past today.
fn next_occurrences(times: &[(u8, u8)], days: i64) -> Vec<String> {
    let now = Local::now();
    let mut out = Vec::new();
    for (h, m) in times {
        let base = match now
            .with_hour(*h as u32)
            .and_then(|t| t.with_minute(*m as u32))
            .and_then(|t| t.with_second(0))
        {
            Some(b) => b,
            None => continue,
        };
        for day in 0..days {
            let dt = base + Duration::days(day);
            if dt > now {
                out.push(dt.format("%m/%d/%y %H:%M:%S").to_string());
            }
        }
    }
    out
}

fn parse_hhmm(s: &str) -> (u8, u8) {
    let mut it = s.split(':');
    let h = it.next().and_then(|x| x.trim().parse().ok()).unwrap_or(3);
    let m = it.next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
    (h, m)
}
