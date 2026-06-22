//! Platform abstraction. Everything OS-specific lives behind this module so the
//! rest of the app (GUI, runner) stays cross-platform. Each backend implements
//! the same surface:
//!
//!  - `register` / `unregister`            — per-task scheduled job
//!  - `register_warm` / `unregister_warm`  — the keep-warm job
//!  - `sync_wake_schedule`                 — ask the OS to *wake* for the times
//!  - `configure_command`                  — hide consoles etc. on spawn
//!  - `keep_awake` -> `KeepAwake`          — RAII: stop the machine sleeping
//!  - `open_path`                          — open a file/folder in the default app
//!
//! Windows uses Task Scheduler (PowerShell) + `SetThreadExecutionState`.
//! macOS uses launchd (LaunchAgent plists) + `pmset` (wake) + `caffeinate`.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;
