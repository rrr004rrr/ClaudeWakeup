//! The tray app + two windows, built on native-windows-gui:
//!  - Task manager: create / edit / remove overnight tasks (each backed by a
//!    wake-the-PC Task Scheduler job).
//!  - Keep-warm: the original wakeup feature — a periodic cheap `claude -p` ping,
//!    with visible last-result feedback. Distinct from scheduled tasks.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use native_windows_derive as nwd;
use native_windows_gui as nwg;
use nwd::NwgUi;
use nwg::NativeUi;

use crate::config::Config;
use crate::i18n::Lang;
use crate::runner;
use crate::scheduler;
use crate::task::{load_tasks, save_tasks, Status, Task};
use crate::util::{local_time_string, timestamp_compact};

const ICON: &[u8] = include_bytes!("../assets/icon.ico");
static COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Default, NwgUi)]
pub struct App {
    dir: RefCell<PathBuf>,
    exe: RefCell<PathBuf>,
    lang: Cell<Lang>,
    cfg: RefCell<Config>,
    editing_id: RefCell<Option<String>>,

    #[nwg_resource(source_bin: Some(ICON))]
    icon: nwg::Icon,

    #[nwg_resource(title: "Select folder", action: nwg::FileDialogAction::OpenDirectory)]
    dir_dialog: nwg::FileDialog,

    // The window must be declared before the popup menu that uses it as parent
    // (the derive builds controls in declaration order).
    #[nwg_control(size: (720, 652), center: true, title: "ClaudeWakeup", flags: "WINDOW|MINIMIZE_BOX", icon: Some(&data.icon))]
    #[nwg_events(OnWindowClose: [App::on_close(SELF, EVT_DATA)])]
    window: nwg::Window,

    // ---- tray + menu ----
    #[nwg_control(icon: Some(&data.icon), tip: Some("Claude Wakeup"))]
    #[nwg_events(MousePressLeftUp: [App::show_menu], OnContextMenu: [App::show_menu])]
    tray: nwg::TrayNotification,

    #[nwg_control(parent: window, popup: true)]
    tray_menu: nwg::Menu,
    #[nwg_control(parent: tray_menu, text: "任務管理 / Task manager")]
    #[nwg_events(OnMenuItemSelected: [App::open_manager])]
    mi_open: nwg::MenuItem,
    #[nwg_control(parent: tray_menu, text: "保溫設定 / Keep-warm")]
    #[nwg_events(OnMenuItemSelected: [App::open_warm])]
    mi_warm: nwg::MenuItem,
    #[nwg_control(parent: tray_menu, text: "推播設定 / Push")]
    #[nwg_events(OnMenuItemSelected: [App::open_notif])]
    mi_notif: nwg::MenuItem,
    #[nwg_control(parent: tray_menu)]
    sep1: nwg::MenuSeparator,
    #[nwg_control(parent: tray_menu, text: "語言 / Language")]
    lang_menu: nwg::Menu,
    #[nwg_control(parent: lang_menu, text: "English")]
    #[nwg_events(OnMenuItemSelected: [App::set_lang_en])]
    mi_lang_en: nwg::MenuItem,
    #[nwg_control(parent: lang_menu, text: "繁體中文")]
    #[nwg_events(OnMenuItemSelected: [App::set_lang_zh])]
    mi_lang_zh: nwg::MenuItem,
    #[nwg_control(parent: tray_menu)]
    sep2: nwg::MenuSeparator,
    #[nwg_control(parent: tray_menu, text: "結束 / Quit")]
    #[nwg_events(OnMenuItemSelected: [App::exit])]
    mi_quit: nwg::MenuItem,

