//! The task store: overnight tasks the user scheduled, persisted as tasks.json
//! next to the executable. Each task is backed by a Windows Task Scheduler job
//! (see scheduler.rs) that wakes the PC to run it.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Pending,
    Running,
    Done,
    Failed,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    /// "HH:MM" local time.
    pub time: String,
    /// "once" | "daily".
    pub freq: String,
    /// Working directory; empty = the .exe folder.
    pub dir: String,
    /// The task message sent to Claude as the prompt.
    pub prompt: String,
    /// Model alias: sonnet | opus | haiku.
    pub model: String,
    #[serde(default = "default_true")]
    pub skip_permissions: bool,
    #[serde(default = "default_timeout")]
    pub timeout_min: u64,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub last_run: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub output_log: String,
}

fn default_true() -> bool {
    true
}
fn default_timeout() -> u64 {
    240
}

pub fn tasks_path(dir: &Path) -> PathBuf {
    dir.join("tasks.json")
}

pub fn load_tasks(dir: &Path) -> Vec<Task> {
    match std::fs::read_to_string(tasks_path(dir)) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn save_tasks(dir: &Path, tasks: &[Task]) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(tasks).unwrap_or_else(|_| "[]".to_string());
    std::fs::write(tasks_path(dir), text)
}
