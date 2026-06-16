//! Small shared helpers (local time formatting, filename sanitizing).

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

/// Keep only ASCII alphanumerics, dashes and underscores — safe for filenames
/// and Task Scheduler job names.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}