    // ---- task manager window contents ----
    #[nwg_control(parent: window, position: (10, 10), size: (698, 240),
        list_style: nwg::ListViewStyle::Detailed,
        ex_flags: nwg::ListViewExFlags::FULL_ROW_SELECT | nwg::ListViewExFlags::GRID)]
    list: nwg::ListView,

    #[nwg_control(parent: window, text: "Name", position: (10, 262), size: (72, 22))]
    lbl_name: nwg::Label,
    #[nwg_control(parent: window, position: (88, 260), size: (620, 24))]
    in_name: nwg::TextInput,

    #[nwg_control(parent: window, text: "Time", position: (10, 292), size: (96, 22))]
    lbl_time: nwg::Label,
    #[nwg_control(parent: window, text: "03:00", position: (110, 290), size: (80, 24))]
    in_time: nwg::TextInput,

    #[nwg_control(parent: window, text: "Freq", position: (210, 292), size: (60, 22))]
    lbl_freq: nwg::Label,
    #[nwg_control(parent: window, position: (274, 290), size: (150, 120))]
    cmb_freq: nwg::ComboBox<String>,

    #[nwg_control(parent: window, text: "Folder", position: (10, 322), size: (72, 22))]
    lbl_dir: nwg::Label,
    #[nwg_control(parent: window, position: (88, 320), size: (520, 24))]
    in_dir: nwg::TextInput,
    #[nwg_control(parent: window, text: "Browse…", position: (614, 319), size: (94, 26))]
    #[nwg_events(OnButtonClick: [App::browse])]
    btn_browse: nwg::Button,

    #[nwg_control(parent: window, text: "Model", position: (10, 352), size: (72, 22))]
    lbl_model: nwg::Label,
    #[nwg_control(parent: window, position: (88, 350), size: (140, 120))]
    cmb_model: nwg::ComboBox<String>,

    #[nwg_control(parent: window, text: "Skip permissions", position: (242, 351), size: (160, 24))]
    chk_skip: nwg::CheckBox,

    #[nwg_control(parent: window, text: "Timeout", position: (412, 352), size: (80, 22))]
    lbl_timeout: nwg::Label,
    #[nwg_control(parent: window, text: "240", position: (496, 350), size: (90, 24))]
    in_timeout: nwg::TextInput,

    #[nwg_control(parent: window, text: "Task message", position: (10, 382), size: (300, 22))]
    lbl_msg: nwg::Label,
    #[nwg_control(parent: window, position: (10, 406), size: (698, 150), flags: "VISIBLE|VSCROLL|AUTOVSCROLL")]
    in_prompt: nwg::TextBox,

    #[nwg_control(parent: window, text: "Edit", position: (10, 568), size: (108, 34))]
    #[nwg_events(OnButtonClick: [App::edit_selected])]
    btn_edit: nwg::Button,
    #[nwg_control(parent: window, text: "Save", position: (126, 568), size: (108, 34))]
    #[nwg_events(OnButtonClick: [App::save_task])]
    btn_save: nwg::Button,
    #[nwg_control(parent: window, text: "Done", position: (242, 568), size: (124, 34))]
    #[nwg_events(OnButtonClick: [App::complete_task])]
    btn_complete: nwg::Button,
    #[nwg_control(parent: window, text: "Output", position: (374, 568), size: (108, 34))]
    #[nwg_events(OnButtonClick: [App::view_output])]
    btn_view: nwg::Button,
    #[nwg_control(parent: window, text: "New", position: (490, 568), size: (132, 34))]
    #[nwg_events(OnButtonClick: [App::clear_form])]
    btn_new: nwg::Button,
    #[nwg_control(parent: window, text: "Close", position: (630, 568), size: (78, 34))]
    #[nwg_events(OnButtonClick: [App::hide])]
    btn_close: nwg::Button,

    // ---- keep-warm window ----
    #[nwg_control(size: (440, 332), center: true, title: "Keep-warm", flags: "WINDOW", icon: Some(&data.icon))]
    #[nwg_events(OnWindowClose: [App::warm_on_close(SELF, EVT_DATA)])]
    warm_window: nwg::Window,

    #[nwg_control(parent: warm_window, text: "Enable", position: (12, 12), size: (200, 24))]
    wchk_enabled: nwg::CheckBox,
    #[nwg_control(parent: warm_window, text: "Notify", position: (220, 12), size: (210, 24))]
    wchk_notify: nwg::CheckBox,
    #[nwg_control(parent: warm_window, text: "Schedule", position: (12, 44), size: (120, 22))]
    wlbl_mode: nwg::Label,
    #[nwg_control(parent: warm_window, position: (140, 42), size: (150, 120))]
    wcmb_mode: nwg::ComboBox<String>,
    #[nwg_control(parent: warm_window, text: "Interval", position: (12, 76), size: (120, 22))]
    wlbl_interval: nwg::Label,
    #[nwg_control(parent: warm_window, text: "300", position: (140, 74), size: (90, 24))]
    win_interval: nwg::TextInput,
    #[nwg_control(parent: warm_window, text: "Daily times", position: (12, 108), size: (120, 22))]
    wlbl_daily: nwg::Label,
    #[nwg_control(parent: warm_window, text: "", position: (140, 106), size: (288, 24))]
    win_daily: nwg::TextInput,
    #[nwg_control(parent: warm_window, text: "Model", position: (12, 140), size: (120, 22))]
    wlbl_model: nwg::Label,
    #[nwg_control(parent: warm_window, position: (140, 138), size: (160, 120))]
    wcmb_model: nwg::ComboBox<String>,
    #[nwg_control(parent: warm_window, text: "Last result:", position: (12, 174), size: (416, 22))]
    wlbl_last: nwg::Label,
    #[nwg_control(parent: warm_window, text: "", position: (12, 196), size: (416, 44))]
    wstatus: nwg::Label,
    #[nwg_control(parent: warm_window, text: "Ping now", position: (12, 252), size: (140, 32))]
    #[nwg_events(OnButtonClick: [App::warm_run_now])]
    wbtn_run: nwg::Button,
    #[nwg_control(parent: warm_window, text: "Save", position: (162, 252), size: (120, 32))]
    #[nwg_events(OnButtonClick: [App::warm_save])]
    wbtn_save: nwg::Button,
    #[nwg_control(parent: warm_window, text: "Close", position: (320, 252), size: (108, 32))]
    #[nwg_events(OnButtonClick: [App::warm_hide])]
    wbtn_close: nwg::Button,

    // ---- notifications (Feishu push) window ----
    #[nwg_control(size: (540, 372), center: true, title: "Push", flags: "WINDOW", icon: Some(&data.icon))]
    #[nwg_events(OnWindowClose: [App::notif_on_close(SELF, EVT_DATA)])]
    notif_window: nwg::Window,

    #[nwg_control(parent: notif_window, text: "", position: (12, 12), size: (516, 76))]
    nlbl_intro: nwg::Label,
    #[nwg_control(parent: notif_window, text: "Webhook URLs", position: (12, 96), size: (516, 22))]
    nlbl_hooks: nwg::Label,
    #[nwg_control(parent: notif_window, position: (12, 120), size: (516, 168), flags: "VISIBLE|VSCROLL|AUTOVSCROLL")]
    nin_hooks: nwg::TextBox,
    #[nwg_control(parent: notif_window, text: "Send test", position: (12, 300), size: (140, 34))]
    #[nwg_events(OnButtonClick: [App::notif_test])]
    nbtn_test: nwg::Button,
    #[nwg_control(parent: notif_window, text: "Save", position: (300, 300), size: (108, 34))]
    #[nwg_events(OnButtonClick: [App::notif_save])]
    nbtn_save: nwg::Button,
    #[nwg_control(parent: notif_window, text: "Close", position: (420, 300), size: (108, 34))]
    #[nwg_events(OnButtonClick: [App::notif_hide])]
    nbtn_close: nwg::Button,
}

