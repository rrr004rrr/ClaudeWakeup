# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-06-16

### Added
- **Overnight task manager** with a GUI (built on native-windows-gui). From the
  tray, open a window to create tasks (name, start time, frequency, working
  folder, model, skip-permissions, timeout, and the task message). Each task is
  shown in a list with its status; mark one **done** to remove it.
- Each task is backed by a **Windows Task Scheduler job that wakes the PC**
  (`-WakeToRun`) and runs `ClaudeWakeup.exe --run-task <id>` at the chosen time —
  so tasks run overnight even while the machine sleeps and you're away.
- Headless task execution keeps the machine awake for the run via
  `SetThreadExecutionState`, captures full output to `logs\task-<id>-<ts>.log`,
  enforces a timeout, and writes the result status back to `tasks.json`.
- **Editing tasks**: select a task → *Edit selected* loads it into the form;
  *Save task* updates it and re-registers its scheduler job.
- **Keep-warm window** (the original wakeup feature, kept separate from tasks):
  enable, **interval or daily schedule** (daily = fixed clock times like
  `07:00, 12:00, 17:00, 22:00`), model, plus *Ping now*, which shows the result
  under "Last result" so the action is visible. Legacy `mode`/`daily_times`
  config is migrated automatically.
- **Daily keep-warm wakes the PC**: daily mode registers a single Task Scheduler
  job (one wake trigger per time) running `--keep-warm`, so pings fire even while
  the machine sleeps / you're away. Interval mode stays an in-app, awake-only
  timer. The job is created/removed automatically on save and on startup.
- Tray menu **language switch** (English / 繁體中文); the manager window
  re-labels live.

- **Feishu/Lark notification on task completion**: after a task finishes, a text
  message (✅ done / ❌ failed, with name, outcome, time, output-log path) is
  POSTed to the configured `feishu_hook` via the built-in `curl.exe` (no HTTP
  dependency). Empty hook = off.
- **Optional keep-warm notification** (`warm_notify`, off by default): when on,
  each scheduled keep-warm ping also POSTs a 🔥 OK / ❌ failed line to
  `feishu_hook`. Toggled by a "Notify Feishu" checkbox in the Keep-warm window.
  The manual *Ping now* test never notifies (its result is shown in the window).
- Wider task-manager window with roomier action buttons.

### Changed
- UI rebuilt on native-windows-gui; binary is ~390 KB (was ~250 KB).
- Config simplified to `language`, `claude_path`, and `warm_*` keys; the old
  `mode`/`daily_times`/`interval_minutes` keep-warm scheduling is now a single
  `warm_interval_minutes`.
- Localization trimmed to English + 繁體中文 (was also 简体中文 / 日本語).

## [0.1.0] - 2026-06-15

### Added
- Windows system-tray app that pings the Claude CLI on a schedule to keep the
  token / usage window warm.
- Two schedule modes: `interval` (every N minutes) and `daily` (fixed local
  clock times).
- Multi-language UI with **English as the default** — English, 繁體中文,
  简体中文, and 日本語 — selectable via the `language` key in the config file.
- Silent background pings via `CREATE_NO_WINDOW` (no console flashes).
- Tray menu with live status, last-run result, run-now, config/log access, and
  reload.
- Self-documenting `claude-wakeup.toml` config and `claude-wakeup.log` log,
  generated next to the executable.
- `install-startup.bat` helper to add/remove a Startup shortcut.
- Size-optimized release build profile (`opt-level = "z"`, LTO, stripped,
  `panic = abort`).

[0.2.0]: https://github.com/rrr004rrr/ClaudeWakeup/releases/tag/v0.2.0
[0.1.0]: https://github.com/rrr004rrr/ClaudeWakeup/releases/tag/v0.1.0
