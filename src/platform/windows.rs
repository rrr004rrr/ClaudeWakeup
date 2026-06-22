//! Windows backend: Task Scheduler jobs (set to *wake the computer*), driven via
//! PowerShell's ScheduledTask cmdlets, plus the keep-awake power request. No extra
//! crates beyond `windows` for the power API.

use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

use windows::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED, EXECUTION_STATE,
};

use crate::task::Task;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Add `CREATE_NO_WINDOW` so spawned CLIs don't flash a console window.
pub fn configure_command(cmd: &mut Command) {
    cmd.creation_flags(CREATE_NO_WINDOW);
}

/// Task Scheduler inherits the system PATH, so `claude.exe` is normally found;
/// nothing extra to do on Windows.
pub fn augment_path(_cmd: &mut Command) {}

/// RAII guard that keeps the machine awake (system required) until dropped.
pub struct KeepAwake;

pub fn keep_awake() -> KeepAwake {
    // CPU activity alone does NOT stop Windows from sleeping — only this request does.
    unsafe {
        SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
    }
    KeepAwake
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        unsafe {
            SetThreadExecutionState(EXECUTION_STATE(ES_CONTINUOUS.0));
        }
    }
}

/// Open a file or folder with its default handler.
pub fn open_path(path: &str) {
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "start", "", path]);
    configure_command(&mut cmd);
    let _ = cmd.spawn();
}

/// Windows wakes per-task via `-WakeToRun`, so there's nothing extra to arm.
pub fn sync_wake_schedule(_times: &[(u8, u8)]) -> Result<(), String> {
    Ok(())
}

fn job_name(id: &str) -> String {
    format!("ClaudeWakeup-{}", id)
}

fn run_ps(script: &str) -> Result<(), String> {
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script]);
    configure_command(&mut cmd);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Create (or replace) the scheduled job that runs `--run-task <id>`.
pub fn register(exe: &Path, workdir: &Path, task: &Task) -> Result<(), String> {
    let (hh, mm) = parse_hhmm(&task.time);
    let limit_hours = (task.timeout_min / 60) + 2;
    let name = job_name(&task.id);

    // `once`: the next occurrence of HH:MM (today if still ahead, else tomorrow).
    let trigger = if task.freq == "daily" {
        format!("$t = New-ScheduledTaskTrigger -Daily -At ('{:02}:{:02}');", hh, mm)
    } else {
        format!(
            "$d = [DateTime]::Today.AddHours({}).AddMinutes({}); \
             if ($d -lt (Get-Date)) {{ $d = $d.AddDays(1) }}; \
             $t = New-ScheduledTaskTrigger -Once -At $d;",
            hh, mm
        )
    };

    let script = format!(
        "$a = New-ScheduledTaskAction -Execute '{exe}' -Argument '--run-task {id}' -WorkingDirectory '{wd}'; \
         {trigger} \
         $s = New-ScheduledTaskSettingsSet -WakeToRun -StartWhenAvailable -ExecutionTimeLimit (New-TimeSpan -Hours {limit}); \
         Register-ScheduledTask -TaskName '{name}' -Action $a -Trigger $t -Settings $s -Description 'ClaudeWakeup overnight task' -Force | Out-Null",
        exe = exe.display(),
        id = task.id,
        wd = workdir.display(),
        trigger = trigger,
        limit = limit_hours,
        name = name,
    );
    run_ps(&script)
}

const WARM_JOB: &str = "ClaudeWakeup-warm";

/// Register (or replace) the keep-warm wake-job: one job with a daily trigger at
/// each `time`, all set to wake the PC, running `--keep-warm`. Empty times -> the
/// job is removed.
pub fn register_warm(exe: &Path, workdir: &Path, times: &[(u8, u8)]) -> Result<(), String> {
    if times.is_empty() {
        return unregister_warm();
    }
    let mut triggers = String::new();
    let mut names = Vec::new();
    for (i, (h, m)) in times.iter().enumerate() {
        triggers.push_str(&format!(
            "$t{} = New-ScheduledTaskTrigger -Daily -At ('{:02}:{:02}'); ",
            i, h, m
        ));
        names.push(format!("$t{}", i));
    }
    let script = format!(
        "$a = New-ScheduledTaskAction -Execute '{exe}' -Argument '--keep-warm' -WorkingDirectory '{wd}'; \
         {triggers} \
         $s = New-ScheduledTaskSettingsSet -WakeToRun -StartWhenAvailable -ExecutionTimeLimit (New-TimeSpan -Hours 1); \
         Register-ScheduledTask -TaskName '{name}' -Action $a -Trigger @({arr}) -Settings $s -Description 'ClaudeWakeup keep-warm' -Force | Out-Null",
        exe = exe.display(),
        wd = workdir.display(),
        triggers = triggers,
        arr = names.join(","),
        name = WARM_JOB,
    );
    run_ps(&script)
}

pub fn unregister_warm() -> Result<(), String> {
    run_ps(&format!(
        "Unregister-ScheduledTask -TaskName '{}' -Confirm:$false -ErrorAction SilentlyContinue",
        WARM_JOB
    ))
}

/// Remove the scheduled job for a task id (ignores "not found").
pub fn unregister(id: &str) -> Result<(), String> {
    let name = job_name(id);
    run_ps(&format!(
        "Unregister-ScheduledTask -TaskName '{}' -Confirm:$false -ErrorAction SilentlyContinue",
        name
    ))
}

fn parse_hhmm(s: &str) -> (u32, u32) {
    let mut it = s.split(':');
    let h = it.next().and_then(|x| x.trim().parse().ok()).unwrap_or(3);
    let m = it.next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
    (h, m)
}