impl App {
    fn lang(&self) -> Lang {
        self.lang.get()
    }
    fn dir(&self) -> PathBuf {
        self.dir.borrow().clone()
    }

    fn setup(&self) {
        let l = self.lang();
        self.list.insert_column(col(l.col_name(), 366));
        self.list.insert_column(col(l.col_time(), 90));
        self.list.insert_column(col(l.col_freq(), 100));
        self.list.insert_column(col(l.col_status(), 132));
        self.list.set_headers_enabled(true);

        self.cmb_model
            .set_collection(vec!["sonnet".into(), "opus".into(), "haiku".into()]);
        self.cmb_model.set_selection(Some(0));
        self.wcmb_model
            .set_collection(vec!["sonnet".into(), "opus".into(), "haiku".into()]);
        self.wcmb_model.set_selection(Some(2));
        self.chk_skip.set_check_state(nwg::CheckBoxState::Checked);

        self.relabel();
    }

    fn relabel(&self) {
        let l = self.lang();
        self.window.set_text(l.win_title());
        self.lbl_name.set_text(l.lbl_name());
        self.lbl_time.set_text(l.lbl_time());
        self.lbl_freq.set_text(l.lbl_freq());
        self.lbl_dir.set_text(l.lbl_dir());
        self.btn_browse.set_text(l.btn_browse());
        self.lbl_model.set_text(l.lbl_model());
        self.chk_skip.set_text(l.lbl_skip());
        self.lbl_timeout.set_text(l.lbl_timeout());
        self.lbl_msg.set_text(l.lbl_message());
        self.btn_edit.set_text(l.btn_edit());
        self.btn_save.set_text(l.btn_save());
        self.btn_complete.set_text(l.btn_complete());
        self.btn_view.set_text(l.btn_view_output());
        self.btn_new.set_text(l.btn_new());
        self.btn_close.set_text(l.btn_close());

        let sel = self.cmb_freq.selection();
        self.cmb_freq
            .set_collection(vec![l.freq_once().into(), l.freq_daily().into()]);
        self.cmb_freq.set_selection(Some(sel.unwrap_or(0)));

        self.refresh_list();
    }

