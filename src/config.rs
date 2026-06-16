//! App settings (language + keep-warm pinger), persisted as claude-wakeup.toml
//! next to the executable. Hand-parsed key=value — no TOML dependency.

use std::path::{Path, PathBuf};

use crate::i18n::Lang;

#[derive(Clone)]
pub struct Config {
    pub language: Lang,
    pub claude_path: String,
    pub warm_enabled: bool,
    /// "interval" | "daily".
    pub warm_mode: String,
    /// interval mode: minutes between pings.
    pub warm_interval_minutes: u64,
    /// daily mode: fixed local clock times.
    pub warm_daily_times: Vec<(u8, u8)>,
    pub warm_model: String,
    pub warm_prompt: String,
    /// Feishu (Lark) custom-bot webhook; notified when a task finishes. Empty = off.
    pub feishu_hook: String,
    /// Also send a Feishu notification after each keep-warm ping. Off by default
    /// (warm pings are frequent — this would be noisy).
    pub warm_notify: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            language: Lang::ZhTw,
            claude_path: "claude".to_string(),
            warm_enabled: true,
            warm_mode: "daily".to_string(),
            warm_interval_minutes: 300,
            warm_daily_times: vec![(7, 0), (12, 0), (17, 0), (22, 0)],
            warm_model: "haiku".to_string(),
            warm_prompt: "hi".to_string(),
            feishu_hook:
                "https://open.feishu.cn/open-apis/bot/v2/hook/5185de2e-16ed-482b-b8e9-333bf80204fc"
                    .to_string(),
            warm_notify: false,
        }
    }
}

impl Config {
    pub fn is_daily(&self) -> bool {
        self.warm_mode.eq_ignore_ascii_case("daily")
    }
}

fn config_path(dir: &Path) -> PathBuf {
    dir.join("claude-wakeup.toml")
}

fn parse_times(v: &str) -> Vec<(u8, u8)> {
    v.split(',')
        .filter_map(|t| {
            let (h, m) = t.trim().split_once(':')?;
            Some((h.trim().parse().ok()?, m.trim().parse().ok()?))
        })
        .collect()
}

fn times_str(times: &[(u8, u8)]) -> String {
    times
        .iter()
        .map(|(h, m)| format!("{:02}:{:02}", h, m))
        .collect::<Vec<_>>()
        .join(", ")
}

fn as_mode(v: &str) -> String {
    if v.eq_ignore_ascii_case("daily") {
        "daily".to_string()
    } else {
        "interval".to_string()
    }
}

impl Config {
    pub fn load(dir: &Path) -> Config {
        match std::fs::read_to_string(config_path(dir)) {
            Ok(text) => {
                let mut c = Config::default();
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((k, v)) = line.split_once('=') {
                        let k = k.trim().to_lowercase();
                        let v = v.trim().to_string();
                        match k.as_str() {
                            "language" => c.language = Lang::parse(&v),
                            "claude_path" if !v.is_empty() => c.claude_path = v,
                            // current keys
                            "warm_enabled" => {
                                c.warm_enabled = v.eq_ignore_ascii_case("true") || v == "1"
                            }
                            "warm_mode" => c.warm_mode = as_mode(&v),
                            "warm_interval_minutes" => {
                                if let Ok(n) = v.parse::<u64>() {
                                    c.warm_interval_minutes = n.max(1);
                                }
                            }
                            "warm_daily_times" => c.warm_daily_times = parse_times(&v),
                            "warm_model" if !v.is_empty() => c.warm_model = v,
                            "warm_prompt" if !v.is_empty() => c.warm_prompt = v,
                            "feishu_hook" => c.feishu_hook = v,
                            "warm_notify" => {
                                c.warm_notify = v.eq_ignore_ascii_case("true") || v == "1"
                            }
                            // legacy keys (migrate from the pre-GUI config)
                            "enabled" => {
                                c.warm_enabled = v.eq_ignore_ascii_case("true") || v == "1"
                            }
                            "mode" => c.warm_mode = as_mode(&v),
                            "interval_minutes" => {
                                if let Ok(n) = v.parse::<u64>() {
                                    c.warm_interval_minutes = n.max(1);
                                }
                            }
                            "daily_times" => c.warm_daily_times = parse_times(&v),
                            "model" if !v.is_empty() => c.warm_model = v,
                            "prompt" if !v.is_empty() => c.warm_prompt = v,
                            _ => {}
                        }
                    }
                }
                c
            }
            Err(_) => {
                let c = Config::default();
                c.save(dir);
                c
            }
        }
    }

    pub fn save(&self, dir: &Path) {
        let text = format!(
            "# ClaudeWakeup settings. Edited from the tray app; safe to hand-edit too.\n\
             \n\
             # UI language: en | zh-TW\n\
             language = {lang}\n\
             \n\
             # `claude` (on PATH) or the full path to claude.exe.\n\
             claude_path = {claude}\n\
             \n\
             # Keep-warm pinger: a cheap periodic `claude -p` to keep the usage window active.\n\
             warm_enabled = {we}\n\
             # Schedule: interval (every N minutes) or daily (fixed clock times).\n\
             warm_mode = {wmode}\n\
             warm_interval_minutes = {wi}\n\
             warm_daily_times = {wdt}\n\
             warm_model = {wm}\n\
             warm_prompt = {wp}\n\
             # Also notify Feishu after each keep-warm ping (noisy; off by default).\n\
             warm_notify = {wn}\n\
             \n\
             # Feishu/Lark bot webhook — notified when a task finishes. Empty = off.\n\
             feishu_hook = {fh}\n",
            lang = self.language.code(),
            claude = self.claude_path,
            we = self.warm_enabled,
            wmode = self.warm_mode,
            wi = self.warm_interval_minutes,
            wdt = times_str(&self.warm_daily_times),
            wm = self.warm_model,
            wp = self.warm_prompt,
            wn = self.warm_notify,
            fh = self.feishu_hook,
        );
        let _ = std::fs::write(config_path(dir), text);
    }
}
