//! Register / unregister a Windows Task Scheduler job per task. Each job is set
//! to *wake the computer* so it runs even if the machine is asleep and the user
//! is away. We shell out to PowerShell's ScheduledTask cmdlets (no extra deps).

use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::task::Task;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn job_name(id: &str) -> String {
    format!("ClaudeWakeup-{}", id)
}

fn run_ps(script: &str) -> Result<(), String> {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
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
    let script = format!(
        "Unregister-ScheduledTask -TaskName '{}' -Confirm:$false -ErrorAction SilentlyContinue",
        name
    );
    run_ps(&script)
}

fn parse_hhmm(s: &str) -> (u32, u32) {
    let mut it = s.split(':');
    let h = it.next().and_then(|x| x.trim().parse().ok()).unwrap_or(3);
    let m = it.next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
    (h, m)
}