    fn show_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn open_manager(&self) {
        self.refresh_list();
        self.window.set_visible(true);
    }
    fn hide(&self) {
        self.window.set_visible(false);
    }
    fn on_close(&self, data: &nwg::EventData) {
        if let nwg::EventData::OnWindowClose(c) = data {
            c.close(false);
        }
        self.window.set_visible(false);
    }
    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
    fn info(&self, msg: &str) {
        nwg::modal_info_message(&self.window, self.lang().title_info(), msg);
    }

    fn set_lang_en(&self) {
        self.set_lang(Lang::En);
    }
    fn set_lang_zh(&self) {
        self.set_lang(Lang::ZhTw);
    }
    fn set_lang(&self, lang: Lang) {
        self.lang.set(lang);
        {
            let mut cfg = self.cfg.borrow_mut();
            cfg.language = lang;
            cfg.save(&self.dir());
        }
        self.relabel();
    }

    fn browse(&self) {
        if self.dir_dialog.run(Some(&self.window)) {
            if let Ok(p) = self.dir_dialog.get_selected_item() {
                self.in_dir.set_text(&p.to_string_lossy());
            }
        }
    }

    /// Read the form into a Task (id/created come from the caller).
    fn read_form(&self, id: String, created: String) -> Option<Task> {
        let time = self.in_time.text();
        let prompt = self.in_prompt.text();
        if prompt.trim().is_empty() || !valid_time(time.trim()) {
            self.info(self.lang().msg_need_prompt());
            return None;
        }
        let name = self.in_name.text();
        let freq = if self.cmb_freq.selection().unwrap_or(0) == 1 {
            "daily"
        } else {
            "once"
        };
        let model = self
            .cmb_model
            .selection_string()
            .unwrap_or_else(|| "sonnet".to_string());
        let skip = matches!(self.chk_skip.check_state(), nwg::CheckBoxState::Checked);
        let timeout = self
            .in_timeout
            .text()
            .trim()
            .parse::<u64>()
            .unwrap_or(240)
            .max(1);
        Some(Task {
            id,
            name: if name.trim().is_empty() {
                first_line(&prompt)
            } else {
                name.trim().to_string()
            },
            time: time.trim().to_string(),
            freq: freq.to_string(),
            dir: self.in_dir.text().trim().to_string(),
            prompt,
            model,
            skip_permissions: skip,
            timeout_min: timeout,
            status: Status::Pending,
            created,
            last_run: String::new(),
            exit_code: None,
            output_log: String::new(),
        })
    }

