//! Lightweight, dependency-free localization. For now: English + 繁體中文,
//! switchable from the tray menu. Add a language by extending the enum and the
//! matches below.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    En,
    #[default]
    ZhTw,
}

impl Lang {
    pub fn parse(s: &str) -> Lang {
        match s.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "zh-tw" | "zh-hant" | "zh" | "zh-cn" | "zh-hans" => Lang::ZhTw,
            _ => Lang::En,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::ZhTw => "zh-TW",
        }
    }

    fn pick(self, en: &'static str, zh: &'static str) -> &'static str {
        match self {
            Lang::En => en,
            Lang::ZhTw => zh,
        }
    }

    // ---- manager window ---------------------------------------------------
    pub fn win_title(self) -> &'static str {
        self.pick("ClaudeWakeup — Tasks", "ClaudeWakeup — 任務")
    }
    pub fn lbl_name(self) -> &'static str {
        self.pick("Name", "名稱")
    }
    pub fn lbl_time(self) -> &'static str {
        self.pick("Time (HH:MM)", "時間 (HH:MM)")
    }
    pub fn lbl_freq(self) -> &'static str {
        self.pick("Frequency", "頻率")
    }
    pub fn lbl_dir(self) -> &'static str {
        self.pick("Folder", "資料夾")
    }
    pub fn btn_browse(self) -> &'static str {
        self.pick("Browse…", "瀏覽…")
    }
    pub fn lbl_model(self) -> &'static str {
        self.pick("Model", "模型")
    }
    pub fn lbl_skip(self) -> &'static str {
        self.pick("Skip permissions", "跳過權限")
    }
    pub fn lbl_timeout(self) -> &'static str {
        self.pick("Timeout (min)", "逾時 (分)")
    }
    pub fn lbl_message(self) -> &'static str {
        self.pick("Task message", "任務訊息")
    }
    pub fn freq_once(self) -> &'static str {
        self.pick("Once", "單次")
    }
    pub fn freq_daily(self) -> &'static str {
        self.pick("Daily", "每日")
    }
    pub fn btn_edit(self) -> &'static str {
        self.pick("Edit selected", "編輯所選")
    }
    pub fn btn_save(self) -> &'static str {
        self.pick("Save task", "儲存任務")
    }
    pub fn btn_new(self) -> &'static str {
        self.pick("New / clear", "新任務 / 清除")
    }
    pub fn btn_complete(self) -> &'static str {
        self.pick("Mark done (remove)", "完成（移除）")
    }
    pub fn btn_view_output(self) -> &'static str {
        self.pick("View output", "查看輸出")
    }
    pub fn btn_close(self) -> &'static str {
        self.pick("Close", "關閉")
    }

    // ---- list columns + status -------------------------------------------
    pub fn col_name(self) -> &'static str {
        self.pick("Name", "名稱")
    }
    pub fn col_time(self) -> &'static str {
        self.pick("Time", "時間")
    }
    pub fn col_freq(self) -> &'static str {
        self.pick("Freq", "頻率")
    }
    pub fn col_status(self) -> &'static str {
        self.pick("Status", "狀態")
    }
    pub fn st_pending(self) -> &'static str {
        self.pick("Pending", "待執行")
    }
    pub fn st_running(self) -> &'static str {
        self.pick("Running", "執行中")
    }
    pub fn st_done(self) -> &'static str {
        self.pick("Done", "已完成")
    }
    pub fn st_failed(self) -> &'static str {
        self.pick("Failed", "失敗")
    }

    // ---- messages ---------------------------------------------------------
    pub fn title_info(self) -> &'static str {
        self.pick("ClaudeWakeup", "ClaudeWakeup")
    }
    pub fn msg_need_prompt(self) -> &'static str {
        self.pick(
            "Please enter a task message and a time (HH:MM).",
            "請輸入任務訊息與時間 (HH:MM)。",
        )
    }
    pub fn msg_added(self) -> &'static str {
        self.pick(
            "Task added and scheduled (the PC will wake to run it).",
            "任務已新增並排程（屆時會喚醒電腦執行）。",
        )
    }
    pub fn msg_select_first(self) -> &'static str {
        self.pick("Select a task in the list first.", "請先在清單中選一筆任務。")
    }
    pub fn msg_sched_failed(self) -> &'static str {
        self.pick(
            "Task saved, but scheduling failed. Try running as Administrator.",
            "任務已存，但排程註冊失敗。可嘗試以系統管理員身分執行。",
        )
    }
    pub fn msg_no_output(self) -> &'static str {
        self.pick("This task has no output yet.", "這筆任務還沒有輸出。")
    }
    pub fn msg_saved(self) -> &'static str {
        self.pick("Saved.", "已儲存。")
    }
    pub fn msg_updated(self) -> &'static str {
        self.pick("Task updated and rescheduled.", "任務已更新並重新排程。")
    }

    // ---- keep-warm window -------------------------------------------------
    pub fn warm_title(self) -> &'static str {
        self.pick("ClaudeWakeup — Keep-warm", "ClaudeWakeup — 保溫")
    }
    pub fn warm_enabled_lbl(self) -> &'static str {
        self.pick("Enable keep-warm", "啟用保溫")
    }
    pub fn warm_notify_lbl(self) -> &'static str {
        self.pick("Enable push", "啟用推播")
    }
    pub fn warm_mode_lbl(self) -> &'static str {
        self.pick("Schedule", "排程方式")
    }
    pub fn warm_mode_interval(self) -> &'static str {
        self.pick("Interval", "間隔")
    }
    pub fn warm_mode_daily(self) -> &'static str {
        self.pick("Daily", "每日")
    }
    pub fn warm_interval_lbl(self) -> &'static str {
        self.pick("Interval (min)", "間隔（分）")
    }
    pub fn warm_daily_lbl(self) -> &'static str {
        self.pick("Daily times", "每日時間")
    }
    pub fn warm_model_lbl(self) -> &'static str {
        self.pick("Model", "模型")
    }
    pub fn warm_run(self) -> &'static str {
        self.pick("Ping now", "立即執行一次")
    }
    pub fn btn_apply(self) -> &'static str {
        self.pick("Save", "儲存")
    }
    pub fn warm_last(self) -> &'static str {
        self.pick("Last result:", "上次結果：")
    }
    pub fn warm_running(self) -> &'static str {
        self.pick("(pinging…)", "（執行中…）")
    }
    pub fn warm_never(self) -> &'static str {
        self.pick("(not run yet)", "（尚未執行）")
    }

    // ---- notifications (Feishu) window ------------------------------------
    pub fn notif_title(self) -> &'static str {
        self.pick("ClaudeWakeup — Push", "ClaudeWakeup — 推播")
    }
    pub fn notif_intro(self) -> &'static str {
        self.pick(
            "Feishu/Lark bot webhook URLs — one per line. Every one is notified \
             when a task finishes, so you can push to multiple users/groups. \
             Leave empty to turn notifications off.",
            "飛書／Lark 機器人 Webhook 網址，每行一個。任務完成時會通知每一個，\
             因此可同時推播給多位用戶／群組。留空即關閉通知。",
        )
    }
    pub fn notif_hooks_lbl(self) -> &'static str {
        self.pick("Webhook URLs", "Webhook 網址")
    }
    pub fn notif_test(self) -> &'static str {
        self.pick("Send test", "傳送測試")
    }
    pub fn notif_test_sent(self) -> &'static str {
        self.pick(
            "Test message sent to all listed webhooks. Check Feishu.",
            "已向所有列出的 Webhook 傳送測試訊息，請至飛書確認。",
        )
    }
    pub fn notif_test_empty(self) -> &'static str {
        self.pick(
            "Add at least one webhook URL first.",
            "請先填入至少一個 Webhook 網址。",
        )
    }
    pub fn notif_test_text(self) -> &'static str {
        self.pick(
            "🔔 ClaudeWakeup test — your Feishu push is working.",
            "🔔 ClaudeWakeup 測試 — 您的飛書推播已正常運作。",
        )
    }
}
