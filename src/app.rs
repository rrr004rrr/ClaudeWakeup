//! Cross-platform GUI: a single egui/eframe codebase shared by Windows and macOS.
//! A tray / menu-bar icon (tray-icon) opens one window with three tabs:
//!   - Tasks:     create / edit / remove overnight tasks (each backed by an OS
//!                scheduled job that wakes the machine — see `platform`).
//!   - Keep-warm: the periodic cheap `claude -p` ping, with last-result feedback.
//!   - Push:      Feishu/Lark webhook URLs notified when a task finishes.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::config::{parse_hooks, Config};
use crate::i18n::Lang;
use crate::platform;
use crate::runner;
use crate::task::{load_tasks, save_tasks, Status, Task};
use crate::util::{local_time_string, timestamp_compact};

const ICON: &[u8] = include_bytes!("../assets/icon.ico");
const MODELS: [&str; 3] = ["sonnet", "opus", "haiku"];

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Tasks,
    Warm,
    Push,
}

/// Editable task form (mirrors a `Task`; strings so the user can type freely).
struct TaskForm {
    name: String,
    time: String,
    daily: bool,
    dir: String,
    model: String,
    skip: bool,
    timeout: String,
    prompt: String,
}

impl Default for TaskForm {
    fn default() -> Self {
        TaskForm {
            name: String::new(),
            time: "03:00".to_string(),
            daily: false,
            dir: String::new(),
            model: "sonnet".to_string(),
            skip: true,
            timeout: "240".to_string(),
            prompt: String::new(),
        }
    }
}

pub struct App {
    dir: PathBuf,
    exe: PathBuf,
    cfg: Config,
    lang: Lang,

    tab: Tab,
    visible: bool,
    want_quit: bool,
    counter: u32,
    status: String,

    tasks: Vec<Task>,
    selected: Option<usize>,
    editing_id: Option<String>,
    form: TaskForm,
    /// Throttle for re-reading tasks.json so status changes (written by the
    /// headless runner) show up live without a manual refresh.
    last_reload: Instant,

    // keep-warm form mirrors cfg until saved
    last_warm: String,
    /// Push tab: Feishu webhook URLs, one per line (mirrors cfg.feishu_hooks).
    push_hooks: String,

