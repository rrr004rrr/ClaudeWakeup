//! Headless task execution. `ClaudeWakeup --run-task <id>` runs one task from the
//! store, keeps the machine awake for the duration, captures the full output to
//! logs/, and writes the result status back to tasks.json.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::platform;
use crate::task::{load_tasks, save_tasks, Status, Task};
use crate::util::{local_time_string, resolve_program, sanitize, timestamp_compact};

/// Run the task with the given id. Updates its status in tasks.json.
pub fn run_task_by_id(dir: &Path, id: &str) {
    let cfg = Config::load(dir);
    let mut tasks = load_tasks(dir);
    let idx = match tasks.iter().position(|t| t.id == id) {
        Some(i) => i,
        None => return,
    };

    // Mark running so a morning glance at the file shows it started.
    tasks[idx].status = Status::Running;
    tasks[idx].last_run = local_time_string();
    let _ = save_tasks(dir, &tasks);
    let task = tasks[idx].clone();

    // Keep the machine awake for the whole run (RAII; released on drop).
    let _awake = platform::keep_awake();

    let logs_dir = dir.join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);
    let out_path = logs_dir.join(format!(
        "task-{}-{}.log",
        sanitize(id),
        timestamp_compact()
    ));

    let result = run_one(&cfg, &task, dir, &out_path);

    // Reload (the GUI may have edited the file meanwhile) and update this task.
    let mut tasks = load_tasks(dir);
    let mut note: Option<(bool, String)> = None; // (ok, outcome text)
    if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
        t.output_log = out_path.display().to_string();
        t.last_run = local_time_string();
        let (status, ok, outcome) = match result {
            Ok(Some(0)) => (Status::Done, true, "OK".to_string()),
            Ok(Some(code)) => (Status::Failed, false, format!("exit {code}")),
            Ok(None) => (Status::Failed, false, format!("timed out ({} min)", task.timeout_min)),
            Err(_) => (Status::Failed, false, "error".to_string()),
        };
        t.status = status;
        t.exit_code = match result {
            Ok(Some(code)) => Some(code),
            _ => None,
        };
        note = Some((ok, outcome));
    }
    let _ = save_tasks(dir, &tasks);

    // A "once" task fires only once: remove its scheduled job now that it has run.
    if task.freq != "daily" {
        let _ = platform::unregister(id);
    }

    // Notify every configured Feishu bot that the task finished.
    if let Some((ok, outcome)) = note {
        let text = format!(
            "{icon} ClaudeWakeup 任務{verb}：{name}\n結果：{outcome}\n時間：{time}\n輸出：{log}",
            icon = if ok { "✅" } else { "❌" },
            verb = if ok { "完成" } else { "失敗" },
            name = task.name,
            outcome = outcome,
            time = local_time_string(),
            log = out_path.display(),
        );
        notify_feishu(&cfg.feishu_hooks, &text);
    }
}

/// POST a text message to every configured Feishu/Lark custom-bot webhook, so
/// multiple users/groups can each receive the notification. Best-effort.
pub fn notify_feishu(hooks: &[String], text: &str) {
    for hook in hooks {
        notify_one(hook, text);
    }
}

/// POST a text message to a single Feishu/Lark custom-bot webhook. Uses `curl`
/// (present on Windows 10+ and macOS) so no HTTP/TLS dependency is needed.
fn notify_one(hook: &str, text: &str) {
    if hook.trim().is_empty() {
        return;
    }
    let body = serde_json::json!({
        "msg_type": "text",
        "content": { "text": text },
    })
    .to_string();

    let tmp = std::env::temp_dir().join(format!("cwk-notify-{}.json", timestamp_compact()));
    if std::fs::write(&tmp, &body).is_err() {
        return;
    }
    let mut cmd = Command::new("curl");
    cmd.args([
        "-s",
        "-S",
        "-m",
        "20",
        "-X",
        "POST",
        "-H",
        "Content-Type: application/json; charset=utf-8",
        "--data-binary",
    ])
    .arg(format!("@{}", tmp.display()))
    .arg(hook);
    platform::configure_command(&mut cmd);
    let _ = cmd.output();
    let _ = std::fs::remove_file(&tmp);
}