    /// Save = update the task being edited, or add a new one.
    fn save_task(&self) {
        let editing = self.editing_id.borrow().clone();
        let dir = self.dir();
        let exe = self.exe.borrow().clone();

        if let Some(id) = editing {
            let mut tasks = load_tasks(&dir);
            let created = tasks
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.created.clone())
                .unwrap_or_default();
            let task = match self.read_form(id.clone(), created) {
                Some(t) => t,
                None => return,
            };
            if let Some(slot) = tasks.iter_mut().find(|t| t.id == id) {
                *slot = task.clone();
            }
            let _ = save_tasks(&dir, &tasks);
            let _ = scheduler::unregister(&id);
            let _ = scheduler::register(&exe, &dir, &task);
            self.info(self.lang().msg_updated());
        } else {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let id = format!("{}-{:03}", timestamp_compact(), n);
            let task = match self.read_form(id, local_time_string()) {
                Some(t) => t,
                None => return,
            };
            let mut tasks = load_tasks(&dir);
            tasks.push(task.clone());
            let _ = save_tasks(&dir, &tasks);
            match scheduler::register(&exe, &dir, &task) {
                Ok(_) => self.info(self.lang().msg_added()),
                Err(_) => self.info(self.lang().msg_sched_failed()),
            }
        }
        self.clear_form();
        self.refresh_list();
    }

    fn edit_selected(&self) {
        let sel = match self.list.selected_item() {
            Some(i) => i,
            None => {
                self.info(self.lang().msg_select_first());
                return;
            }
        };
        let tasks = load_tasks(&self.dir());
        if let Some(t) = tasks.get(sel) {
            self.in_name.set_text(&t.name);
            self.in_time.set_text(&t.time);
            self.cmb_freq
                .set_selection(Some(if t.freq == "daily" { 1 } else { 0 }));
            self.in_dir.set_text(&t.dir);
            self.in_prompt.set_text(&t.prompt);
            self.in_timeout.set_text(&t.timeout_min.to_string());
            self.chk_skip.set_check_state(if t.skip_permissions {
                nwg::CheckBoxState::Checked
            } else {
                nwg::CheckBoxState::Unchecked
            });
            let mi = match t.model.as_str() {
                "opus" => 1,
                "haiku" => 2,
                _ => 0,
            };
            self.cmb_model.set_selection(Some(mi));
            *self.editing_id.borrow_mut() = Some(t.id.clone());
        }
    }

    fn complete_task(&self) {
        let sel = match self.list.selected_item() {
            Some(i) => i,
            None => {
                self.info(self.lang().msg_select_first());
                return;
            }
        };
        let dir = self.dir();
        let mut tasks = load_tasks(&dir);
        if sel < tasks.len() {
            let id = tasks[sel].id.clone();
            let _ = scheduler::unregister(&id);
            tasks.remove(sel);
            let _ = save_tasks(&dir, &tasks);
            if self.editing_id.borrow().as_deref() == Some(id.as_str()) {
                self.clear_form();
            }
            self.refresh_list();
        }
    }

    fn view_output(&self) {
        let sel = match self.list.selected_item() {
            Some(i) => i,
            None => {
                self.info(self.lang().msg_select_first());
                return;
            }
        };
        let tasks = load_tasks(&self.dir());
        if let Some(t) = tasks.get(sel) {
            if t.output_log.is_empty() || !PathBuf::from(&t.output_log).exists() {
                self.info(self.lang().msg_no_output());
            } else {
                let _ = Command::new("notepad").arg(&t.output_log).spawn();
            }
        }
    }

    fn clear_form(&self) {
        self.in_name.set_text("");
        self.in_time.set_text("03:00");
        self.cmb_freq.set_selection(Some(0));
        self.in_dir.set_text("");
        self.cmb_model.set_selection(Some(0));
        self.chk_skip.set_check_state(nwg::CheckBoxState::Checked);
        self.in_timeout.set_text("240");
        self.in_prompt.set_text("");
        *self.editing_id.borrow_mut() = None;
    }

    fn refresh_list(&self) {
        let l = self.lang();
        let tasks = load_tasks(&self.dir());
        self.list.clear();
        for (r, t) in tasks.iter().enumerate() {
            let freq = if t.freq == "daily" {
                l.freq_daily()
            } else {
                l.freq_once()
            };
            let status = match t.status {
                Status::Pending => l.st_pending(),
                Status::Running => l.st_running(),
                Status::Done => l.st_done(),
                Status::Failed => l.st_failed(),
            };
            set_cell(&self.list, r, 0, &t.name);
            set_cell(&self.list, r, 1, &t.time);
            set_cell(&self.list, r, 2, freq);
            set_cell(&self.list, r, 3, status);
        }
    }

    // ---- keep-warm window ----
    fn open_warm(&self) {
        let l = self.lang();
        let cfg = self.cfg.borrow();
        self.warm_window.set_text(l.warm_title());
        self.wchk_enabled.set_text(l.warm_enabled_lbl());
        self.wchk_enabled.set_check_state(if cfg.warm_enabled {
            nwg::CheckBoxState::Checked
        } else {
            nwg::CheckBoxState::Unchecked
        });
        self.wchk_notify.set_text(l.warm_notify_lbl());
        self.wchk_notify.set_check_state(if cfg.warm_notify {
            nwg::CheckBoxState::Checked
        } else {
            nwg::CheckBoxState::Unchecked
        });
        self.wlbl_mode.set_text(l.warm_mode_lbl());
        self.wcmb_mode
            .set_collection(vec![l.warm_mode_interval().into(), l.warm_mode_daily().into()]);
        self.wcmb_mode
            .set_selection(Some(if cfg.is_daily() { 1 } else { 0 }));
        self.wlbl_interval.set_text(l.warm_interval_lbl());
        self.win_interval
            .set_text(&cfg.warm_interval_minutes.to_string());
        self.wlbl_daily.set_text(l.warm_daily_lbl());
        self.win_daily.set_text(&times_str(&cfg.warm_daily_times));
        self.wlbl_model.set_text(l.warm_model_lbl());
        self.wcmb_model.set_selection(Some(match cfg.warm_model.as_str() {
            "opus" => 1,
            "haiku" => 2,
            _ => 0,
        }));
        self.wlbl_last.set_text(l.warm_last());
        self.wbtn_run.set_text(l.warm_run());
        self.wbtn_save.set_text(l.btn_apply());
        self.wbtn_close.set_text(l.btn_close());
        drop(cfg);
        self.update_warm_status();
        self.warm_window.set_visible(true);
    }

    fn update_warm_status(&self) {
        let r = runner::warm_result();
        let txt = if r.is_empty() {
            self.lang().warm_never().to_string()
        } else {
            r
        };
        self.wstatus.set_text(&txt);
    }

    fn warm_run_now(&self) {
        // Use the current (possibly unsaved) model/prompt from config + field.
        let mut c = self.cfg.borrow().clone();
        if let Some(m) = self.wcmb_model.selection_string() {
            c.warm_model = m;
        }
        // Honor the (possibly just-ticked) notify checkbox so this doubles as a
        // push test — even if the user hasn't pressed Save yet.
        c.warm_notify = matches!(self.wchk_notify.check_state(), nwg::CheckBoxState::Checked);
        self.wstatus.set_text(self.lang().warm_running());
        // Synchronous: a haiku ping is quick, and the result then shows below.
        runner::run_keep_warm(&c);
        self.update_warm_status();
    }

    fn warm_save(&self) {
        {
            let mut cfg = self.cfg.borrow_mut();
            cfg.warm_enabled = matches!(
                self.wchk_enabled.check_state(),
                nwg::CheckBoxState::Checked
            );
            cfg.warm_notify = matches!(
                self.wchk_notify.check_state(),
                nwg::CheckBoxState::Checked
            );
            cfg.warm_mode = if self.wcmb_mode.selection().unwrap_or(0) == 1 {
                "daily".to_string()
            } else {
                "interval".to_string()
            };
            cfg.warm_interval_minutes = self
                .win_interval
                .text()
                .trim()
                .parse::<u64>()
                .unwrap_or(300)
                .max(1);
            cfg.warm_daily_times = parse_times_field(&self.win_daily.text());
            if let Some(m) = self.wcmb_model.selection_string() {
                cfg.warm_model = m;
            }
            cfg.save(&self.dir());
        }
        // (Re)register or remove the keep-warm wake-job to match the new settings.
        let exe = self.exe.borrow().clone();
        let dir = self.dir();
        let (en, daily, times) = {
            let c = self.cfg.borrow();
            (c.warm_enabled, c.is_daily(), c.warm_daily_times.clone())
        };
        sync_warm_job(exe, dir, en, daily, times);
        self.info(self.lang().msg_saved());
    }

    fn warm_hide(&self) {
        self.warm_window.set_visible(false);
    }
    fn warm_on_close(&self, data: &nwg::EventData) {
        if let nwg::EventData::OnWindowClose(c) = data {
            c.close(false);
        }
        self.warm_window.set_visible(false);
    }

    // ---- notifications (Feishu push) window ----
    fn open_notif(&self) {
        let l = self.lang();
        self.notif_window.set_text(l.notif_title());
        self.nlbl_intro.set_text(l.notif_intro());
        self.nlbl_hooks.set_text(l.notif_hooks_lbl());
        self.nbtn_test.set_text(l.notif_test());
        self.nbtn_save.set_text(l.btn_apply());
        self.nbtn_close.set_text(l.btn_close());
        // One URL per line (CRLF for the Win32 multiline edit control).
        let text = self.cfg.borrow().feishu_hooks.join("\r\n");
        self.nin_hooks.set_text(&text);
        self.notif_window.set_visible(true);
    }

    fn notif_save(&self) {
        let hooks = crate::config::parse_hooks(&self.nin_hooks.text());
        {
            let mut cfg = self.cfg.borrow_mut();
            cfg.feishu_hooks = hooks;
            cfg.save(&self.dir());
        }
        self.info(self.lang().msg_saved());
    }

    fn notif_test(&self) {
        // Test against what's currently typed, even if not saved yet.
        let hooks = crate::config::parse_hooks(&self.nin_hooks.text());
        if hooks.is_empty() {
            self.info(self.lang().notif_test_empty());
            return;
        }
        runner::notify_feishu(&hooks, self.lang().notif_test_text());
        self.info(self.lang().notif_test_sent());
    }

    fn notif_hide(&self) {
        self.notif_window.set_visible(false);
    }
    fn notif_on_close(&self, data: &nwg::EventData) {
        if let nwg::EventData::OnWindowClose(c) = data {
            c.close(false);
        }
        self.notif_window.set_visible(false);
    }
}

