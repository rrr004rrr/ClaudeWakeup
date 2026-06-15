//! Lightweight, dependency-free localization.
//!
//! English is the default language. The active language is selected by the
//! `language` key in `claude-wakeup.toml` (e.g. `language = zh-TW`). Adding a
//! new language means adding one arm to each `match self { ... }` below — no
//! external files, so everything stays inside the single static binary.

/// Supported UI languages. Extend this enum (and the matches below) to add more.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    ZhTw,
    ZhCn,
    Ja,
}

/// Comment lines written into the generated config file, already localized.
pub struct CfgComments {
    pub header: &'static str,
    pub language: &'static str,
    pub enabled: &'static str,
    pub mode: &'static str,
    pub interval: &'static str,
    pub daily: &'static str,
    pub command: &'static str,
    pub extra: &'static str,
}

impl Lang {
    /// Parse a language tag such as `en`, `zh-TW`, `zh_cn`, or `ja`.
    /// Anything unrecognized falls back to English (the default language).
    pub fn parse(s: &str) -> Lang {
        match s.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "zh-tw" | "zh-hant" | "zh-hant-tw" => Lang::ZhTw,
            "zh-cn" | "zh-hans" | "zh-hans-cn" | "zh" => Lang::ZhCn,
            "ja" | "ja-jp" => Lang::Ja,
            _ => Lang::En,
        }
    }

    /// Canonical tag written back into the config file.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::ZhTw => "zh-TW",
            Lang::ZhCn => "zh-CN",
            Lang::Ja => "ja",
        }
    }

    // ---- tray + menu ------------------------------------------------------

    pub fn tray_tip(self) -> &'static str {
        match self {
            Lang::En => "Claude Wakeup",
            Lang::ZhTw => "Claude 喚醒",
            Lang::ZhCn => "Claude 唤醒",
            Lang::Ja => "Claude ウェイクアップ",
        }
    }

    pub fn running(self) -> &'static str {
        match self {
            Lang::En => "Running",
            Lang::ZhTw => "執行中",
            Lang::ZhCn => "运行中",
            Lang::Ja => "実行中",
        }
    }

    pub fn paused(self) -> &'static str {
        match self {
            Lang::En => "Paused",
            Lang::ZhTw => "已暫停",
            Lang::ZhCn => "已暂停",
            Lang::Ja => "一時停止",
        }
    }

    pub fn menu_enable(self) -> &'static str {
        match self {
            Lang::En => "Enabled",
            Lang::ZhTw => "啟用",
            Lang::ZhCn => "启用",
            Lang::Ja => "有効",
        }
    }

    pub fn menu_run_now(self) -> &'static str {
        match self {
            Lang::En => "Run now (wake Claude)",
            Lang::ZhTw => "立即執行（喚醒 Claude）",
            Lang::ZhCn => "立即执行（唤醒 Claude）",
            Lang::Ja => "今すぐ実行（Claude を起動）",
        }
    }

    pub fn menu_edit_config(self) -> &'static str {
        match self {
            Lang::En => "Edit config…",
            Lang::ZhTw => "編輯設定檔…",
            Lang::ZhCn => "编辑配置文件…",
            Lang::Ja => "設定ファイルを編集…",
        }
    }

    pub fn menu_open_log(self) -> &'static str {
        match self {
            Lang::En => "Open log",
            Lang::ZhTw => "開啟紀錄檔",
            Lang::ZhCn => "打开日志",
            Lang::Ja => "ログを開く",
        }
    }

    pub fn menu_reload(self) -> &'static str {
        match self {
            Lang::En => "Reload config",
            Lang::ZhTw => "重新載入設定",
            Lang::ZhCn => "重新加载配置",
            Lang::Ja => "設定を再読み込み",
        }
    }

    pub fn menu_quit(self) -> &'static str {
        match self {
            Lang::En => "Quit",
            Lang::ZhTw => "結束",
            Lang::ZhCn => "退出",
            Lang::Ja => "終了",
        }
    }

    // ---- status line ------------------------------------------------------

    /// "Every {every} min · next in ~{mins} min"
    pub fn sched_interval(self, every: u64, mins: u64) -> String {
        match self {
            Lang::En => format!("Every {every} min · next in ~{mins} min"),
            Lang::ZhTw => format!("每 {every} 分鐘 · 約 {mins} 分鐘後執行"),
            Lang::ZhCn => format!("每 {every} 分钟 · 约 {mins} 分钟后执行"),
            Lang::Ja => format!("{every} 分ごと · 約 {mins} 分後に実行"),
        }
    }

    pub fn sched_daily(self, times: &str) -> String {
        match self {
            Lang::En => format!("Daily {times}"),
            Lang::ZhTw => format!("每日 {times}"),
            Lang::ZhCn => format!("每日 {times}"),
            Lang::Ja => format!("毎日 {times}"),
        }
    }

    // ---- run results (written to the log + tooltip) -----------------------

    pub fn run_ok(self, time: &str, reply: &str) -> String {
        match self {
            Lang::En => format!("[{time}] OK — {reply}"),
            Lang::ZhTw => format!("[{time}] 成功 — {reply}"),
            Lang::ZhCn => format!("[{time}] 成功 — {reply}"),
            Lang::Ja => format!("[{time}] 成功 — {reply}"),
        }
    }

    pub fn run_failed(self, time: &str, code: Option<i32>, err: &str) -> String {
        match self {
            Lang::En => format!("[{time}] Failed (exit {code:?}) {err}"),
            Lang::ZhTw => format!("[{time}] 失敗（結束碼 {code:?}）{err}"),
            Lang::ZhCn => format!("[{time}] 失败（退出码 {code:?}）{err}"),
            Lang::Ja => format!("[{time}] 失敗（終了コード {code:?}）{err}"),
        }
    }

    pub fn run_error(self, time: &str, err: &str) -> String {
        match self {
            Lang::En => format!("[{time}] Error — {err}"),
            Lang::ZhTw => format!("[{time}] 錯誤 — {err}"),
            Lang::ZhCn => format!("[{time}] 错误 — {err}"),
            Lang::Ja => format!("[{time}] エラー — {err}"),
        }
    }

    pub fn log_created(self) -> &'static str {
        match self {
            Lang::En => "[log file created]",
            Lang::ZhTw => "[紀錄檔已建立]",
            Lang::ZhCn => "[日志文件已创建]",
            Lang::Ja => "[ログファイルを作成しました]",
        }
    }

    // ---- generated config-file comments -----------------------------------

    pub fn cfg_comments(self) -> CfgComments {
        match self {
            Lang::En => CfgComments {
                header: "ClaudeWakeup configuration. After editing, choose \"Reload config\" from the tray menu.",
                language: "UI language: en | zh-TW | zh-CN | ja",
                enabled: "Master switch (true = on / false = paused).",
                mode: "Schedule mode: interval | daily (fixed clock times)",
                interval: "interval mode: minutes between wake-ups (Claude's usage window is ~5 h = 300).",
                daily: "daily mode: comma-separated 24-hour local times.",
                command: "Wake-up command. The defaults keep token cost minimal.",
                extra: "Extra command-line arguments, space-separated (optional).",
            },
            Lang::ZhTw => CfgComments {
                header: "ClaudeWakeup 設定檔。修改後，請在工具列選單選擇「重新載入設定」。",
                language: "介面語言：en | zh-TW | zh-CN | ja",
                enabled: "總開關（true 啟用 / false 暫停）。",
                mode: "排程模式：interval（間隔）| daily（每日固定時間）",
                interval: "interval 模式：每隔幾分鐘喚醒一次（Claude 用量視窗約 5 小時 = 300）。",
                daily: "daily 模式：以逗號分隔的 24 小時制本地時間。",
                command: "喚醒指令。預設值已將 token 花費降到最低。",
                extra: "額外的命令列參數，以空白分隔（選填）。",
            },
            Lang::ZhCn => CfgComments {
                header: "ClaudeWakeup 配置文件。修改后，请在托盘菜单选择“重新加载配置”。",
                language: "界面语言：en | zh-TW | zh-CN | ja",
                enabled: "总开关（true 启用 / false 暂停）。",
                mode: "调度模式：interval（间隔）| daily（每日固定时间）",
                interval: "interval 模式：每隔几分钟唤醒一次（Claude 用量窗口约 5 小时 = 300）。",
                daily: "daily 模式：以逗号分隔的 24 小时制本地时间。",
                command: "唤醒命令。默认值已将 token 花费降到最低。",
                extra: "额外的命令行参数，以空格分隔（可选）。",
            },
            Lang::Ja => CfgComments {
                header: "ClaudeWakeup の設定ファイル。編集後、トレイメニューの「設定を再読み込み」を選択してください。",
                language: "UI 言語：en | zh-TW | zh-CN | ja",
                enabled: "メインスイッチ（true = 有効 / false = 一時停止）。",
                mode: "スケジュールモード：interval（間隔）| daily（毎日の固定時刻）",
                interval: "interval モード：何分ごとに起動するか（Claude の利用枠は約 5 時間 = 300）。",
                daily: "daily モード：カンマ区切りの 24 時間表記のローカル時刻。",
                command: "起動コマンド。既定値はトークン消費を最小限に抑えます。",
                extra: "追加のコマンドライン引数（スペース区切り、任意）。",
            },
        }
    }
}
