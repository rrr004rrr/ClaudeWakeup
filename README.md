<div align="center">

# ClaudeWakeup

**A cross-platform tray / menu-bar app that (1) keeps your Claude usage window warm with cheap scheduled pings, and (2) lets you schedule overnight tasks from a GUI — each backed by an OS job that wakes the machine and runs while you're away.**

[![Version](https://img.shields.io/github/v/release/rrr004rrr/ClaudeWakeup?label=version&color=blue)](https://github.com/rrr004rrr/ClaudeWakeup/releases/latest)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-555?logo=apple)](https://github.com/rrr004rrr/ClaudeWakeup)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-000000?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

English · [繁體中文](README.zh-TW.md)

</div>

---

ClaudeWakeup lives in the system tray (Windows) or menu bar (macOS). Beyond the
original keep-warm ping, it lets you **write tasks ahead of time and run Claude on
them overnight**, then check the results in the morning. The GUI is one egui
window with **three tabs** (Tasks / Keep-warm / Push), shared by both platforms.

## Two features

**1. Keep-warm ping (cheap)** — periodically runs the smallest possible command,
`claude -p "hi" --model haiku`, in the background to keep the usage window active.
No console window flashes (hidden on Windows; silent on macOS).

**2. Overnight task manager** — open the *Tasks* tab from the tray/menu and add a
task (name, start time, frequency, working folder, model, skip-permissions,
timeout, and the task message). On **Save**:

- The task is saved to `tasks.json` and an **OS job that wakes the machine** is
  registered to run `ClaudeWakeup --run-task <id>` at your chosen time:
  - **Windows** — a Task Scheduler job with `-WakeToRun`.
  - **macOS** — a launchd LaunchAgent (`StartCalendarInterval`) plus a `pmset`
    hardware-wake (see [macOS wake notes](#macos-sleep--wake-notes)).
- At that time the machine wakes and runs the task; during the run it is **kept
  awake** (`SetThreadExecutionState` on Windows, `caffeinate` on macOS — CPU
  activity alone does not prevent sleep).
- Full output is saved to `logs/task-<id>-<ts>.log`; the run is killed past its
  timeout.
- A "once" task removes its own job after it runs; "daily" repeats.
- In the morning you open the list to see each task's status (pending / running /
  done / failed) and output; **Mark done** removes a task (and its job).

## Quick start

> **Prerequisite:** the [Claude CLI](https://docs.claude.com/en/docs/claude-code)
> must be installed and runnable. If `claude` isn't on your `PATH`, set
> `claude_path` to its full path in the config.

### Windows

1. Download or build `ClaudeWakeup.exe`, drop it in any folder, double-click it.
2. A terracotta dot appears in the tray (click `^` to expand hidden icons).
3. **Left- or right-click** the icon to open the menu.

Config (`claude-wakeup.toml`), tasks (`tasks.json`), and output (`logs/`) are
created next to the executable.

### macOS

1. Build the app bundle: `./build.sh` → produces `dist/ClaudeWakeup.app`.
2. **Move it to `/Applications`** (recommended): `mv dist/ClaudeWakeup.app /Applications/`.
   Running it from an iCloud-synced `~/Documents`/`~/Desktop` makes macOS prompt
   for iCloud access on every launch; `/Applications` is not synced.
3. Launch it: `open /Applications/ClaudeWakeup.app`. An icon appears in the
   **menu bar**; click it to open the menu.

Config/tasks/logs live in `~/.claudewakeup/`. (A plain hidden folder in your home
directory — chosen over `~/Library/Application Support` so an unsigned app doesn't
trigger the macOS "access other app data" prompt on every launch.)

## Tray / menu-bar menu

| Item              | Action                                                |
|-------------------|-------------------------------------------------------|
| **Task manager**  | Open the window on the Tasks tab                      |
| **Keep-warm**     | Open the window on the Keep-warm tab                  |
| **Push**          | Open the window on the Push tab (Feishu webhooks)     |
| **Language**      | Switch English / 繁體中文 (the window re-labels live)  |
| **Quit**          | Remove the icon and exit                              |

> Menu labels are bilingual; switching language affects the window text.
> Closing the window hides it back to the tray/menu bar (it does not quit).

## Tasks tab

- The task list (Name / Time / Freq / Status) is on top — click a row to select it.
- The form below:
  - **Name**: blank = first line of the task message.
  - **Time (HH:MM)**: 24-hour local time.
  - **Frequency**: Once (next occurrence; tomorrow if today's time passed) or Daily.
  - **Folder**: working directory (use *Browse…*). Blank = the data folder.
    **Point it at a git repo** so you can `git diff` what Claude did.
  - **Model**: `sonnet` / `opus` / `haiku` (use sonnet or opus for real work).
  - **Skip permissions**: passes `--dangerously-skip-permissions`, letting Claude
    edit files / run commands unattended. **Only enable for a folder you trust.**
  - **Timeout (min)**: the run is killed past this.
  - **Task message**: the prompt sent to Claude (multi-line).
- Buttons:
  - **Edit selected** — load the selected task into the form.
  - **Save task** — update the task being edited (and reschedule it), or add new.
  - **Mark done (remove)** — delete the selected task and its OS job.
  - **View output** — open that run's log in the default text app.
  - **New / clear** — empty the form and leave edit mode.

## Keep-warm tab (the original wakeup feature)

ClaudeWakeup's original function, **separate from scheduled tasks**: periodically
runs a cheap `claude -p` ping to keep the usage window active.

- **Enable keep-warm**.
- **Schedule**:
  - **Interval** — ping every "Interval (min)" minutes. Fires only while the
    machine is **awake** (in-app timer).
  - **Daily** — ping at each fixed time in "Daily times" (e.g.
    `07:00, 12:00, 17:00, 22:00`, comma-separated). Daily mode **registers a
    wake-the-machine job**, so it fires even while you're away. Daily is default.
- **Model** — `haiku` is cheapest for warming.
- **Ping now** — pings immediately; the outcome shows under "Last result".
- **Save** — writes back to `claude-wakeup.toml` and creates/removes the keep-warm
  wake-job to match the mode.

## Push tab (Feishu/Lark)

When a task finishes, ClaudeWakeup can push a message to one or more
[Feishu/Lark custom-bot webhooks](https://www.feishu.cn/hc/en-US/articles/360024984973)
(keep-warm pings can optionally push too). Manage recipients here instead of
hand-editing the config:

- **Webhook URLs** — one bot webhook URL per line. **Every listed URL is
  notified**, so you can push to multiple users/groups at once.
- **Send test** — posts a test message to all listed webhooks right now (uses the
  current textbox, even before Save), to confirm each bot is wired up.
- **Save** — writes the URLs back to `claude-wakeup.toml` (one `feishu_hook = <url>`
  line per recipient). Clearing the box turns notifications off.

## macOS sleep & wake notes

launchd runs the scheduled jobs at their clock time **while the Mac is awake**
(no extra setup). To also wake a *sleeping* Mac, ClaudeWakeup can arm
`pmset schedule wake` for the next few occurrences of each scheduled time.

- Waking is **opt-in**: press **Arm Mac wake (admin)…** in the Keep-warm tab. It
  covers all scheduled times (tasks + keep-warm). `pmset` needs root, so this is
  the *only* action that shows an admin password prompt (via `osascript … with
  administrator privileges`) — ordinary Save never prompts.
- It arms a few days ahead, so for long unattended stretches press it again
  periodically. `pmset schedule cancelall` clears previous wake events before
  re-arming (this also clears wake events you set manually with `pmset`).
- Waking from sleep generally requires AC power on laptops.

**Background activity & code signing.** The keep-warm/daily jobs are LaunchAgents,
so macOS lists ClaudeWakeup under *System Settings → General → Login Items →
Allow in the Background* and shows a one-time "background activity" notice when a
job is first registered (not on every launch). Because the app isn't signed with
an Apple Developer ID it appears as "from an unidentified developer" — this is
expected for a self-built tool and works as long as the toggle is on. Keep a
single copy at a stable path (e.g. `/Applications`) so it stays one entry.

## Windows sleep & wake notes

- Waking only works while the PC is **on AC power** — Windows disables wake timers
  on battery by default.
- Confirm "Allow wake timers" isn't disabled by a corporate group policy.
- The scheduled job runs as the logged-on user; a locked screen does not block it.
- If scheduling fails, the status line says so — try running as Administrator.

## Configuration

First run creates `claude-wakeup.toml` in the data folder:

```ini
# UI language: en | zh-TW
language = en

# `claude` (on PATH) or the full path to the claude binary.
claude_path = claude

# Keep-warm pinger: a cheap periodic `claude -p` to keep the usage window active.
warm_enabled = true
warm_mode = daily
warm_interval_minutes = 300
warm_daily_times = 07:00, 12:00, 17:00, 22:00
warm_model = haiku
warm_prompt = hi
# Also notify Feishu after each keep-warm ping (noisy; off by default).
warm_notify = false

# Feishu/Lark bot webhooks — every one is notified when a task finishes.
# Add one `feishu_hook = <url>` line per recipient. No lines / empty = off.
feishu_hook = https://open.feishu.cn/open-apis/bot/v2/hook/xxxxxxxx
feishu_hook = https://open.feishu.cn/open-apis/bot/v2/hook/yyyyyyyy
```

Language can also be switched from the menu (it writes back to this file).

**Task-done notification (Feishu)**: after each task finishes, a text message is
`curl`-POSTed to **every** configured `feishu_hook` (✅ done / ❌ failed, with the
task name, outcome, time, and output-log path). Manage the recipient list in the
**Push** tab (one URL per line), or add multiple `feishu_hook = <url>` lines here;
clear them all to turn it off. (`curl` ships with Windows 10+ and macOS.)

**Keep-warm notification (optional)**: by default keep-warm pings do **not**
notify. To notify on every ping, tick "Enable push" in **Keep-warm**, or set
`warm_notify = true` (a `feishu_hook` is still required).

## Run at startup

**Windows** — place a shortcut in your Startup folder:

```bat
install-startup.bat            :: install
install-startup.bat remove     :: uninstall
```

**macOS** — install a LaunchAgent that opens the app at login:

```bash
./install-startup.sh           # install
./install-startup.sh remove    # uninstall
```

## Build from source

Requires the [Rust toolchain](https://rustup.rs/).

**Windows**

```bat
build.bat
:: or
cargo build --release
```

Output: `target\release\ClaudeWakeup.exe` (a single file; tray icon embedded).

**macOS**

```bash
./build.sh
# or just the binary:
cargo build --release
```

`build.sh` also assembles `dist/ClaudeWakeup.app` with `LSUIElement` set (a pure
menu-bar app, no Dock icon).

## Project layout

```
src/
├── main.rs            Entry: --run-task / --keep-warm dispatch, data dir, launch GUI
├── app.rs             Cross-platform GUI: tray + window with Tasks / Keep-warm tabs (egui)
├── platform/
│   ├── mod.rs         Platform abstraction (scheduling, keep-awake, open, wake)
│   ├── windows.rs     Task Scheduler + SetThreadExecutionState
│   └── macos.rs       launchd + pmset wake + caffeinate
├── task.rs            Task struct + tasks.json read/write (serde)
├── runner.rs          Headless task execution (--run-task), keep-warm ping
├── config.rs          Settings (language + keep-warm) read/write
├── i18n.rs            en / zh-TW localization
└── util.rs            Time formatting (chrono), filename sanitizing
assets/icon.ico        Tray icon (embedded; loaded as RGBA at runtime)
build.bat / build.sh   Release build helpers (Windows / macOS)
install-startup.*      Create/remove a startup item
```

## License

[MIT](LICENSE) © 2026 rrr004rrr