fn col(text: &str, width: i32) -> nwg::InsertListViewColumn {
    nwg::InsertListViewColumn {
        index: None,
        fmt: None,
        width: Some(width),
        text: Some(text.to_string()),
    }
}

fn set_cell(list: &nwg::ListView, row: usize, column: i32, text: &str) {
    list.insert_item(nwg::InsertListViewItem {
        index: Some(row as i32),
        column_index: column,
        text: Some(text.to_string()),
        ..Default::default()
    });
}

fn times_str(times: &[(u8, u8)]) -> String {
    times
        .iter()
        .map(|(h, m)| format!("{:02}:{:02}", h, m))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_times_field(s: &str) -> Vec<(u8, u8)> {
    s.split(',')
        .filter_map(|t| {
            let (h, m) = t.trim().split_once(':')?;
            let (h, m): (u8, u8) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
            if h < 24 && m < 60 {
                Some((h, m))
            } else {
                None
            }
        })
        .collect()
}

fn valid_time(s: &str) -> bool {
    let mut it = s.split(':');
    match (it.next(), it.next(), it.next()) {
        (Some(h), Some(m), None) => {
            matches!((h.parse::<u32>(), m.parse::<u32>()), (Ok(h), Ok(m)) if h < 24 && m < 60)
        }
        _ => false,
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().chars().take(24).collect()
}

/// Register or remove the keep-warm wake-job on a background thread (the
/// PowerShell call is slow; don't block the UI).
fn sync_warm_job(exe: PathBuf, dir: PathBuf, enabled: bool, daily: bool, times: Vec<(u8, u8)>) {
    std::thread::spawn(move || {
        if enabled && daily && !times.is_empty() {
            let _ = scheduler::register_warm(&exe, &dir, &times);
        } else {
            let _ = scheduler::unregister_warm();
        }
    });
}

/// Build the UI and run the event loop (blocks until Quit).
pub fn run(dir: PathBuf, exe: PathBuf) {
    nwg::init().expect("Failed to init native-windows-gui");
    let cfg = Config::load(&dir);
    let lang = cfg.language;
    // Keep the daily keep-warm wake-job in sync with the saved settings on start.
    sync_warm_job(
        exe.clone(),
        dir.clone(),
        cfg.warm_enabled,
        cfg.is_daily(),
        cfg.warm_daily_times.clone(),
    );
    let app = App {
        dir: RefCell::new(dir),
        exe: RefCell::new(exe),
        lang: Cell::new(lang),
        cfg: RefCell::new(cfg),
        ..Default::default()
    };
    let ui = App::build_ui(app).expect("Failed to build UI");
    ui.setup();
    nwg::dispatch_thread_events();
}
