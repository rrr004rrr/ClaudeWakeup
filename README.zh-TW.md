<div align="center">

# ClaudeWakeup（Claude 喚醒）

**跨平台的工具列／選單列小工具：① 依排程做廉價 ping 讓 Claude 用量視窗保持喚醒；② 用 GUI 安排「半夜任務」，由會喚醒電腦的系統排程在你不在時自動執行。支援 Windows 與 macOS。**

[![Version](https://img.shields.io/github/v/release/rrr004rrr/ClaudeWakeup?label=version&color=blue)](https://github.com/rrr004rrr/ClaudeWakeup/releases/latest)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-555?logo=apple)](https://github.com/rrr004rrr/ClaudeWakeup)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-000000?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

[English](README.md) · 繁體中文

</div>

---

ClaudeWakeup 常駐在系統工具列（Windows）或選單列（macOS）。除了原本的「保溫
ping」，現在還能讓你**預先寫好任務、安排時間，在半夜自動跑 Claude**，隔天來檢查
結果。GUI 是一個 egui 視窗，內含**三個分頁**（任務／保溫／推播），兩個平台共用同一份。

## 兩個功能

**1. 保溫 ping（省 token）** — 每隔一段時間在背景跑最精簡的指令
`claude -p "hi" --model haiku`，讓用量視窗保持有效。不會閃命令列視窗（Windows
隱藏視窗，macOS 本就無視窗）。

**2. 半夜任務管理** — 在工具列／選單開啟「任務」分頁，新增一筆任務（名稱、啟動時間、
頻率、工作資料夾、模型、是否跳過權限、逾時、任務訊息）。按「儲存」後：

- 程式把任務存到 `tasks.json`，並**註冊一個「會喚醒電腦」的系統排程**，在你設定的
  時間執行 `ClaudeWakeup --run-task <id>`：
  - **Windows** — 帶 `-WakeToRun` 的工作排程器（Task Scheduler）工作。
  - **macOS** — launchd LaunchAgent（`StartCalendarInterval`）加上 `pmset`
    硬體喚醒（見 [macOS 喚醒說明](#macos-睡眠與喚醒說明)）。
- 屆時電腦被叫醒、跑這筆任務；執行期間**把電腦釘醒**（Windows 用
  `SetThreadExecutionState`，macOS 用 `caffeinate`——CPU 在忙並不會阻止睡眠）。
- 完整輸出存到 `logs/task-<id>-<時間>.log`；逾時自動中止。
- 「單次」任務跑完後會自行移除排程；「每日」則重複。
- 隔天打開清單看狀態（待執行／執行中／已完成／失敗）與輸出，按「完成」即移除該筆
  （連同排程一起刪）。

## 快速開始

> **前置需求：** 必須已安裝並可執行 [Claude CLI](https://docs.claude.com/en/docs/claude-code)。
> 若 `claude` 不在 `PATH`，請在設定檔把 `claude_path` 改為完整路徑。

### Windows

1. 下載或編譯出 `ClaudeWakeup.exe`，放到任意資料夾後雙擊執行。
2. 工具列出現一個橘紅色圓點（看不到就點 `^` 展開隱藏圖示）。
3. **左鍵或右鍵點圖示**開啟選單。

設定檔（`claude-wakeup.toml`）、任務檔（`tasks.json`）與輸出（`logs/`）會產生在
`.exe` 所在資料夾。

### macOS

1. 編譯 app bundle：`./build.sh` → 產生 `dist/ClaudeWakeup.app`。
2. **移到 `/Applications`（建議）**：`mv dist/ClaudeWakeup.app /Applications/`。
   若從受 iCloud 同步的 `~/Documents`／`~/Desktop` 執行，macOS 每次啟動都會要求
   iCloud 存取權限；`/Applications` 不受同步。
3. 啟動：`open /Applications/ClaudeWakeup.app`，**選單列**出現圖示，點它開啟選單。

設定／任務／輸出存放在 `~/.claudewakeup/`。（home 下的隱藏資料夾 —— 選它而非
`~/Library/Application Support`，是為了避免未簽章 app 每次啟動都跳「取用其他 App
資料」的授權框。）

## 工具列／選單列選單

| 項目             | 功能                                              |
|------------------|---------------------------------------------------|
| **任務管理**     | 開啟視窗並切到「任務」分頁                          |
| **保溫設定**     | 開啟視窗並切到「保溫」分頁                          |
| **推播設定**     | 開啟視窗並切到「推播」分頁（飛書 webhook）          |
| **語言**         | 切換 English／繁體中文（視窗即時換語言）            |
| **結束**         | 移除圖示並離開                                      |

> 選單文字採中英並列；切換語言主要改變視窗內文字。
> 關閉視窗會把它收回工具列／選單列（不會結束程式）。

## 任務分頁

- 上方是任務清單（名稱／時間／頻率／狀態）——點一列即選取。
- 下方是表單：
  - **名稱**：留空會自動取任務訊息的第一行。
  - **時間 (HH:MM)**：24 小時制本地時間。
  - **頻率**：單次（下一個該時刻；今天過了就明天）或每日。
  - **資料夾**：任務的工作目錄（按「瀏覽…」選）。留空 = 資料目錄。
    **建議指向一個 git 專案**，早上用 `git diff` 檢查 Claude 做了什麼。
  - **模型**：`sonnet`／`opus`／`haiku`（半夜真任務建議 sonnet 或 opus）。
  - **跳過權限**：勾選會帶入 `--dangerously-skip-permissions`，讓 Claude 無人看管下
    自行改檔案／執行指令。**請只對你信任的資料夾啟用。**
  - **逾時 (分)**：超過就中止，避免卡死。
  - **任務訊息**：送給 Claude 的 prompt（可多行）。
- 按鈕：
  - **編輯所選**：把清單選中的任務載入表單。
  - **儲存任務**：若在編輯就更新該筆（並重新排程）；否則新增一筆。
  - **完成（移除）**：刪掉選中的任務與它的排程。
  - **查看輸出**：用系統預設程式開該筆的執行 log。
  - **新任務 / 清除**：清空表單、離開編輯狀態。

## 保溫分頁（原本的喚醒功能）

這是 ClaudeWakeup 最初的功能，**和排程任務分開**：定期跑一次廉價的
`claude -p`，讓用量視窗保持有效。

- **啟用保溫**。
- **排程方式**：
  - **間隔** — 每隔「間隔（分）」分鐘 ping 一次。只在**電腦醒著**時跑（APP 內部計時器）。
  - **每日** — 在「每日時間」列出的每個固定時刻各 ping 一次（例如
    `07:00, 12:00, 17:00, 22:00`，逗號分隔）。**每日模式會註冊一個「會喚醒電腦」的
    排程**，即使你不在也會被叫醒來 ping。預設走每日。
- **模型**：保溫用 `haiku` 最省。
- **立即執行一次**：馬上 ping，結果顯示在「上次結果」。
- **儲存**：寫回 `claude-wakeup.toml`，並依模式自動建立／移除保溫的喚醒排程。

## 推播分頁（飛書／Lark）

任務完成時，ClaudeWakeup 可推播一則訊息到一個或多個
[飛書／Lark 自訂機器人 webhook](https://www.feishu.cn/hc/zh-TW/articles/360024984973)
（保溫 ping 也可選擇性推播）。在這裡管理收件對象，不必手動編輯設定檔：

- **Webhook 網址** — 每行貼一個機器人 webhook 網址。**列出的每個網址都會收到通知**，
  因此可同時推播給多位用戶／群組。
- **傳送測試** — 立即向所有列出的 webhook 送出測試訊息（用目前輸入框的內容，**即使還沒
  儲存**也能測），確認每個機器人都接好了。
- **儲存** — 把網址寫回 `claude-wakeup.toml`（每個收件對象一行 `feishu_hook = <網址>`）。
  清空輸入框即關閉推播。

## macOS 睡眠與喚醒說明

launchd 會在排程時刻、**Mac 醒著時**自動執行任務（不需額外設定）。若還想叫醒
**睡眠中**的 Mac，可用 `pmset schedule wake` 為每個排程時刻安排接下來幾天的喚醒。

- 喚醒是**選擇性**的：在「保溫」分頁按 **「設定 Mac 喚醒（需管理員）…」**。它涵蓋所有
  排程時間（任務＋保溫）。`pmset` 需要 root，所以這是**唯一**會跳管理員密碼的動作
  （`osascript … with administrator privileges`）——一般的儲存完全不會跳。
- 它會預先排好幾天份。長時間無人看管時請偶爾再按一次。重新安排前會用
  `pmset schedule cancelall` 清掉先前的喚醒事件（也會一併清掉你手動設的）。
- 筆電通常需要插著電才能從睡眠喚醒。

**背景活動與程式簽章。** 保溫／每日任務是 LaunchAgent，所以 macOS 會把 ClaudeWakeup
列在「系統設定 → 一般 → 登入項目 → 允許在背景執行」，並在**首次註冊**某個工作時跳一次
「背景活動」通知（不是每次啟動都跳）。由於這個 app 沒有用 Apple Developer ID 簽章，會
顯示「來自未識別開發者」——這是自行編譯工具的正常現象，只要開關開著就能運作。請只在
固定路徑（例如 `/Applications`）保留**一份** app，背景清單才會維持單一項目。

## Windows 睡眠與喚醒說明

- 喚醒只在電腦**插著電**時有效——Windows 預設在電池模式下停用喚醒計時器。
- 請確認電源設定的「允許喚醒計時器」未被公司群組原則停用。
- 排程以「目前登入使用者」身分執行；螢幕鎖定不影響執行。
- 若註冊排程失敗，狀態列會提示；可嘗試以系統管理員身分執行。

## 設定檔

第一次執行會在資料目錄產生 `claude-wakeup.toml`：

```ini
# 介面語言：en | zh-TW
language = zh-TW

# `claude`（走 PATH）或 claude 執行檔的完整路徑。
claude_path = claude

# 保溫 ping：定期跑一次廉價的 claude -p，讓用量視窗保持有效。
warm_enabled = true
warm_mode = daily
warm_interval_minutes = 300
warm_daily_times = 07:00, 12:00, 17:00, 22:00
warm_model = haiku
warm_prompt = hi
# 保溫 ping 跑完也通知飛書（會比任務頻繁，預設關閉）。
warm_notify = false

# 飛書（Lark）機器人 webhook：任務完成（或失敗）時，列出的每個都會收到通知。
# 每個收件對象一行 feishu_hook = <網址>。沒有任何一行／留空 = 關閉。
feishu_hook = https://open.feishu.cn/open-apis/bot/v2/hook/xxxxxxxx
feishu_hook = https://open.feishu.cn/open-apis/bot/v2/hook/yyyyyyyy
```

語言也可從選單直接切換（會寫回此檔）。

**任務完成通知（飛書）**：每筆任務跑完後，用 `curl` POST 一則文字訊息到**所有**設定的
`feishu_hook`（成功 ✅／失敗 ❌，含任務名稱、結果、時間、輸出 log 路徑）。可在「推播」分頁
管理收件清單（每行一個網址），或在此檔加入多行 `feishu_hook = <網址>`；全部清空即關閉。
（`curl` 在 Windows 10+ 與 macOS 都內建。）

**保溫通知（選用）**：預設保溫 ping **不**發通知。若要保溫也通知，在「保溫」分頁勾選
「啟用推播」，或在設定檔把 `warm_notify` 設為 `true`（仍需至少有一個 `feishu_hook`）。

## 開機自動啟動

**Windows** — 把捷徑放到「開機啟動」資料夾：

```bat
install-startup.bat            :: 安裝
install-startup.bat remove     :: 移除
```

**macOS** — 安裝一個會在登入時開啟 app 的 LaunchAgent：

```bash
./install-startup.sh           # 安裝
./install-startup.sh remove    # 移除
```

## 自行編譯

需要 [Rust 工具鏈](https://rustup.rs/)。

**Windows**

```bat
build.bat
:: 或
cargo build --release
```

產生：`target\release\ClaudeWakeup.exe`（單一獨立檔案；icon 已內嵌）。

**macOS**

```bash
./build.sh
# 或只要執行檔：
cargo build --release
```

`build.sh` 也會組出設定了 `LSUIElement` 的 `dist/ClaudeWakeup.app`（純選單列 app，
不顯示 Dock 圖示）。

## 專案結構

```
src/
├── main.rs            進入點：--run-task / --keep-warm 分派、資料目錄、啟動 GUI
├── app.rs             跨平台 GUI：工具列 + 視窗（任務／保溫分頁，egui）
├── platform/
│   ├── mod.rs         平台抽象層（排程、釘醒、開檔、喚醒）
│   ├── windows.rs     工作排程器 + SetThreadExecutionState
│   └── macos.rs       launchd + pmset 喚醒 + caffeinate
├── task.rs            任務資料結構與 tasks.json 讀寫（serde）
├── runner.rs          無頭執行任務（--run-task）、保溫 ping
├── config.rs          設定檔（語言 + 保溫）讀寫
├── i18n.rs            en / zh-TW 多語系
└── util.rs            時間格式（chrono）、檔名清理
assets/icon.ico        工具列圖示（內嵌；執行時載入為 RGBA）
build.bat / build.sh   release 編譯（Windows／macOS）
install-startup.*      建立／移除開機項目
```

## 授權

[MIT](LICENSE) © 2026 rrr004rrr
