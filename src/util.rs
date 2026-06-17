//! Small shared helpers (local time formatting, filename sanitizing).

use std::path::PathBuf;

use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::System::SystemInformation::GetLocalTime;

pub fn local_time() -> SYSTEMTIME {
    unsafe { GetLocalTime() }
}

/// "YYYY-MM-DD HH:MM:SS" in local time.
pub fn local_time_string() -> String {
    let st = local_time();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
    )
}

/// "YYYYMMDD-HHMMSS" — for log filenames and task ids.
pub fn timestamp_compact() -> String {
    let st = local_time();
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
    )
}

/// Resolve a program name to a path the OS can actually spawn.
///
/// On Windows `std::process::Command` only finds `name` and `name.exe` on PATH —
/// it does **not** honor PATHEXT, so an npm-installed `claude.cmd` or a `.bat`
/// shim (and Claude's `~\.local\bin` install) shows up as "program not found".
/// This searches PATH (plus a couple of common Claude install dirs) trying the
/// usual Windows executable extensions and returns the first hit. Falls back to
/// the input unchanged so the caller's spawn error still surfaces if nothing
/// matches. `.cmd`/`.bat` results are safe to hand to `Command` on modern Rust
/// (it runs batch files via `cmd.exe` with proper argument escaping).
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

/// Keep only ASCII alphanumerics, dashes and underscores — safe for filenames
/// and Task Scheduler job names.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}
