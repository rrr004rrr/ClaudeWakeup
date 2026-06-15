# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/rrr004rrr/ClaudeWakeup/releases/tag/v0.1.0
