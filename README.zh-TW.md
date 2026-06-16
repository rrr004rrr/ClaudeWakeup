<div align="center">

# ClaudeWakeup（Claude 喚醒）

**常駐 Windows 工具列的小工具：① 依排程做廉價 ping 讓 Claude 用量視窗保持喚醒；② 用 GUI 安排「半夜任務」，由會喚醒電腦的 Windows 排程在你不在時自動執行。**

[![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows)](https://github.com/rrr004rrr/ClaudeWakeup)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-000000?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

[English](README.md) · 繁體中文

</div>

---

ClaudeWakeup 常駐在通知區（系統工具列）。除了原本的「保溫 ping」，現在還能讓你
**預先寫好任務、安排時間，在半夜自動跑 Claude**，隔天來檢查結果。

## 兩個功能

**1. 保溫 ping（省 token）** — 每隔一段時間在背景跑最精簡的指令
`claude -p "hi" --model haiku`，讓用量視窗保持有效。以 `CREATE_NO_WINDOW`
啟動，不會閃命令列視窗。

**2. 半夜任務管理** — 在工具列開啟「任務管理」視窗，新增一筆任務（名稱、啟動時間、
頻率、工作資料夾、模型、是否跳過權限、逾時、任務訊息）。按「新增」後：

- 程式把任務存到 `tasks.json`，並**註冊一個「會喚醒電腦」的 Windows 排程**，
  在你設定的時間執行 `ClaudeWakeup.exe --run-task <id>`。
- 屆時 Windows 把睡著的電腦叫醒、跑這筆任務；執行期間用
  `SetThreadExecutionState` **把電腦釘醒**（CPU 在忙並不會阻止睡眠，只有這個請求可以）。
- 完整輸出存到 `logs\task-<id>-<時間>.log`；逾時自動中止。
- 隔天你打開清單看狀態（待執行／執行中／已完成／失敗）與輸出，按「完成」那筆就移除
  （連同它的排程一起刪掉）。

## 快速開始（免編譯）

1. 下載 `ClaudeWakeup.exe`，放到任意資料夾後雙擊執行。
2. 工具列出現一個橘紅色圓點（看不到就點 `^` 展開隱藏圖示）。
3. **左鍵或右鍵點圖示**開啟選單。

設定檔（`claude-wakeup.toml`）、任務檔（`tasks.json`）與輸出（`logs\`）都會自動
產生在 `.exe` 所在資料夾。

> **前置需求：** 必須已安裝並可執行 [Claude CLI](https://docs.claude.com/en/docs/claude-code)。
> 若 `claude` 不在 `PATH`，請在設定檔把 `claude_path` 改為 `claude.exe` 的完整路徑。

## 工具列選單

| 項目                  | 功能                                              |
|-----------------------|---------------------------------------------------|
| **任務管理…**         | 開啟任務清單 + 新增/編輯任務的視窗                  |
| **保溫設定…**         | 原本的喚醒功能（定期 ping）的設定與即時測試         |
| **語言**              | 切換 English／繁體中文（視窗即時換語言）            |
| **結束**              | 移除圖示並離開                                      |

> 選單文字採中英並列；切換語言主要改變視窗內的文字。

## 任務管理視窗

- 上方是任務清單（名稱／時間／頻率／狀態）。
- 下方是表單：
  - **名稱**：留空會自動取任務訊息的第一行。
  - **時間 (HH:MM)**：24 小時制本地時間。
  - **頻率**：單次（下一個該時刻；今天過了就明天）或每日。
  - **資料夾**：任務的工作目錄（按「瀏覽…」選）。留空 = `.exe` 所在資料夾。
    **建議指向一個 git 專案**，早上用 `git diff` 檢查 Claude 做了什麼。
  - **模型**：`sonnet`／`opus`／`haiku`（半夜真任務建議 sonnet 或 opus）。
  - **跳過權限**：勾選會帶入 `--dangerously-skip-permissions`，讓 Claude 無人看管下
    自行改檔案／執行指令。**請只對你信任的資料夾啟用。**
  - **逾時 (分)**：超過就中止，避免卡死。
  - **任務訊息**：送給 Claude 的 prompt（可多行）。
- 按鈕：
  - **編輯所選**：把清單選中的任務載入表單。
  - **儲存任務**：若是在編輯就更新該筆（並重新排程）；否則新增一筆。
  - **完成（移除）**：刪掉選中的任務與它的排程。
  - **查看輸出**：用記事本開該筆的執行 log。
  - **新任務 / 清除**：清空表單、離開編輯狀態。
  - **關閉**：把視窗收回工具列。

## 保溫設定（原本的喚醒功能）

這是 ClaudeWakeup 最初的功能，**和排程任務分開**：定期跑一次廉價的
`claude -p`，讓用量視窗保持有效。從工具列「保溫設定…」開啟：

- **啟用保溫**。
- **排程方式**：
  - **間隔** — 每隔「間隔（分）」分鐘 ping 一次。只在**電腦醒著**時跑（APP 內部計時器）。
  - **每日** — 在「每日時間」列出的每個固定時刻各 ping 一次（例如
    `07:00, 12:00, 17:00, 22:00`，逗號分隔）。**每日模式會自動註冊一個「會喚醒電腦」
    的 Windows 排程**，所以即使你不在、電腦睡著了，到時間也會被叫醒來 ping。預設走每日。
- **模型**：保溫用 `haiku` 最省。
- **立即執行一次**：馬上 ping，並把結果顯示在「上次結果」（這樣你就看得到它有作用）。
- **儲存**：寫回 `claude-wakeup.toml`，並依模式自動建立／移除保溫的喚醒排程。

> 喚醒一樣只在電腦**插著電**時有效（電池模式 Windows 預設停用喚醒計時器）。

## 重要：睡眠與喚醒的前提

- 喚醒只在電腦**插著電**時有效——Windows 預設在電池模式下停用喚醒計時器。
- 請確認系統電源設定的「允許喚醒計時器」未被公司群組原則停用。
- 排程任務以「目前登入使用者」身分執行；半夜螢幕鎖定不影響 `claude -p` 執行。
- 若註冊排程失敗，視窗會提示；可嘗試以系統管理員身分執行。

## 設定檔

第一次執行會在 `.exe` 旁產生 `claude-wakeup.toml`：

```ini
# 介面語言：en | zh-TW
language = zh-TW

# `claude`（走 PATH）或 claude.exe 的完整路徑。
claude_path = claude

# 保溫 ping：定期跑一次廉價的 claude -p，讓用量視窗保持有效。
warm_enabled = true
warm_interval_minutes = 300
warm_model = haiku
warm_prompt = hi
# 保溫 ping 跑完也通知飛書（會比任務頻繁，預設關閉）。
warm_notify = false

# 飛書（Lark）機器人 webhook：任務完成（或失敗）時通知。留空 = 關閉。
feishu_hook = https://open.feishu.cn/open-apis/bot/v2/hook/xxxxxxxx
```

語言也可從工具列選單直接切換（會寫回此檔）。

**任務完成通知（飛書）**：每筆任務跑完後，會用 `curl` POST 一則文字訊息到
`feishu_hook`（成功 ✅／失敗 ❌，含任務名稱、結果、時間、輸出 log 路徑）。
你可以靠這則通知得知任務完成。不需要時把 `feishu_hook` 留空即可關閉。

**保溫通知（選用）**：預設保溫 ping **不**發通知（它比任務頻繁，會洗版）。若要保溫也通知，
在「保溫設定…」勾選「啟用推播」，或在設定檔把 `warm_notify` 設為 `true`（仍需有
`feishu_hook`）。「立即執行一次」會依「啟用推播」的勾選狀態決定要不要推播，可當作推播測試。

## 開機自動啟動

把 `ClaudeWakeup.exe` 的捷徑放到「開機啟動」資料夾即可：

```bat
install-startup.bat            :: 安裝
install-startup.bat remove     :: 移除
```

## 自行編譯

需要 [Rust 工具鏈](https://rustup.rs/)。

```bat
build.bat
:: 或
cargo build --release
```

產生：`target\release\ClaudeWakeup.exe`（單一獨立檔案；icon 由 `assets\icon.ico`
內嵌進 binary）。

## 專案結構

```
src/
├── main.rs        進入點：--run-task 分派、保溫執行緒、啟動 GUI
├── ui.rs          工具列 + 任務管理視窗（native-windows-gui）
├── task.rs        任務資料結構與 tasks.json 讀寫（serde）
├── scheduler.rs   為每筆任務註冊／移除「會喚醒電腦」的 Windows 排程
├── runner.rs      無頭執行任務（--run-task）、保溫 ping、釘醒電腦
├── config.rs      設定檔（語言 + 保溫）讀寫
├── i18n.rs        en / zh-TW 多語系
└── util.rs        時間格式、檔名清理等小工具
assets/icon.ico    工具列圖示（內嵌）
build.bat          release 編譯
install-startup.bat  建立／移除開機捷徑
```

## 授權

[MIT](LICENSE) © 2026 rrr004rrr
