<div align="center">

# ClaudeWakeup

**A Windows tray app that (1) keeps your Claude usage window warm with cheap scheduled pings, and (2) lets you schedule overnight tasks from a GUI — each backed by a wake-the-PC Task Scheduler job that runs while you're away.**

[![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows)](https://github.com/rrr004rrr/ClaudeWakeup)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-000000?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

English · [繁體中文](README.zh-TW.md)

</div>

---

ClaudeWakeup lives in the notification area (system tray). Beyond the original
keep-warm ping, it now lets you **write tasks ahead of time and run Claude on
them overnight**, then check the results in the morning.

## Two features

**1. Keep-warm ping (cheap)** — periodically runs the smallest possible command,
`claude -p "hi" --model haiku`, in the background to keep the usage window
active. Launched with `CREATE_NO_WINDOW`, so no console flashes.

**2. Overnight task manager** — open the *Task manager* window from the tray and
add a task (name, start time, frequency, working folder, model, skip-permissions,
timeout, and the task message). On **Add**:

- The task is saved to `tasks.json` and a **Windows Task Scheduler job that wakes
  the PC** (`-WakeToRun`) is registered to run `ClaudeWakeup.exe --run-task <id>`
  at your chosen time.
- At that time Windows wakes the sleeping machine and runs the task; during the
  run `SetThreadExecutionState` **keeps the machine awake** (CPU activity alone
  does not prevent sleep — only this request does).
- Full output is saved to `logs\task-<id>-<ts>.log`; the run is killed past its
  timeout.
- In the morning you open the list to see each task's status (pending / running /
  done / failed) and output; **Mark done** removes a task (and its scheduler job).

## Quick start (no build required)

1. Download `ClaudeWakeup.exe`, drop it in any folder, and double-click it.
2. A terracotta dot appears in the tray (click `^` to expand hidden icons).
3. **Left- or right-click** the icon to open the menu.

