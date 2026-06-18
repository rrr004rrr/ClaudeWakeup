//! Small shared helpers (local time formatting, filename sanitizing, resolving
//! the Claude CLI path). Cross-platform: local time comes from `chrono`.

#[cfg(windows)]
use std::path::PathBuf;

use chrono::Local;

/// "YYYY-MM-DD HH:MM:SS" in local time.
pub fn local_time_string() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// "YYYYMMDD-HHMMSS" — for log filenames and task ids.
pub fn timestamp_compact() -> String {
    Local::now().format("%Y%m%d-%H%M%S").to_string()
}

/// Resolve a program name to something the OS can actually spawn.
///
/// On Windows `std::process::Command` only finds `name` and `name.exe` on PATH —
/// it does **not** honor PATHEXT, so an npm-installed `claude.cmd` / `.bat` shim
/// (and Claude's `~\.local\bin` install) shows up as "program not found". There
/// we search PATH plus the common Claude install dirs, trying the usual Windows
/// executable extensions, and return the first hit. On Unix (macOS) `Command`
/// already resolves bare names against PATH, so the name is returned unchanged
/// (the runner widens PATH itself — see `platform::augment_path`).
#[cfg(windows)]
pub fn resolve_program(name: &str) -> String {
    const EXTS: [&str; 4] = ["exe", "cmd", "bat", "com"];
    let name = name.trim();
    if name.is_empty() {
        return name.to_string();
    }

    // Explicit path (contains a separator): use as-is, else try adding an ext.
    if name.contains('\\') || name.contains('/') {
        let p = PathBuf::from(name);
        if p.is_file() {
            return name.to_string();
        }
        for ext in EXTS {
            let c = p.with_extension(ext);
            if c.is_file() {
                return c.to_string_lossy().into_owned();
            }
        }
        return name.to_string();
    }

    // Bare name: search PATH plus common Claude install locations.
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    if let Some(home) = std::env::var_os("USERPROFILE") {
        dirs.push(PathBuf::from(&home).join(".local").join("bin"));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("npm"));
    }

    let already_has_ext = PathBuf::from(name).extension().is_some();
    for dir in &dirs {
        let base = dir.join(name);
        if already_has_ext {
            if base.is_file() {
                return base.to_string_lossy().into_owned();
            }
            continue;
        }
        for ext in EXTS {
            let c = base.with_extension(ext);
            if c.is_file() {
                return c.to_string_lossy().into_owned();
            }
        }
        if base.is_file() {
            return base.to_string_lossy().into_owned();
        }
    }

    name.to_string()
}

/// Non-Windows: `Command` resolves bare names against PATH already.
#[cfg(not(windows))]
pub fn resolve_program(name: &str) -> String {
    name.trim().to_string()
}

/// Keep only ASCII alphanumerics, dashes and underscores — safe for filenames
/// and scheduled-job names.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}