    // tray (kept alive) + menu item ids for event matching
    _tray: Option<TrayIcon>,
    id_tasks: MenuId,
    id_warm: MenuId,
    id_push: MenuId,
    id_lang_en: MenuId,
    id_lang_zh: MenuId,
    id_quit: MenuId,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, dir: PathBuf, exe: PathBuf, rgba: Vec<u8>, w: u32, h: u32) -> Self {
        install_cjk_font(&cc.egui_ctx);
        apply_style(&cc.egui_ctx);

        let cfg = Config::load(&dir);
        let lang = cfg.language;
        let tasks = load_tasks(&dir);
        let push_hooks = cfg.feishu_hooks.join("\n");

        // Build the tray menu (bilingual labels, so no relabel on language switch).
        let menu = Menu::new();
        let mi_tasks = MenuItem::new("任務管理 / Task manager", true, None);
        let mi_warm = MenuItem::new("保溫設定 / Keep-warm", true, None);
        let mi_push = MenuItem::new("推播設定 / Push", true, None);
        let lang_sub = Submenu::new("語言 / Language", true);
        let mi_en = MenuItem::new("English", true, None);
        let mi_zh = MenuItem::new("繁體中文", true, None);
        let _ = lang_sub.append_items(&[&mi_en, &mi_zh]);
        let mi_quit = MenuItem::new("結束 / Quit", true, None);
        let _ = menu.append_items(&[
            &mi_tasks,
            &mi_warm,
            &mi_push,
            &PredefinedMenuItem::separator(),
            &lang_sub,
            &PredefinedMenuItem::separator(),
            &mi_quit,
        ]);

        let (id_tasks, id_warm, id_push, id_lang_en, id_lang_zh, id_quit) = (
            mi_tasks.id().clone(),
            mi_warm.id().clone(),
            mi_push.id().clone(),
            mi_en.id().clone(),
            mi_zh.id().clone(),
            mi_quit.id().clone(),
        );

        let tray = tray_icon::Icon::from_rgba(rgba, w, h)
            .ok()
            .and_then(|icon| {
                TrayIconBuilder::new()
                    .with_menu(Box::new(menu))
                    .with_tooltip("Claude Wakeup")
                    .with_icon(icon)
                    .build()
                    .ok()
            });

        let app = App {
            dir,
            exe,
            cfg,
            lang,
            tab: Tab::Tasks,
            visible: false,
            want_quit: false,
            counter: 0,
            status: String::new(),
            tasks,
            selected: None,
            editing_id: None,
            form: TaskForm::default(),
            last_reload: Instant::now(),
            last_warm: String::new(),
            push_hooks,
            _tray: tray,
            id_tasks,
            id_warm,
            id_push,
            id_lang_en,
            id_lang_zh,
            id_quit,
        };
        // Silently keep the scheduled jobs pointed at the *current* binary, so a
        // moved/renamed app (or a fresh build) doesn't leave jobs aimed at a stale
        // path — and so all jobs share one binary identity in macOS background
        // activity. (Wake arming via pmset needs an admin prompt, so that stays an
        // explicit Save — no password prompt on every login.)
        app.reschedule_warm();
        app.reschedule_tasks();
        app
    }

    // ---- helpers ----------------------------------------------------------

    fn refresh(&mut self) {
        self.tasks = load_tasks(&self.dir);
        if let Some(s) = self.selected {
            if s >= self.tasks.len() {
                self.selected = None;
            }
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.visible = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn hide(&mut self, ctx: &egui::Context) {
        self.visible = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    /// All clock times the machine should wake for: task times + warm daily times.
    fn collect_wake_times(&self) -> Vec<(u8, u8)> {
        let mut times: Vec<(u8, u8)> = self
            .tasks
            .iter()
            .filter_map(|t| parse_hhmm(&t.time))
            .collect();
        if self.cfg.warm_enabled && self.cfg.is_daily() {
            times.extend(self.cfg.warm_daily_times.iter().copied());
        }
        times.sort_unstable();
        times.dedup();
        times
    }

    /// Re-arm OS wake-ups for all scheduled times (macOS pmset; no-op elsewhere).
    /// Runs on a thread because it may show an admin prompt / be slow.
    fn sync_wakes(&self) {
        let times = self.collect_wake_times();
        std::thread::spawn(move || {
            let _ = platform::sync_wake_schedule(&times);
        });
    }

    /// Re-register the OS jobs for all still-active tasks against the current
    /// binary path (daily tasks, plus once-tasks that haven't run yet). Done jobs
    /// are left alone — a finished once-task already removed its own job.
    fn reschedule_tasks(&self) {
        let exe = self.exe.clone();
        let dir = self.dir.clone();
        let tasks = self.tasks.clone();
        std::thread::spawn(move || {
            for t in &tasks {
                let active =
                    t.freq == "daily" || matches!(t.status, Status::Pending | Status::Running);
                if active {
                    let _ = platform::register(&exe, &dir, t);
                }
            }
        });
    }

    /// (Re)register or remove the keep-warm job to match current settings.
    fn reschedule_warm(&self) {
        let exe = self.exe.clone();
        let dir = self.dir.clone();
        let on = self.cfg.warm_enabled && self.cfg.is_daily();
        let times = self.cfg.warm_daily_times.clone();
        std::thread::spawn(move || {
            if on && !times.is_empty() {
                let _ = platform::register_warm(&exe, &dir, &times);
            } else {
                let _ = platform::unregister_warm();
            }
        });
    }

    fn load_form(&mut self, t: &Task) {
        self.form = TaskForm {
            name: t.name.clone(),
            time: t.time.clone(),
            daily: t.freq == "daily",
            dir: t.dir.clone(),
            model: t.model.clone(),
            skip: t.skip_permissions,
            timeout: t.timeout_min.to_string(),
            prompt: t.prompt.clone(),
        };
        self.editing_id = Some(t.id.clone());
    }

    fn clear_form(&mut self) {
        self.form = TaskForm::default();
        self.editing_id = None;
    }

    /// Build a Task from the form, or return None (with a status message) if invalid.
    fn read_form(&mut self, id: String, created: String) -> Option<Task> {
        if self.form.prompt.trim().is_empty() || !valid_time(self.form.time.trim()) {
            self.status = self.lang().msg_need_prompt().to_string();
            return None;
        }
        let name = if self.form.name.trim().is_empty() {
            first_line(&self.form.prompt)
        } else {
            self.form.name.trim().to_string()
        };
        let timeout = self.form.timeout.trim().parse::<u64>().unwrap_or(240).max(1);
        Some(Task {
            id,
            name,
            time: self.form.time.trim().to_string(),
            freq: if self.form.daily { "daily" } else { "once" }.to_string(),
            dir: self.form.dir.trim().to_string(),
            prompt: self.form.prompt.clone(),
            model: self.form.model.clone(),
            skip_permissions: self.form.skip,
            timeout_min: timeout,
            status: Status::Pending,
            created,
            last_run: String::new(),
            exit_code: None,
            output_log: String::new(),
        })
    }

    fn save_task(&mut self) {
        let editing = self.editing_id.clone();
        if let Some(id) = editing {
            let mut tasks = load_tasks(&self.dir);
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
            let _ = save_tasks(&self.dir, &tasks);
            let _ = platform::unregister(&id);
            let _ = platform::register(&self.exe, &self.dir, &task);
            self.status = self.lang().msg_updated().to_string();
        } else {
            let id = format!("{}-{:03}", timestamp_compact(), self.counter);
            self.counter += 1;
            let task = match self.read_form(id, local_time_string()) {
                Some(t) => t,
                None => return,
            };
            let mut tasks = load_tasks(&self.dir);
            tasks.push(task.clone());
            let _ = save_tasks(&self.dir, &tasks);
            self.status = match platform::register(&self.exe, &self.dir, &task) {
                Ok(_) => self.lang().msg_added().to_string(),
                Err(_) => self.lang().msg_sched_failed().to_string(),
            };
        }
        self.clear_form();
        self.refresh();
    }

    fn remove_selected(&mut self) {
        let sel = match self.selected {
            Some(i) => i,
            None => {
                self.status = self.lang().msg_select_first().to_string();
                return;
            }
        };
        let mut tasks = load_tasks(&self.dir);
        if sel < tasks.len() {
            let id = tasks[sel].id.clone();
            let _ = platform::unregister(&id);
            tasks.remove(sel);
            let _ = save_tasks(&self.dir, &tasks);
            if self.editing_id.as_deref() == Some(id.as_str()) {
                self.clear_form();
            }
            self.selected = None;
            self.refresh();
        }
    }

    fn view_output(&mut self) {
        let sel = match self.selected {
            Some(i) => i,
            None => {
                self.status = self.lang().msg_select_first().to_string();
                return;
            }
        };
        if let Some(t) = self.tasks.get(sel) {
            if t.output_log.is_empty() || !PathBuf::from(&t.output_log).exists() {
                self.status = self.lang().msg_no_output().to_string();
            } else {
                platform::open_path(&t.output_log);
            }
        }
    }

    fn lang(&self) -> Lang {
        self.lang
    }

    fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
        self.cfg.language = lang;
        self.cfg.save(&self.dir);
    }

    fn warm_save(&mut self) {
        self.cfg.save(&self.dir);
        self.reschedule_warm();
        self.status = self.lang().msg_saved().to_string();
    }

    /// Explicit, opt-in macOS wake arming (the only path that shows the admin
    /// prompt). On Windows this is a harmless no-op.
    fn arm_wake(&mut self) {
        self.sync_wakes();
        self.status = self.lang().msg_wake_armed().to_string();
    }

    fn warm_run_now(&self) {
        let cfg = self.cfg.clone();
        std::thread::spawn(move || runner::run_keep_warm(&cfg));
    }

    // ---- event handling ---------------------------------------------------

    fn poll_tray(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id == self.id_tasks {
                self.tab = Tab::Tasks;
                self.refresh();
                self.show(ctx);
            } else if ev.id == self.id_warm {
                self.tab = Tab::Warm;
                self.show(ctx);
            } else if ev.id == self.id_push {
                self.tab = Tab::Push;
                self.show(ctx);
            } else if ev.id == self.id_lang_en {
                self.set_lang(Lang::En);
            } else if ev.id == self.id_lang_zh {
                self.set_lang(Lang::ZhTw);
            } else if ev.id == self.id_quit {
                self.want_quit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = ev
            {
                self.tab = Tab::Tasks;
                self.refresh();
                self.show(ctx);
            }
        }
    }

    // ---- UI ---------------------------------------------------------------

    fn ui_tabs(&mut self, ui: &mut egui::Ui) {
        let l = self.lang();
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for (tab, label) in [
                (Tab::Tasks, l.tab_tasks()),
                (Tab::Warm, l.tab_warm()),
                (Tab::Push, l.tab_push()),
            ] {
                let selected = self.tab == tab;
                if ui
                    .selectable_label(selected, egui::RichText::new(label).size(16.0))
                    .clicked()
                {
                    self.tab = tab;
                }
            }
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(6.0);
    }

    fn ui_tasks(&mut self, ui: &mut egui::Ui) {
        let l = self.lang();

        // Task list, in a bordered panel with a fixed, compact height.
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .min_scrolled_height(132.0)
                    .max_height(132.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        if self.tasks.is_empty() {
                            ui.add_space(8.0);
                            ui.weak(l.no_tasks());
                            return;
                        }
                        egui::Grid::new("tasks_grid")
                            .num_columns(4)
                            .striped(true)
                            .spacing([18.0, 6.0])
                            .min_col_width(80.0)
                            .show(ui, |ui| {
                                ui.strong(l.col_name());
                                ui.strong(l.col_time());
                                ui.strong(l.col_freq());
                                ui.strong(l.col_status());
                                ui.end_row();
                                for (i, t) in self.tasks.iter().enumerate() {
                                    let selected = self.selected == Some(i);
                                    let freq = if t.freq == "daily" { l.freq_daily() } else { l.freq_once() };
                                    let status = match t.status {
                                        Status::Pending => l.st_pending(),
                                        Status::Running => l.st_running(),
                                        Status::Done => l.st_done(),
                                        Status::Failed => l.st_failed(),
                                    };
                                    if ui.selectable_label(selected, &t.name).clicked() {
                                        self.selected = Some(i);
                                    }
                                    ui.label(&t.time);
                                    ui.label(freq);
                                    ui.label(status);
                                    ui.end_row();
                                }
                            });
                    });
            });

        ui.add_space(12.0);

        // Form.
        egui::Grid::new("task_form")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label(l.lbl_name());
                ui.add(egui::TextEdit::singleline(&mut self.form.name).desired_width(f32::INFINITY));
                ui.end_row();

                ui.label(l.lbl_time());
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.form.time).desired_width(80.0));
                    ui.label(l.lbl_freq());
                    egui::ComboBox::from_id_salt("freq")
                        .selected_text(if self.form.daily { l.freq_daily() } else { l.freq_once() })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.form.daily, false, l.freq_once());
                            ui.selectable_value(&mut self.form.daily, true, l.freq_daily());
                        });
                });
                ui.end_row();

                ui.label(l.lbl_dir());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.form.dir).desired_width(360.0),
                    );
                    if ui.button(l.btn_browse()).clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            self.form.dir = p.display().to_string();
                        }
                    }
                });
                ui.end_row();

                ui.label(l.lbl_model());
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("model")
                        .selected_text(&self.form.model)
                        .show_ui(ui, |ui| {
                            for m in MODELS {
                                ui.selectable_value(&mut self.form.model, m.to_string(), m);
                            }
                        });
                    ui.checkbox(&mut self.form.skip, l.lbl_skip());
                    ui.label(l.lbl_timeout());
                    ui.add(egui::TextEdit::singleline(&mut self.form.timeout).desired_width(70.0));
                });
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.label(l.lbl_message());
        ui.add(
            egui::TextEdit::multiline(&mut self.form.prompt)
                .desired_rows(4)
                .desired_width(f32::INFINITY),
        );

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button(l.btn_edit()).clicked() {
                if let Some(sel) = self.selected {
                    if let Some(t) = self.tasks.get(sel).cloned() {
                        self.load_form(&t);
                    }
                } else {
                    self.status = l.msg_select_first().to_string();
                }
            }
            if ui.button(l.btn_save()).clicked() {
                self.save_task();
            }
            if ui.button(l.btn_complete()).clicked() {
                self.remove_selected();
            }
            if ui.button(l.btn_view_output()).clicked() {
                self.view_output();
            }
            if ui.button(l.btn_new()).clicked() {
                self.clear_form();
            }
        });
    }

    fn ui_warm(&mut self, ui: &mut egui::Ui) {
        let l = self.lang();
        ui.checkbox(&mut self.cfg.warm_enabled, l.warm_enabled_lbl());
        ui.checkbox(&mut self.cfg.warm_notify, l.warm_notify_lbl());

        let mut daily = self.cfg.is_daily();
        ui.horizontal(|ui| {
            ui.label(l.warm_mode_lbl());
            egui::ComboBox::from_id_salt("warm_mode")
                .selected_text(if daily { l.warm_mode_daily() } else { l.warm_mode_interval() })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut daily, false, l.warm_mode_interval());
                    ui.selectable_value(&mut daily, true, l.warm_mode_daily());
                });
        });
        self.cfg.warm_mode = if daily { "daily" } else { "interval" }.to_string();

        if daily {
            ui.horizontal(|ui| {
                ui.label(l.warm_daily_lbl());
                let mut s = times_str(&self.cfg.warm_daily_times);
                if ui
                    .add(egui::TextEdit::singleline(&mut s).desired_width(300.0))
                    .changed()
                {
                    self.cfg.warm_daily_times = parse_times_field(&s);
                }
            });
        } else {
            ui.horizontal(|ui| {
                ui.label(l.warm_interval_lbl());
                let mut s = self.cfg.warm_interval_minutes.to_string();
                if ui
                    .add(egui::TextEdit::singleline(&mut s).desired_width(90.0))
                    .changed()
                {
                    self.cfg.warm_interval_minutes = s.trim().parse::<u64>().unwrap_or(300).max(1);
                }
            });
        }

        ui.horizontal(|ui| {
            ui.label(l.warm_model_lbl());
            egui::ComboBox::from_id_salt("warm_model")
                .selected_text(&self.cfg.warm_model)
                .show_ui(ui, |ui| {
                    for m in MODELS {
                        ui.selectable_value(&mut self.cfg.warm_model, m.to_string(), m);
                    }
                });
        });

        ui.separator();
        ui.label(l.warm_last());
        let last = if self.last_warm.is_empty() {
            l.warm_never().to_string()
        } else {
            self.last_warm.clone()
        };
        ui.label(last);

        ui.horizontal(|ui| {
            if ui.button(l.warm_run()).clicked() {
                self.last_warm = l.warm_running().to_string();
                self.warm_run_now();
            }
            if ui.button(l.btn_apply()).clicked() {
                self.warm_save();
            }
        });

        // macOS hardware wake is opt-in (it needs an admin prompt), so it lives
        // behind an explicit button instead of firing on every Save.
        if cfg!(target_os = "macos") {
            ui.add_space(10.0);
            ui.separator();
            ui.label(egui::RichText::new(l.warm_wake_hint()).weak());
            if ui.button(l.warm_arm_wake()).clicked() {
                self.arm_wake();
            }
        }
    }

    // ---- push (Feishu webhooks) tab ---------------------------------------
    fn ui_push(&mut self, ui: &mut egui::Ui) {
        let l = self.lang();
        ui.label(egui::RichText::new(l.notif_intro()).weak());
        ui.add_space(8.0);
        ui.label(l.notif_hooks_lbl());
        ui.add(
            egui::TextEdit::multiline(&mut self.push_hooks)
                .desired_rows(6)
                .desired_width(f32::INFINITY)
                .hint_text("https://open.feishu.cn/open-apis/bot/v2/hook/…"),
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button(l.notif_test()).clicked() {
                self.push_test();
            }
            if ui.button(l.btn_apply()).clicked() {
                self.push_save();
            }
        });
    }

    fn push_save(&mut self) {
        self.cfg.feishu_hooks = parse_hooks(&self.push_hooks);
        self.cfg.save(&self.dir);
        self.status = self.lang().msg_saved().to_string();
    }

    fn push_test(&mut self) {
        let hooks = parse_hooks(&self.push_hooks);
        if hooks.is_empty() {
            self.status = self.lang().notif_test_empty().to_string();
            return;
        }
        let text = self.lang().notif_test_text().to_string();
        std::thread::spawn(move || runner::notify_feishu(&hooks, &text));
        self.status = self.lang().notif_test_sent().to_string();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_tray(ctx);

        // Pull the latest keep-warm result (set by the background ping thread).
        let r = runner::warm_result();
        if !r.is_empty() {
            self.last_warm = r;
        }

        // Re-read tasks.json once a second while visible, so a task's status flips
        // from Running to Done/Failed live when the headless runner finishes.
        if self.visible && self.last_reload.elapsed() >= Duration::from_secs(1) {
            self.refresh();
            self.last_reload = Instant::now();
        }

        // Close = hide to tray, unless Quit was chosen.
        if ctx.input(|i| i.viewport().close_requested()) && !self.want_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide(ctx);
        }

        let frame = egui::Frame::central_panel(&ctx.style())
            .inner_margin(egui::Margin::symmetric(18.0, 14.0));
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            self.ui_tabs(ui);
            match self.tab {
                Tab::Tasks => self.ui_tasks(ui),
                Tab::Warm => self.ui_warm(ui),
                Tab::Push => self.ui_push(ui),
            }
            if !self.status.is_empty() {
                ui.add_space(6.0);
                ui.separator();
                ui.colored_label(ui.visuals().weak_text_color(), &self.status);
            }
        });

        // Keep polling tray/menu events and the warm result.
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}

