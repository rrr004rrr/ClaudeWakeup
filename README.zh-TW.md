<div align="center">

# ClaudeWakeup（Claude 喚醒）

**一個常駐 Windows 工具列的超小型程式，依排程自動呼叫 Claude CLI，讓你的用量視窗持續保持喚醒。**

[![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows)](https://github.com/rrr004rrr/ClaudeWakeup)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-000000?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/rrr004rrr/ClaudeWakeup?include_prereleases)](https://github.com/rrr004rrr/ClaudeWakeup/releases)

[English](README.md) · 繁體中文

</div>

---

ClaudeWakeup 是一個約 250 KB 的單檔 Windows 小工具，常駐在通知區（系統工具列）。
它會依你設定的排程，在背景靜默執行 Claude CLI，讓你的 token／用量視窗保持有效——
不需要終端機、不會跳出視窗。

## 為什麼很省 token

每次喚醒執行的指令都是最精簡的：

```text
claude -p "hi" --model haiku
```

- `-p` / `--print` 是 Claude 的**無互動、單次回合**模式。
- `haiku` 是最便宜的模型。
- 程式以 `CREATE_NO_WINDOW` 啟動，不會閃出黑色命令列視窗。

因此每次喚醒花費的 token 都是最低限度。

## 功能特色

- 🪶 **輕巧獨立**——單一約 250 KB 的 `.exe`，免安裝、無執行階段相依。
- 🕒 **兩種排程模式**——固定間隔（每 N 分鐘）或每日固定時間。
- 🌐 **多語系介面**——English（預設）、繁體中文、简体中文、日本語。見[語言設定](#語言設定)。
- 🔕 **背景靜默執行**——不會出現命令列視窗。
- 📋 **即時提示與選單**——一眼掌握狀態、下次執行時間與上次結果。
- 📝 **純文字設定與紀錄**——用記事本編輯，從選單重新載入。

## 快速開始（免編譯）

1. 到 [**Releases**](https://github.com/rrr004rrr/ClaudeWakeup/releases) 頁面下載 `ClaudeWakeup.exe`。
2. 放到任意位置（桌面、工具資料夾……）後雙擊執行。
3. 工具列會出現一個橘紅色圓點。看不到的話，點工具列的 `^` 展開隱藏圖示。
4. **左鍵或右鍵點圖示**即可開啟選單。
5. 預設每 5 小時喚醒一次。想立即測試請點「**立即執行**」。

設定檔（`claude-wakeup.toml`）與紀錄檔（`claude-wakeup.log`）會自動產生在 `.exe` 所在資料夾。

> **前置需求：** 必須已安裝並可執行 [Claude CLI](https://docs.claude.com/en/docs/claude-code)。
> 若 `claude` 不在 `PATH`，請將設定檔的 `claude_path` 改為 `claude.exe` 的完整路徑。

## 工具列選單

| 項目             | 功能                                       |
|------------------|--------------------------------------------|
| （狀態）         | 執行中／已暫停、排程、下次執行時間（灰色）   |
| （上次結果）     | 最近一次喚醒的結果（灰色）                   |
| **啟用**         | 暫停／恢復排程（打勾＝執行中）               |
| **立即執行**     | 馬上喚醒一次                                 |
| **編輯設定檔…**  | 用記事本開啟 `claude-wakeup.toml`           |
| **開啟紀錄檔**   | 開啟 `claude-wakeup.log`                     |
| **重新載入設定** | 改完設定檔後重新讀取                         |
| **結束**         | 移除工具列圖示並離開                         |

把滑鼠移到圖示上，會以提示文字顯示目前狀態。

## 設定說明

第一次執行時，會在 `.exe` 旁自動產生 `claude-wakeup.toml`。從「**編輯設定檔…**」修改，
再選「**重新載入設定**」即可生效。

```ini
# 介面語言：en | zh-TW | zh-CN | ja
language = zh-TW

# 總開關（true 啟用 / false 暫停）。
enabled = true

# 排程模式：interval（間隔）| daily（每日固定時間）
mode = interval

# interval 模式：每隔幾分鐘喚醒一次（Claude 用量視窗約 5 小時 = 300）。
interval_minutes = 300

# daily 模式：以逗號分隔的 24 小時制本地時間。
daily_times = 09:00, 14:00, 19:00

# 喚醒指令。預設值已將 token 花費降到最低。
claude_path = claude
model = haiku
prompt = hi

# 額外的命令列參數，以空白分隔（選填）。
extra_args =
```

| 設定鍵             | 說明                                                          |
|--------------------|---------------------------------------------------------------|
| `language`         | 介面語言：`en`（預設）、`zh-TW`、`zh-CN`、`ja`。              |
| `enabled`          | 總開關（也可從選單切換）。                                     |
| `mode`             | `interval` 或 `daily`。                                        |
| `interval_minutes` | `interval` 模式下，每隔幾分鐘喚醒。Claude 視窗約 5 小時，故 `300`。 |
| `daily_times`      | `daily` 模式下，以逗號分隔的 24 小時制本地時間。              |
| `claude_path`      | `claude`（使用 `PATH`）或 `claude.exe` 的完整路徑。           |
| `model`            | 傳給 `--model` 的模型別名。`haiku` 最便宜。                   |
| `prompt`           | 每次喚醒送出的提示，刻意保持精簡。                             |
| `extra_args`       | 附加的命令列參數（進階，通常留空）。                           |

- **`interval` 模式**：每隔 `interval_minutes` 分鐘執行一次。第一次會在啟動「之後」
  一個間隔才執行；想馬上跑請用「立即執行」。
- **`daily` 模式**：在每個 `HH:MM`（本地時間）各執行一次。

## 語言設定

介面預設為**英文**。要切換語言，請在設定檔修改 `language`，再選「**重新載入設定**」（或重新啟動）：

| 值      | 語言              |
|---------|-------------------|
| `en`    | English（預設）   |
| `zh-TW` | 繁體中文           |
| `zh-CN` | 简体中文           |
| `ja`    | 日本語             |

新增語言只需在 [`src/i18n.rs`](src/i18n.rs) 做一個小而獨立的修改——新增一個列舉變體、
每個字串各加一行；沒有外部資源檔，全部都包在單一執行檔內。

## 開機自動啟動

把 `ClaudeWakeup.exe` 的捷徑放到「開機啟動」資料夾即可。最簡單的方式：

```bat
install-startup.bat            :: 安裝
install-startup.bat remove     :: 移除
```

或手動操作：

1. 按 <kbd>Win</kbd>+<kbd>R</kbd>，輸入 `shell:startup`，按 Enter。
2. 把 `ClaudeWakeup.exe` 的捷徑放進開啟的資料夾。

## 自行編譯

需要 [Rust 工具鏈](https://rustup.rs/)（`cargo`）。

```bat
build.bat
:: 或
cargo build --release
```

產生：`target\release\ClaudeWakeup.exe`（單一獨立檔案）。release 設定已針對檔案大小最佳化
（`opt-level = "z"`、LTO、strip、panic = abort）。

## 專案結構

```
src/
├── main.rs        Win32 工具列程式：視窗程序、排程、設定讀寫、圖示
└── i18n.rs        無相依的多語系（en / zh-TW / zh-CN / ja）
build.bat          release 編譯輔助腳本
install-startup.bat  建立／移除開機捷徑
Cargo.toml         crate 設定（針對體積最佳化的 release profile）
```

## 授權

[MIT](LICENSE) © 2026 rrr004rrr