/// Spawn the Claude CLI, stream full output to `out_path`, enforce the timeout.
/// `Ok(Some(code))` on exit, `Ok(None)` on timeout-kill.
fn run_one(
    cfg: &Config,
    task: &Task,
    dir: &Path,
    out_path: &Path,
) -> std::io::Result<Option<i32>> {
    let out_file = OpenOptions::new().create(true).append(true).open(out_path)?;
    let err_file = out_file.try_clone()?;

    let cwd: PathBuf = if task.dir.trim().is_empty() {
        dir.to_path_buf()
    } else {
        PathBuf::from(&task.dir)
    };

    {
        let mut h = out_file.try_clone()?;
        let _ = writeln!(
            h,
            "=== ClaudeWakeup task \"{name}\" @ {time} ===\n\
             model: {model} · skip_permissions: {skip} · timeout: {to} min\n\
             cwd: {cwd}\n\
             --- task ---\n{prompt}\n--- output ---",
            name = task.name,
            time = local_time_string(),
            model = task.model,
            skip = task.skip_permissions,
            to = task.timeout_min,
            cwd = cwd.display(),
            prompt = task.prompt.trim(),
        );
    }

    let mut cmd = Command::new(resolve_program(&cfg.claude_path));
    cmd.arg("-p").arg(&task.prompt);
    if !task.model.is_empty() {
        cmd.arg("--model").arg(&task.model);
    }
    if task.skip_permissions {
        cmd.arg("--dangerously-skip-permissions");
    }
    cmd.current_dir(&cwd);
    cmd.stdout(Stdio::from(out_file));
    cmd.stderr(Stdio::from(err_file));
    platform::configure_command(&mut cmd);
    platform::augment_path(&mut cmd);

    let mut child = cmd.spawn()?;
    let deadline = Instant::now() + Duration::from_secs(task.timeout_min.max(1) * 60);
    loop {
        match child.try_wait()? {
            Some(status) => return Ok(Some(status.code().unwrap_or(-1))),
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok(None);
                }
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

/// Last keep-warm result line (shared with the UI for visible feedback).
static WARM_RESULT: Mutex<String> = Mutex::new(String::new());

pub fn warm_result() -> String {
    WARM_RESULT.lock().map(|g| g.clone()).unwrap_or_default()
}

/// A cheap "keep the usage window warm" ping (`claude -p hi --model haiku`).
/// Captures the outcome into WARM_RESULT so the UI can show it.
pub fn run_keep_warm(cfg: &Config) {
    let mut cmd = Command::new(resolve_program(&cfg.claude_path));
    cmd.arg("-p").arg(&cfg.warm_prompt);
    if !cfg.warm_model.is_empty() {
        cmd.arg("--model").arg(&cfg.warm_model);
    }
    platform::configure_command(&mut cmd);
    platform::augment_path(&mut cmd);

    let t = local_time_string();
    let (ok, line) = match cmd.output() {
        Ok(o) if o.status.success() => {
            let r = String::from_utf8_lossy(&o.stdout);
            let r: String = r.trim().replace('\n', " ").chars().take(60).collect();
            (true, format!("[{t}] OK — {r}"))
        }
        Ok(o) => {
            let e = String::from_utf8_lossy(&o.stderr);
            let e: String = e.trim().replace('\n', " ").chars().take(60).collect();
            (false, format!("[{t}] failed (exit {:?}) {e}", o.status.code()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (
            false,
            format!("[{t}] claude not found — install the Claude CLI or set claude_path in claude-wakeup.toml"),
        ),
        Err(e) => (false, format!("[{t}] error — {e}")),
    };
    if let Ok(mut g) = WARM_RESULT.lock() {
        *g = line.clone();
    }

    // Optional Feishu notification (off by default — warm pings are frequent).
    if cfg.warm_notify {
        let text = format!(
            "{icon} ClaudeWakeup 保溫 ping {verb}\n{line}",
            icon = if ok { "🔥" } else { "❌" },
            verb = if ok { "OK" } else { "失敗" },
            line = line,
        );
        notify_feishu(&cfg.feishu_hooks, &text);
    }
}
