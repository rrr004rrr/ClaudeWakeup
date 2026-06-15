<div align="center">

# ClaudeWakeup

**A tiny Windows tray app that pings the Claude CLI on a schedule to keep your usage window warm.**

[![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows)](https://github.com/rrr004rrr/ClaudeWakeup)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-000000?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/rrr004rrr/ClaudeWakeup?include_prereleases)](https://github.com/rrr004rrr/ClaudeWakeup/releases)

English · [繁體中文](README.zh-TW.md)

</div>

---

ClaudeWakeup is a ~250 KB, single-file Windows utility that lives in your
notification area (system tray). On a schedule you choose, it quietly runs the
Claude CLI in the background so your token / usage window stays active — no
terminal, no visible window, no fuss.

## Why it's cheap on tokens

Each wake-up runs the smallest possible command:

```text
claude -p "hi" --model haiku
```

- `-p` / `--print` is Claude's **non-interactive, single-turn** mode.
- `haiku` is the **cheapest** model.
- The process is launched with `CREATE_NO_WINDOW`, so no black console flashes on screen.

The result: every ping costs the bare minimum number of tokens.

## Features

- 🪶 **Tiny & self-contained** — one ~250 KB `.exe`, no installer, no runtime dependencies.
- 🕒 **Two schedule modes** — fixed `interval` (every N minutes) or `daily` clock times.
- 🌐 **Multi-language UI** — English (default), 繁體中文, 简体中文, 日本語. See [Language](#language).
- 🔕 **Silent background pings** — no console window ever appears.
- 📋 **Live tray tooltip & menu** — status, next run time, and the last result at a glance.
- 📝 **Plain-text config & log** — edit in Notepad, reload from the menu.

## Quick start (no build required)

1. Download `ClaudeWakeup.exe` from the [**Releases**](https://github.com/rrr004rrr/ClaudeWakeup/releases) page.
2. Drop it anywhere (Desktop, a tools folder, …) and double-click it.
3. A terracotta dot appears in the tray. If you can't see it, click the `^`
   arrow to expand hidden icons.
4. **Left- or right-click the icon** to open the menu.
5. By default it wakes Claude every 5 hours. To test immediately, click **Run now**.

A config file (`claude-wakeup.toml`) and log file (`claude-wakeup.log`) are
created automatically in the same folder as the `.exe`.

> **Prerequisite:** the [Claude CLI](https://docs.claude.com/en/docs/claude-code) must be
> installed and runnable. If `claude` isn't on your `PATH`, set `claude_path` in
> the config to the full path of `claude.exe`.

## Tray menu

| Item              | Action                                                            |
|-------------------|------------------------------------------------------------------|
| *(status)*        | Running / Paused, schedule, and time until next run (greyed out) |
| *(last result)*   | Outcome of the most recent wake-up (greyed out)                  |
| **Enabled**       | Pause / resume the scheduler (checked = running)                  |
| **Run now**       | Wake Claude immediately                                           |
| **Edit config…**  | Open `claude-wakeup.toml` in Notepad                             |
| **Open log**      | Open `claude-wakeup.log`                                          |
| **Reload config** | Re-read the config after editing                                  |
| **Quit**          | Remove the tray icon and exit                                    |

Hover over the icon to see the current status as a tooltip.

## Configuration

On first run, `claude-wakeup.toml` is generated next to the executable. Edit it
via **Edit config…**, then choose **Reload config** to apply your changes.

```ini
# UI language: en | zh-TW | zh-CN | ja
language = en

# Master switch (true = on / false = paused).
enabled = true

# Schedule mode: interval | daily (fixed clock times)
mode = interval

# interval mode: minutes between wake-ups (Claude's usage window is ~5 h = 300).
interval_minutes = 300

# daily mode: comma-separated 24-hour local times.
daily_times = 09:00, 14:00, 19:00

# Wake-up command. The defaults keep token cost minimal.
claude_path = claude
model = haiku
prompt = hi

# Extra command-line arguments, space-separated (optional).
extra_args =
```

| Key                | Description                                                                          |
|--------------------|-------------------------------------------------------------------------------------|
| `language`         | UI language: `en` (default), `zh-TW`, `zh-CN`, `ja`.                                 |
| `enabled`          | Master on/off switch (also toggled from the tray menu).                              |
| `mode`             | `interval` or `daily`.                                                               |
| `interval_minutes` | In `interval` mode, minutes between wake-ups. Claude's window is ~5 h, so `300`.     |
| `daily_times`      | In `daily` mode, comma-separated 24-hour local times, e.g. `09:00, 14:00, 19:00`.   |
| `claude_path`      | `claude` (uses `PATH`) or the full path to `claude.exe`.                             |
| `model`            | Model alias passed to `--model`. `haiku` is cheapest.                                |
| `prompt`           | Prompt sent on each wake-up. Kept tiny on purpose.                                   |
| `extra_args`       | Extra CLI arguments appended to the command (advanced; usually empty).               |

- **`interval` mode** fires every `interval_minutes` minutes. The first run
  happens *one interval after* startup — use **Run now** to fire immediately.
- **`daily` mode** fires once at each `HH:MM` (local time).

## Language

The UI defaults to **English**. To switch, set `language` in the config and
choose **Reload config** (or restart):

| Value   | Language          |
|---------|-------------------|
| `en`    | English (default) |
| `zh-TW` | 繁體中文           |
| `zh-CN` | 简体中文           |
| `ja`    | 日本語             |

Adding a language is a small, self-contained change in
[`src/i18n.rs`](src/i18n.rs) — one new enum variant and one arm per string. No
external resource files; everything stays inside the single binary.

## Run at startup

Place a shortcut to `ClaudeWakeup.exe` in your Startup folder. The easiest way:

```bat
install-startup.bat            :: install
install-startup.bat remove     :: uninstall
```

Or do it manually:

1. Press <kbd>Win</kbd>+<kbd>R</kbd>, type `shell:startup`, press Enter.
2. Drop a shortcut to `ClaudeWakeup.exe` into the folder that opens.

## Build from source

Requires the [Rust toolchain](https://rustup.rs/) (`cargo`).

```bat
build.bat
:: or
cargo build --release
```

Output: `target\release\ClaudeWakeup.exe` — a single, self-contained file. The
release profile is tuned for size (`opt-level = "z"`, LTO, stripped, panic =
abort).

## Project layout

```
src/
├── main.rs        Win32 tray app: window proc, scheduler, config I/O, icon
└── i18n.rs        Dependency-free localization (en / zh-TW / zh-CN / ja)
build.bat          Release build helper
install-startup.bat  Create/remove a Startup shortcut
Cargo.toml         Crate manifest (size-optimized release profile)
```

## How it works

ClaudeWakeup is built directly on the Win32 API via the
[`windows`](https://crates.io/crates/windows) crate — no GUI framework — which
is why the binary stays small. It creates a message-only window, registers a
tray icon (drawn at runtime, no image asset), and uses a `WM_TIMER` tick as the
scheduler. Wake-up pings run on a background thread so the UI never blocks.

## Contributing

Issues and pull requests are welcome. To add a language, edit `src/i18n.rs` and
update the [Language](#language) table above.

## License

[MIT](LICENSE) © 2026 rrr004rrr