A config file (`claude-wakeup.toml`), task file (`tasks.json`), and output
(`logs\`) are created next to the executable.

> **Prerequisite:** the [Claude CLI](https://docs.claude.com/en/docs/claude-code)
> must be installed and runnable. If `claude` isn't on your `PATH`, set
> `claude_path` to the full path of `claude.exe`.

## Tray menu

| Item                 | Action                                                |
|----------------------|-------------------------------------------------------|
| **Task manager…**    | Open the task list + create/edit window               |
| **Keep-warm…**       | Settings + live test for the original wakeup ping     |
| **Language**         | Switch English / 繁體中文 (the window re-labels live)  |
| **Quit**             | Remove the icon and exit                              |

> Menu labels are bilingual; switching language mainly affects the window text.

## Task manager window

- The task list (Name / Time / Freq / Status) is on top.
- The form below:
  - **Name**: blank = first line of the task message.
  - **Time (HH:MM)**: 24-hour local time.
  - **Frequency**: Once (next occurrence; tomorrow if today's time passed) or Daily.
  - **Folder**: working directory (use *Browse…*). Blank = the `.exe` folder.
    **Point it at a git repo** so you can `git diff` what Claude did.
  - **Model**: `sonnet` / `opus` / `haiku` (use sonnet or opus for real work).
  - **Skip permissions**: passes `--dangerously-skip-permissions`, letting Claude
    edit files / run commands unattended. **Only enable for a folder you trust.**
  - **Timeout (min)**: the run is killed past this.
  - **Task message**: the prompt sent to Claude (multi-line).
- Buttons:
  - **Edit selected** — load the selected task into the form.
  - **Save task** — update the task being edited (and reschedule it), or add new.
  - **Mark done (remove)** — delete the selected task and its scheduler job.
  - **View output** — open that run's log in Notepad.
  - **New / clear** — empty the form and leave edit mode.
  - **Close** — send the window back to the tray.

## Keep-warm window (the original wakeup feature)

ClaudeWakeup's original function, **separate from scheduled tasks**: periodically
runs a cheap `claude -p` ping to keep the usage window active. Open it from the
tray ("Keep-warm…"):

- **Enable keep-warm**.
- **Schedule**:
  - **Interval** — ping every "Interval (min)" minutes. Fires only while the PC
    is **awake** (in-app timer).
  - **Daily** — ping at each fixed time in "Daily times" (e.g.
    `07:00, 12:00, 17:00, 22:00`, comma-separated). Daily mode **registers a
    wake-the-PC Task Scheduler job**, so it fires even while you're away and the
    machine is asleep. Daily is the default.
- **Model** — `haiku` is cheapest for warming.
- **Ping now** — pings immediately and shows the outcome under "Last result" (so
  you can see it actually did something).
- **Save** — writes back to `claude-wakeup.toml` and creates/removes the keep-warm
  wake-job to match the mode.

> Waking only works while the PC is on AC power (Windows disables wake timers on
> battery by default).

## Important: sleep & wake prerequisites

- Waking only works while the PC is **on AC power** — Windows disables wake timers
  on battery by default.
- Confirm "Allow wake timers" isn't disabled by a corporate group policy.
- The scheduled job runs as the logged-on user; a locked screen at night does not
  block `claude -p`.
- If scheduling fails, the window says so — try running as Administrator.

## Configuration

First run creates `claude-wakeup.toml` next to the executable:

```ini
# UI language: en | zh-TW
language = en

# `claude` (on PATH) or the full path to claude.exe.
claude_path = claude

# Keep-warm pinger: a cheap periodic `claude -p` to keep the usage window active.
warm_enabled = true
warm_interval_minutes = 300
warm_model = haiku
warm_prompt = hi
# Also notify Feishu after each keep-warm ping (noisy; off by default).
warm_notify = false

# Feishu/Lark bot webhook: notified when a task finishes. Empty = off.
feishu_hook = https://open.feishu.cn/open-apis/bot/v2/hook/xxxxxxxx
```

Language can also be switched from the tray menu (it writes back to this file).

**Task-done notification (Feishu)**: after each task finishes, a text message is
`curl`-POSTed to `feishu_hook` (✅ done / ❌ failed, with the task name, outcome,
time, and output-log path) — so you learn when a task completes. Clear the
`feishu_hook` value to turn it off.

**Keep-warm notification (optional)**: by default keep-warm pings do **not**
notify (they're far more frequent than tasks). To notify on every ping, tick
"Enable push" in **Keep-warm**, or set `warm_notify = true` in the config (a
`feishu_hook` is still required). "Ping now" follows the "Enable push" checkbox,
so it doubles as a push test.

## Run at startup

Place a shortcut to `ClaudeWakeup.exe` in your Startup folder:

```bat
install-startup.bat            :: install
install-startup.bat remove     :: uninstall
```

## Build from source

Requires the [Rust toolchain](https://rustup.rs/).

```bat
build.bat
:: or
cargo build --release
```

Output: `target\release\ClaudeWakeup.exe` (a single file; the tray icon is
embedded from `assets\icon.ico`).

## Project layout

```
src/
├── main.rs        Entry point: --run-task dispatch, keep-warm thread, launch GUI
├── ui.rs          Tray + task-manager window (native-windows-gui)
├── task.rs        Task struct + tasks.json read/write (serde)
├── scheduler.rs   Register/remove a wake-the-PC Task Scheduler job per task
├── runner.rs      Headless task execution (--run-task), keep-warm ping, keep-awake
├── config.rs      Settings (language + keep-warm) read/write
├── i18n.rs        en / zh-TW localization
└── util.rs        Time formatting, filename sanitizing
assets/icon.ico    Tray icon (embedded)
build.bat          Release build helper
install-startup.bat  Create/remove a Startup shortcut
```

## License

[MIT](LICENSE) © 2026 rrr004rrr