/// A bit more breathing room than egui's defaults: slightly larger UI, roomier
/// spacing and button padding, gentler rounding.
fn apply_style(ctx: &egui::Context) {
    ctx.set_zoom_factor(1.15);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size.y = 28.0;
    style.spacing.combo_width = 150.0;
    let r = egui::Rounding::same(6.0);
    style.visuals.widgets.inactive.rounding = r;
    style.visuals.widgets.hovered.rounding = r;
    style.visuals.widgets.active.rounding = r;
    style.visuals.widgets.open.rounding = r;
    ctx.set_style(style);
}

/// Load a system CJK font so 繁體中文 labels render (egui's bundled fonts are
/// Latin-only). Best-effort: if none is found, Latin text still works.
fn install_cjk_font(ctx: &egui::Context) {
    #[cfg(target_os = "macos")]
    let candidates: &[&str] = &[
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/Library/Fonts/Arial Unicode.ttf",
    ];
    #[cfg(windows)]
    let candidates: &[&str] = &[
        "C:\\Windows\\Fonts\\msjh.ttc",
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\simsun.ttc",
        "C:\\Windows\\Fonts\\mingliu.ttc",
    ];

    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            // Keep egui's bundled Latin font as primary (its row metrics give the
            // correct, centered line height); add the CJK font as a *fallback*,
            // nudged down so its glyphs land on the Latin baseline instead of
            // floating above it.
            let mut cjk = egui::FontData::from_owned(bytes);
            cjk.tweak.y_offset_factor = 0.18;
            fonts.font_data.insert("cjk".to_owned(), cjk);
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(family).or_default().push("cjk".to_owned());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

/// Build the UI and run the event loop (blocks until Quit).
pub fn run(dir: PathBuf, exe: PathBuf) {
    // Decode the .ico once for both the window icon and the tray icon.
    let (rgba, w, h) = match image::load_from_memory(ICON) {
        Ok(img) => {
            let img = img.to_rgba8();
            let (w, h) = img.dimensions();
            (img.into_raw(), w, h)
        }
        Err(_) => (vec![0u8; 4], 1, 1),
    };

    let icon = egui::IconData {
        rgba: rgba.clone(),
        width: w,
        height: h,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([780.0, 720.0])
            .with_min_inner_size([560.0, 560.0])
            .with_visible(false)
            .with_icon(Arc::new(icon)),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "ClaudeWakeup",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, dir, exe, rgba, w, h)))),
    );
}

// ---- small helpers ---------------------------------------------------------

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
            (h < 24 && m < 60).then_some((h, m))
        })
        .collect()
}

fn parse_hhmm(s: &str) -> Option<(u8, u8)> {
    let (h, m) = s.trim().split_once(':')?;
    let (h, m): (u8, u8) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    (h < 24 && m < 60).then_some((h, m))
}

fn valid_time(s: &str) -> bool {
    parse_hhmm(s).is_some()
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().chars().take(24).collect()
}
