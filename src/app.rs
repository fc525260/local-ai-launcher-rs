use crate::command::{bat_script, build_args, command_preview, llama_server_path};
use crate::config::{load_config, save_config, AppConfig, ManualModel, Preset};
use crate::discovery::{
    discover_configured_models, discover_draft_models, DraftModelInfo, ModelInfo,
};
use crate::server::{self, ServerEvent, ServerProcess};
use eframe::egui::{
    self, Color32, CornerRadius, CursorIcon, FontId, PointerButton, RichText, Stroke, Vec2,
};
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const INK: Color32 = Color32::from_rgb(29, 29, 31);
const GRAPHITE: Color32 = Color32::from_rgb(112, 112, 112);
const FOG: Color32 = Color32::from_rgb(245, 245, 247);
const SNOW: Color32 = Color32::WHITE;
const SILVER_MIST: Color32 = Color32::from_rgb(232, 232, 237);
const AZURE: Color32 = Color32::from_rgb(0, 113, 227);
const COBALT_LINK: Color32 = Color32::from_rgb(0, 102, 204);
const CAUTION: Color32 = Color32::from_rgb(182, 68, 0);

pub fn configure_fonts(ctx: &egui::Context) {
    configure_theme(ctx);
    let mut fonts = egui::FontDefinitions::default();
    if let Some((name, data)) = load_chinese_font() {
        fonts
            .font_data
            .insert(name.clone(), egui::FontData::from_owned(data).into());
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, name.clone());
        }
        ctx.set_fonts(fonts);
    }
}

fn configure_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(INK);
    visuals.weak_text_color = Some(GRAPHITE);
    visuals.hyperlink_color = COBALT_LINK;
    visuals.faint_bg_color = FOG;
    visuals.extreme_bg_color = SILVER_MIST;
    visuals.text_edit_bg_color = Some(SNOW);
    visuals.code_bg_color = FOG;
    visuals.panel_fill = FOG;
    visuals.window_fill = SNOW;
    visuals.window_stroke = Stroke::new(1.0, SILVER_MIST);
    visuals.window_corner_radius = CornerRadius::same(28);
    visuals.menu_corner_radius = CornerRadius::same(18);
    visuals.warn_fg_color = CAUTION;
    visuals.button_frame = true;
    visuals.collapsing_header_frame = false;
    visuals.indent_has_left_vline = false;
    visuals.striped = false;

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::same(12);
        widget.bg_stroke = Stroke::new(1.0, SILVER_MIST);
        widget.fg_stroke = Stroke::new(1.0, INK);
    }
    visuals.widgets.noninteractive.bg_fill = SNOW;
    visuals.widgets.inactive.bg_fill = SNOW;
    visuals.widgets.inactive.weak_bg_fill = SILVER_MIST;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(250, 250, 252);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(238, 238, 242);
    visuals.widgets.active.bg_fill = SILVER_MIST;
    visuals.widgets.open.bg_fill = SNOW;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(16.0, 8.0);
    style.spacing.interact_size = Vec2::new(32.0, 28.0);
    style.spacing.text_edit_width = 180.0;
    style.spacing.combo_width = 150.0;
    style.spacing.icon_width = 18.0;
    style.spacing.icon_width_inner = 12.0;
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(26.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(16.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::new(14.5, egui::FontFamily::Monospace),
    );
    ctx.set_style(style);
}

fn load_chinese_font() -> Option<(String, Vec<u8>)> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let candidates = [
        "NotoSansSC-VF.ttf",
        "msyh.ttc",
        "Deng.ttf",
        "simhei.ttf",
        "simsun.ttc",
    ];

    candidates.iter().find_map(|file| {
        let path = Path::new(&windir).join("Fonts").join(file);
        fs::read(&path)
            .ok()
            .map(|data| (format!("chinese-{}", file), data))
    })
}

#[derive(Clone, Copy)]
struct ParamHelp {
    title: &'static str,
    purpose: &'static str,
    cpu: &'static str,
    memory: &'static str,
    vram: &'static str,
}

const fn help(
    title: &'static str,
    purpose: &'static str,
    cpu: &'static str,
    memory: &'static str,
    vram: &'static str,
) -> ParamHelp {
    ParamHelp {
        title,
        purpose,
        cpu,
        memory,
        vram,
    }
}

fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "模型".to_string()
    } else {
        trimmed.to_string()
    }
}

fn rgba_to_color(value: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(value[0], value[1], value[2], value[3])
}

fn color_to_rgba(value: Color32) -> [u8; 4] {
    [value.r(), value.g(), value.b(), value.a()]
}

pub struct LauncherApp {
    config: AppConfig,
    models: Vec<ModelInfo>,
    search: String,
    selected_index: Option<usize>,
    preset: Preset,
    use_mm: bool,
    logs: Vec<String>,
    warnings: Vec<String>,
    server: Option<ServerProcess>,
    status: String,
    help_popup: Option<ParamHelp>,
    rename_popup_model: Option<String>,
    rename_popup_text: String,
    dragging_model: Option<String>,
    show_log_window: bool,
    active_preset_label: String,
    show_draft_picker: bool,
}

impl LauncherApp {
    pub fn new() -> Self {
        let mut config = load_config();
        let models = discover_configured_models(&config);
        let valid_ids: std::collections::BTreeSet<String> =
            models.iter().map(|model| model.id.clone()).collect();
        let before = config.model_presets.len();
        config
            .model_presets
            .retain(|model_id, _| valid_ids.contains(model_id));
        if config.model_presets.len() != before {
            let _ = save_config(&config);
        }
        let preset = config
            .global_presets
            .get("默认")
            .cloned()
            .unwrap_or_default();
        Self {
            config,
            models,
            search: String::new(),
            selected_index: None,
            preset,
            use_mm: true,
            logs: Vec::new(),
            warnings: Vec::new(),
            server: None,
            status: "空闲".to_string(),
            help_popup: None,
            rename_popup_model: None,
            rename_popup_text: String::new(),
            dragging_model: None,
            show_log_window: false,
            active_preset_label: "默认".to_string(),
            show_draft_picker: false,
        }
    }

    fn selected_model(&self) -> Option<&ModelInfo> {
        self.selected_index.and_then(|idx| self.models.get(idx))
    }

    fn refresh_models(&mut self) {
        self.models = discover_configured_models(&self.config);
        self.prune_missing_model_presets();
        self.selected_index = None;
        self.preset = self
            .config
            .global_presets
            .get("默认")
            .cloned()
            .unwrap_or_default();
        self.active_preset_label = "默认".to_string();
        self.status = format!("{} 个模型", self.models.len());
    }

    fn select_model(&mut self, idx: usize) {
        self.selected_index = Some(idx);
        if let Some(model) = self.models.get(idx) {
            self.use_mm = model.mmproj.is_some();
            if let Some(preset) = self.config.model_presets.get(&model.id) {
                self.preset = preset.clone();
                self.active_preset_label = "当前模型预设".to_string();
            } else {
                self.preset = self
                    .config
                    .global_presets
                    .get("默认")
                    .cloned()
                    .unwrap_or_default();
                self.active_preset_label = "默认".to_string();
            }
        }
    }

    fn save_config(&mut self) {
        self.config
            .global_presets
            .insert("默认".to_string(), self.preset.clone());
        self.config.selected_preset = "默认".to_string();
        if let Err(err) = save_config(&self.config) {
            self.warnings.push(format!("保存配置失败: {err}"));
        }
    }

    fn save_model_preset(&mut self) {
        let Some(model) = self.selected_model() else {
            self.warnings.push("请先选择模型".to_string());
            return;
        };
        let model_id = model.id.clone();
        let model_name = model.display_name.clone();
        self.config
            .model_presets
            .insert(model_id, self.preset.clone());
        self.save_app_config();
        self.active_preset_label = "当前模型预设".to_string();
        self.status = format!("已保存模型预设: {}", model_name);
    }

    fn prune_missing_model_presets(&mut self) {
        let valid_ids: std::collections::BTreeSet<String> =
            self.models.iter().map(|model| model.id.clone()).collect();
        let before = self.config.model_presets.len();
        self.config
            .model_presets
            .retain(|model_id, _| valid_ids.contains(model_id));
        if self.config.model_presets.len() != before {
            self.save_app_config();
        }
    }

    fn save_app_config(&mut self) {
        if let Err(err) = save_config(&self.config) {
            self.warnings.push(format!("保存配置失败: {err}"));
        }
    }

    fn add_manual_model(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("GGUF 模型", &["gguf"])
            .pick_file()
        else {
            return;
        };
        let id = path.to_string_lossy().replace('\\', "/").to_lowercase();
        if self.config.manual_models.iter().any(|model| {
            model
                .path
                .to_string_lossy()
                .replace('\\', "/")
                .to_lowercase()
                == id
        }) {
            self.status = "模型已在手动列表中".to_string();
            return;
        }
        self.config.draft_models.retain(|draft_id| draft_id != &id);
        let rel_path = if path.is_absolute() {
            path.to_string_lossy().replace('\\', "/")
        } else {
            path.strip_prefix(&self.config.models_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/")
        };
        self.config
            .model_draft_overrides
            .retain(|_, draft_id| draft_id != &id && draft_id != &rel_path);
        let display_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("manual-model")
            .to_string();
        self.config
            .manual_models
            .push(ManualModel { path, display_name });
        self.save_app_config();
        self.refresh_models();
    }

    fn rename_model(&mut self, id: &str, name: &str) {
        self.config
            .model_aliases
            .insert(id.to_string(), name.trim().to_string());
        self.save_app_config();
        self.refresh_models();
    }

    fn command_args(&self) -> Vec<String> {
        let Some(model) = self.selected_model() else {
            return Vec::new();
        };
        build_args(
            model,
            &self.preset,
            &self.config.models_dir,
            self.use_mm,
            self.config
                .model_draft_overrides
                .get(&model.id)
                .map(String::as_str),
        )
    }

    fn preview(&self) -> String {
        command_preview(&self.command_args(), &self.config.llama_cpp_dir)
    }

    fn export_bat_script(&mut self) {
        if self.selected_model().is_none() {
            self.warnings.push("请先选择模型".to_string());
            return;
        }
        let default_name = self
            .selected_model()
            .map(|model| format!("启动-{}.bat", sanitize_file_name(&model.display_name)))
            .unwrap_or_else(|| "启动模型.bat".to_string());
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("BAT 启动脚本", &["bat"])
            .save_file()
        else {
            return;
        };
        let script = bat_script(&self.command_args(), &self.config.llama_cpp_dir);
        match fs::write(&path, script) {
            Ok(()) => self.status = format!("已导出脚本: {}", path.display()),
            Err(err) => self.warnings.push(format!("导出 bat 失败: {err}")),
        }
    }

    fn model_path(&self, model: &ModelInfo) -> PathBuf {
        let candidate = Path::new(&model.rel_path);
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.config
                .models_dir
                .join(model.rel_path.replace('/', "\\"))
        }
    }

    fn current_draft_model(&self) -> Option<String> {
        let model = self.selected_model()?;
        self.config
            .model_draft_overrides
            .get(&model.id)
            .cloned()
            .or_else(|| model.draft_model.clone())
    }

    fn draft_display_name(&self, rel_path: &str) -> String {
        discover_draft_models(&self.config)
            .into_iter()
            .find(|draft| draft.rel_path == rel_path)
            .map(|draft| draft.display_name)
            .unwrap_or_else(|| rel_path.to_string())
    }

    fn set_model_as_draft(&mut self, model: &ModelInfo) {
        if !self.config.draft_models.contains(&model.id) {
            self.config.draft_models.push(model.id.clone());
        }
        self.config
            .model_draft_overrides
            .retain(|main_id, draft_id| {
                main_id != &model.id && draft_id != &model.id && draft_id != &model.rel_path
            });
        self.save_app_config();
        self.refresh_models();
        self.status = format!("已设置为 draft 草稿模型: {}", model.display_name);
    }

    fn open_model_in_explorer(&mut self, model: &ModelInfo) {
        let path = self.model_path(model);
        let result = Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn();
        if let Err(err) = result {
            self.warnings.push(format!("打开文件资源管理器失败: {err}"));
        }
    }

    fn begin_rename_model(&mut self, model: &ModelInfo) {
        self.rename_popup_model = Some(model.id.clone());
        self.rename_popup_text = model.display_name.clone();
    }

    fn move_model_before(&mut self, dragged_id: &str, target_id: &str) {
        if dragged_id == target_id {
            return;
        }
        let mut order: Vec<String> = self.models.iter().map(|model| model.id.clone()).collect();
        order.retain(|id| id != dragged_id);
        let target_idx = order
            .iter()
            .position(|id| id == target_id)
            .unwrap_or(order.len());
        order.insert(target_idx, dragged_id.to_string());
        self.config.model_order = order;
        self.save_app_config();
        self.models = discover_configured_models(&self.config);
        self.selected_index = self.models.iter().position(|model| model.id == dragged_id);
    }

    fn check_paths(&mut self) -> bool {
        self.warnings.clear();
        let server = llama_server_path(&self.config.llama_cpp_dir);
        if !server.is_file() {
            self.warnings
                .push(format!("找不到 llama-server.exe: {}", server.display()));
        }
        if !self.config.models_dir.is_dir() {
            self.warnings.push(format!(
                "models 目录不存在: {}",
                self.config.models_dir.display()
            ));
        }
        self.warnings.is_empty()
    }

    fn port_in_use(&self) -> bool {
        let host = if self.preset.host == "0.0.0.0" {
            "127.0.0.1"
        } else {
            self.preset.host.as_str()
        };
        let Ok(port) = self.preset.port.parse::<u16>() else {
            return false;
        };
        let Ok(addr) = format!("{host}:{port}").parse::<SocketAddr>() else {
            return false;
        };
        TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
    }

    fn start(&mut self) {
        if self.server.is_some() {
            return;
        }
        if self.selected_model().is_none() {
            self.warnings.push("请先选择模型".to_string());
            return;
        }
        if !self.check_paths() {
            return;
        }
        if self.port_in_use() {
            self.warnings.push("端口已被占用，请修改端口".to_string());
            return;
        }
        let args = self.command_args();
        match server::start_server(&args, &self.config.llama_cpp_dir) {
            Ok(process) => {
                self.logs.clear();
                self.logs.push("启动 llama-server...".to_string());
                self.server = Some(process);
                self.status = "启动中".to_string();
                self.show_log_window = true;
            }
            Err(err) => self.warnings.push(format!("启动失败: {err}")),
        }
    }

    fn stop(&mut self) {
        if let Some(mut server) = self.server.take() {
            server::stop_process(&mut server.child);
            self.status = "已发送停止命令".to_string();
            self.logs.push("已发送停止命令".to_string());
        }
    }

    fn poll_server(&mut self) {
        let mut clear_server = false;
        if let Some(server) = self.server.as_mut() {
            while let Ok(event) = server.rx.try_recv() {
                match event {
                    ServerEvent::Log(line) => {
                        let lower = line.to_lowercase();
                        if lower.contains("server is listening") || lower.contains("listening on") {
                            self.status = "已监听".to_string();
                        }
                        self.logs.push(line);
                        if self.logs.len() > 500 {
                            self.logs.remove(0);
                        }
                    }
                }
            }
            if let Ok(Some(status)) = server.child.try_wait() {
                let code = status.code().unwrap_or(-1);
                self.logs.push(format!("进程已退出: {code}"));
                self.status = if code == 0 {
                    "空闲".to_string()
                } else {
                    format!("异常退出: {code}")
                };
                clear_server = true;
            }
        }
        if clear_server {
            self.server = None;
        }
    }

    fn path_row(ui: &mut egui::Ui, label: &str, value: &mut PathBuf) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.add_sized(
                [92.0, 22.0],
                egui::Label::new(RichText::new(label).color(GRAPHITE).size(13.0)),
            );
            let mut text = value.to_string_lossy().to_string();
            if ui
                .add_sized([390.0, 26.0], egui::TextEdit::singleline(&mut text))
                .changed()
            {
                *value = PathBuf::from(text);
                changed = true;
            }
            if Self::small_button(ui, "选择").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    *value = path;
                    changed = true;
                }
            }
        });
        changed
    }

    fn help_body(ui: &mut egui::Ui, help: ParamHelp) {
        ui.set_max_width(360.0);
        ui.label(RichText::new(help.purpose).color(INK).size(14.0));
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!("CPU: {}", help.cpu))
                .color(GRAPHITE)
                .size(13.0),
        );
        ui.label(
            RichText::new(format!("内存: {}", help.memory))
                .color(GRAPHITE)
                .size(13.0),
        );
        ui.label(
            RichText::new(format!("显存: {}", help.vram))
                .color(GRAPHITE)
                .size(13.0),
        );
    }

    fn help_button(ui: &mut egui::Ui, help: ParamHelp, popup: &mut Option<ParamHelp>) {
        let response = ui.add(
            egui::Button::new(RichText::new("?").color(INK).size(13.0).strong())
                .fill(SILVER_MIST)
                .stroke(Stroke::NONE)
                .corner_radius(CornerRadius::same(11))
                .min_size(egui::vec2(22.0, 22.0)),
        );
        let clicked = response.clicked();
        response.on_hover_ui(|ui| Self::help_body(ui, help));
        if clicked {
            *popup = Some(help);
        }
    }

    fn param_text(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut String,
        help: ParamHelp,
        popup: &mut Option<ParamHelp>,
    ) {
        ui.horizontal(|ui| {
            ui.add_sized(
                [82.0, 22.0],
                egui::Label::new(RichText::new(label).color(GRAPHITE).size(13.0)),
            );
            Self::help_button(ui, help, popup);
            ui.add_sized([148.0, 24.0], egui::TextEdit::singleline(value));
        });
    }

    fn cache_combo(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut String,
        help: ParamHelp,
        popup: &mut Option<ParamHelp>,
    ) {
        const TYPES: &[&str] = &[
            "f32", "f16", "bf16", "q8_0", "q4_0", "q4_1", "iq4_nl", "q5_0", "q5_1",
        ];
        let selected = if value.trim().is_empty() {
            "默认".to_string()
        } else {
            value.clone()
        };
        ui.horizontal(|ui| {
            ui.add_sized(
                [82.0, 22.0],
                egui::Label::new(RichText::new(label).color(GRAPHITE).size(13.0)),
            );
            Self::help_button(ui, help, popup);
            egui::ComboBox::from_id_salt(label)
                .width(148.0)
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    ui.selectable_value(value, String::new(), "默认");
                    for ty in TYPES {
                        ui.selectable_value(value, (*ty).to_string(), *ty);
                    }
                });
        });
    }

    fn param_checkbox(
        ui: &mut egui::Ui,
        value: &mut bool,
        label: &str,
        help: ParamHelp,
        popup: &mut Option<ParamHelp>,
    ) {
        ui.horizontal(|ui| {
            ui.add_sized([226.0, 24.0], egui::Checkbox::new(value, label));
            Self::help_button(ui, help, popup);
        });
    }

    fn show_help_popup(&mut self, ctx: &egui::Context) {
        let Some(help) = self.help_popup else {
            return;
        };
        let mut open = true;
        let mut close_clicked = false;
        egui::Window::new(help.title)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                Self::help_body(ui, help);
                ui.add_space(10.0);
                if Self::small_button(ui, "关闭").clicked() {
                    close_clicked = true;
                }
            });
        if !open || close_clicked {
            self.help_popup = None;
        }
    }

    fn show_rename_popup(&mut self, ctx: &egui::Context) {
        let Some(model_id) = self.rename_popup_model.clone() else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        egui::Window::new("修改模型显示名称")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(RichText::new("显示名称").color(GRAPHITE).size(13.0));
                ui.add_sized(
                    [320.0, 28.0],
                    egui::TextEdit::singleline(&mut self.rename_popup_text),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if Self::small_button(ui, "应用重命名").clicked() {
                        apply = true;
                    }
                    if Self::small_button(ui, "取消").clicked() {
                        cancel = true;
                    }
                });
            });
        if apply {
            let name = self.rename_popup_text.clone();
            self.rename_model(&model_id, &name);
            self.rename_popup_model = None;
            self.rename_popup_text.clear();
        } else if cancel || !open {
            self.rename_popup_model = None;
            self.rename_popup_text.clear();
        }
    }

    fn show_draft_picker(&mut self, ctx: &egui::Context) {
        if !self.show_draft_picker {
            return;
        }
        let Some(model) = self.selected_model().cloned() else {
            self.show_draft_picker = false;
            return;
        };
        let draft_models = discover_draft_models(&self.config);
        let mut open = true;
        let mut close_clicked = false;
        let mut selected: Option<DraftModelInfo> = None;
        let mut clear = false;
        egui::Window::new("添加/修改 draft 草稿模型")
            .collapsible(false)
            .resizable(true)
            .default_width(430.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("选择后会为当前主模型添加或替换 --spec-draft-model。")
                        .color(GRAPHITE)
                        .size(13.0),
                );
                if let Some(current) = self.current_draft_model() {
                    ui.label(
                        RichText::new(format!("当前: {}", self.draft_display_name(&current)))
                            .color(GRAPHITE)
                            .size(13.0),
                    );
                }
                ui.add_space(8.0);
                if draft_models.is_empty() {
                    ui.label(RichText::new("未发现 draft / MTP 草稿模型。").color(CAUTION));
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(260.0)
                        .show(ui, |ui| {
                            for draft in &draft_models {
                                let current = self
                                    .config
                                    .model_draft_overrides
                                    .get(&model.id)
                                    .is_some_and(|rel| rel == &draft.rel_path)
                                    || model.draft_model.as_ref() == Some(&draft.rel_path);
                                let mut label = draft.display_name.clone();
                                if !draft.size_label.is_empty() {
                                    label.push_str("  ");
                                    label.push_str(&draft.size_label);
                                }
                                if ui.selectable_label(current, label).clicked() {
                                    selected = Some(draft.clone());
                                }
                                ui.label(RichText::new(&draft.rel_path).color(GRAPHITE).size(12.0));
                                ui.add_space(4.0);
                            }
                        });
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if Self::small_button(ui, "清除当前 draft").clicked() {
                        clear = true;
                    }
                    if Self::small_button(ui, "关闭").clicked() {
                        close_clicked = true;
                    }
                });
            });
        if let Some(draft) = selected {
            self.config
                .model_draft_overrides
                .insert(model.id.clone(), draft.rel_path.clone());
            self.save_app_config();
            self.status = format!("已设置 draft 草稿模型: {}", draft.display_name);
            self.show_draft_picker = false;
        } else if clear {
            self.config.model_draft_overrides.remove(&model.id);
            self.save_app_config();
            self.status = "已清除当前模型的 draft 草稿模型设置".to_string();
            self.show_draft_picker = false;
        } else if close_clicked || !open {
            self.show_draft_picker = false;
        }
    }

    fn show_server_log_window(&mut self, ctx: &egui::Context) {
        if !self.show_log_window {
            return;
        }
        let mut open = self.show_log_window;
        egui::Window::new("运行日志")
            .default_width(760.0)
            .default_height(420.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.status_pill());
                    if Self::small_button(ui, "复制日志").clicked() {
                        ui.ctx().copy_text(self.logs.join("\n"));
                    }
                    if Self::small_button(ui, "清空").clicked() {
                        self.logs.clear();
                    }
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.logs.is_empty() {
                            ui.label(RichText::new("暂无日志").color(GRAPHITE).size(14.0));
                        } else {
                            for line in &self.logs {
                                ui.monospace(line);
                            }
                        }
                    });
            });
        self.show_log_window = open;
    }

    fn preset_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_top(|ui| {
            ui.add_sized(
                [68.0, 22.0],
                egui::Label::new(RichText::new("参数预设").color(GRAPHITE).size(13.0)),
            );
            egui::ComboBox::from_id_salt("preset")
                .width(158.0)
                .selected_text(&self.active_preset_label)
                .show_ui(ui, |ui| {
                    let default_selected = self.active_preset_label == "默认";
                    if ui.selectable_label(default_selected, "默认").clicked() {
                        self.preset = self
                            .config
                            .global_presets
                            .get("默认")
                            .cloned()
                            .unwrap_or_default();
                        self.active_preset_label = "默认".to_string();
                    }
                    if let Some(model) = self.selected_model() {
                        if self.config.model_presets.contains_key(&model.id) {
                            let model_selected = self.active_preset_label == "当前模型预设";
                            if ui
                                .selectable_label(model_selected, "当前模型预设")
                                .clicked()
                            {
                                if let Some(preset) = self.config.model_presets.get(&model.id) {
                                    self.preset = preset.clone();
                                    self.active_preset_label = "当前模型预设".to_string();
                                }
                            }
                        }
                    }
                });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_sized(
                [104.0, 24.0],
                egui::Checkbox::new(&mut self.use_mm, "启用多模态"),
            );
            Self::help_button(
                ui,
                help(
                    "启用多模态",
                    "选择模型绑定的 mmproj 文件并向 llama-server 传入 --mmproj，用于图像理解。",
                    "图像编码阶段会增加 CPU 调度压力，文本生成阶段影响较小。",
                    "图像 token 会占用上下文和 KV 缓存，图片越多内存需求越高。",
                    "视觉编码和更长的图像上下文会增加显存占用。",
                ),
                &mut self.help_popup,
            );
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if Self::small_button(ui, "保存默认").clicked() {
                self.save_config();
            }
            if Self::small_button(ui, "保存当前模型").clicked() {
                self.save_model_preset();
            }
            if Self::small_button(ui, "draft 草稿模型").clicked() {
                if self.selected_model().is_some() {
                    self.show_draft_picker = true;
                } else {
                    self.warnings.push("请先选择模型".to_string());
                }
            }
        });
    }

    fn action_bar_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let button_size = egui::vec2(168.0, 52.0);
            let start_enabled = self.server.is_none();
            let stop_enabled = self.server.is_some();

            if ui
                .add_enabled(
                    start_enabled,
                    egui::Button::new(RichText::new("启动").color(SNOW).size(18.0).strong())
                        .fill(AZURE)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(28))
                        .min_size(button_size),
                )
                .clicked()
            {
                self.start();
            }
            if ui
                .add_enabled(
                    stop_enabled,
                    egui::Button::new(RichText::new("停止").color(INK).size(18.0).strong())
                        .fill(SILVER_MIST)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(28))
                        .min_size(button_size),
                )
                .clicked()
            {
                self.stop();
            }
        });
    }

    fn small_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
        ui.add(
            egui::Button::new(RichText::new(text).color(INK).size(13.0))
                .fill(SILVER_MIST)
                .stroke(Stroke::NONE)
                .corner_radius(CornerRadius::same(18))
                .min_size(egui::vec2(54.0, 24.0)),
        )
    }

    fn text_color(&self) -> Color32 {
        rgba_to_color(self.config.appearance.panel_text)
    }

    fn weak_text_color(&self) -> Color32 {
        rgba_to_color(self.config.appearance.weak_text)
    }

    fn title_text(&self, text: impl Into<String>, size: f32) -> RichText {
        let rich = RichText::new(text.into())
            .color(self.text_color())
            .size(size);
        if self.config.appearance.bold_text {
            rich.strong()
        } else {
            rich
        }
    }

    fn card_frame_with_border(border: Color32) -> egui::Frame {
        egui::Frame::new()
            .fill(SNOW)
            .stroke(Stroke::new(1.0, border))
            .corner_radius(CornerRadius::same(28))
            .inner_margin(egui::Margin::same(16))
    }

    fn fixed_size_card(
        ui: &mut egui::Ui,
        width: f32,
        height: f32,
        border: Color32,
        add_contents: impl FnOnce(&mut egui::Ui),
    ) {
        Self::card_frame_with_border(border).show(ui, |ui| {
            ui.set_min_width(width - 34.0);
            ui.set_max_width(width - 34.0);
            ui.set_min_height(height - 34.0);
            ui.set_max_height(height - 34.0);
            add_contents(ui);
        });
    }

    fn recessed_frame() -> egui::Frame {
        egui::Frame::new()
            .fill(FOG)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(18))
            .inner_margin(egui::Margin::same(12))
    }

    fn status_pill(&self) -> RichText {
        RichText::new(format!("状态: {}", self.status))
            .color(GRAPHITE)
            .size(13.0)
    }

    fn color_setting(ui: &mut egui::Ui, label: &str, value: &mut [u8; 4]) -> bool {
        let mut color = rgba_to_color(*value);
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.add_sized(
                [116.0, 22.0],
                egui::Label::new(RichText::new(label).color(GRAPHITE).size(13.0)),
            );
            if egui::color_picker::color_edit_button_srgba(
                ui,
                &mut color,
                egui::color_picker::Alpha::Opaque,
            )
            .changed()
            {
                *value = color_to_rgba(color);
                changed = true;
            }
        });
        changed
    }

    fn menu_bar_ui(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("外观", |ui| {
                let mut changed = false;
                changed |=
                    Self::color_setting(ui, "顶部边框", &mut self.config.appearance.top_border);
                changed |=
                    Self::color_setting(ui, "模型栏边框", &mut self.config.appearance.model_border);
                changed |= Self::color_setting(
                    ui,
                    "参数栏边框",
                    &mut self.config.appearance.preset_border,
                );
                changed |= Self::color_setting(
                    ui,
                    "预览栏边框",
                    &mut self.config.appearance.preview_border,
                );
                ui.separator();
                changed |=
                    Self::color_setting(ui, "主文字颜色", &mut self.config.appearance.panel_text);
                changed |=
                    Self::color_setting(ui, "辅助文字颜色", &mut self.config.appearance.weak_text);
                changed |= ui
                    .checkbox(&mut self.config.appearance.bold_text, "标题/模型名称加粗")
                    .changed();
                ui.separator();
                if Self::small_button(ui, "恢复默认外观").clicked() {
                    self.config.appearance = Default::default();
                    changed = true;
                }
                if changed {
                    self.save_app_config();
                }
            });
        });
    }

    fn eye_badge(ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 18.0), egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 9.0, Color32::from_rgb(221, 220, 140));
        let center = rect.center();
        let stroke = Stroke::new(1.4, INK);
        let left = rect.left() + 5.0;
        let right = rect.right() - 5.0;
        let top = center.y - 3.4;
        let bottom = center.y + 3.4;
        painter.line_segment(
            [egui::pos2(left, center.y), egui::pos2(center.x, top)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(center.x, top), egui::pos2(right, center.y)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(left, center.y), egui::pos2(center.x, bottom)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(center.x, bottom), egui::pos2(right, center.y)],
            stroke,
        );
        painter.circle_filled(center, 2.4, INK);
    }

    fn params_ui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("基础")
            .default_open(true)
            .show(ui, |ui| {
                Self::param_text(
                    ui,
                    "监听地址",
                    &mut self.preset.host,
                    help(
                        "监听地址",
                        "设置 llama-server 绑定的网络地址。127.0.0.1 仅本机访问，0.0.0.0 允许局域网访问。",
                        "几乎无影响。",
                        "几乎无影响。",
                        "无影响。",
                    ),
                    &mut self.help_popup,
                );
                Self::param_text(
                    ui,
                    "端口",
                    &mut self.preset.port,
                    help(
                        "端口",
                        "设置 HTTP 服务端口，客户端通过这个端口连接本地模型服务。",
                        "几乎无影响。",
                        "几乎无影响。",
                        "无影响。",
                    ),
                    &mut self.help_popup,
                );
                Self::param_text(
                    ui,
                    "超时秒数",
                    &mut self.preset.timeout,
                    help(
                        "超时秒数",
                        "设置请求超时时间。长请求、长上下文或慢速模型可以适当增大。",
                        "不会直接增加 CPU 占用，只会允许请求运行更久。",
                        "长时间保留请求状态时会略增内存占用。",
                        "生成持续时间更长时，显存占用会保持更久。",
                    ),
                    &mut self.help_popup,
                );
                Self::param_text(
                    ui,
                    "模型别名",
                    &mut self.preset.alias,
                    help(
                        "模型别名",
                        "设置 API 返回和调用时使用的模型名称，便于兼容 OpenAI 风格客户端。",
                        "无影响。",
                        "无影响。",
                        "无影响。",
                    ),
                    &mut self.help_popup,
                );
            });
        egui::CollapsingHeader::new("性能")
            .default_open(true)
            .show(ui, |ui| {
                Self::param_text(
                    ui,
                    "GPU 层数",
                    &mut self.preset.ngl,
                    help(
                        "GPU 层数",
                        "对应 --gpu-layers（兼容短名 -ngl），控制卸载到 GPU 的模型层数。数值越高，越依赖显卡。",
                        "通常会降低 CPU 推理压力。",
                        "CPU 内存中保留的权重可能减少，但仍需装载模型和缓存。",
                        "显存占用显著增加，层数过高可能爆显存。",
                    ),
                    &mut self.help_popup,
                );
                Self::param_text(
                    ui,
                    "CPU MoE 层数",
                    &mut self.preset.n_cpu_moe,
                    help(
                        "CPU MoE 层数",
                        "对应 --n-cpu-moe，控制 MoE 专家层放在 CPU 的数量，用于在显存不足时分担。",
                        "会增加 CPU 计算和内存带宽压力。",
                        "会增加系统内存占用。",
                        "可降低显存压力，但可能降低速度。",
                    ),
                    &mut self.help_popup,
                );
                Self::param_text(
                    ui,
                    "线程数",
                    &mut self.preset.threads,
                    help(
                        "线程数",
                        "对应 --threads（兼容短名 -t），设置 CPU 推理线程数。通常接近物理核心数或性能核心数更稳。",
                        "直接影响 CPU 占用，过高可能抢占系统资源。",
                        "影响较小。",
                        "无直接影响。",
                    ),
                    &mut self.help_popup,
                );
                Self::param_text(
                    ui,
                    "批大小",
                    &mut self.preset.batch_size,
                    help(
                        "批大小",
                        "对应 --batch-size（兼容短名 -b），影响 prompt 处理和批量 token 处理规模。",
                        "较大批次可提高吞吐，但会增加调度压力。",
                        "会增加临时缓冲和 KV 相关内存需求。",
                        "GPU 参与时会明显增加显存峰值。",
                    ),
                    &mut self.help_popup,
                );
                Self::param_text(
                    ui,
                    "微批大小",
                    &mut self.preset.ubatch_size,
                    help(
                        "微批大小",
                        "对应 --ubatch-size（兼容短名 -ub），将大批次拆成更小的微批执行，用于平衡速度和显存峰值。",
                        "较小微批可能增加调度次数。",
                        "通常可降低峰值内存压力。",
                        "降低该值通常能降低显存峰值，但可能变慢。",
                    ),
                    &mut self.help_popup,
                );
                Self::param_text(
                    ui,
                    "并发槽位",
                    &mut self.preset.parallel,
                    help(
                        "并发槽位",
                        "对应 --parallel（兼容短名 -np），设置服务端可并行处理的请求槽位数量。",
                        "并发越高，CPU 调度和采样压力越高。",
                        "每个槽位会分配上下文和缓存，内存占用可能成倍增加。",
                        "KV 缓存在 GPU 时，显存也会随并发明显增加。",
                    ),
                    &mut self.help_popup,
                );
            });
        egui::CollapsingHeader::new("上下文/KV")
            .default_open(true)
            .show(ui, |ui| {
                Self::param_text(
                    ui,
                    "上下文长度",
                    &mut self.preset.ctx_size,
                    help(
                        "上下文长度",
                        "对应 --ctx-size（兼容短名 -c），设置可保留的最大 token 上下文。长文档和多轮对话需要更高值。",
                        "长上下文会增加注意力计算量，生成后期更吃 CPU/GPU。",
                        "KV 缓存随上下文长度线性增长。",
                        "KV offload 开启时，显存会随上下文长度线性增长。",
                    ),
                    &mut self.help_popup,
                );
                Self::cache_combo(
                    ui,
                    "K 缓存类型",
                    &mut self.preset.cache_type_k,
                    help(
                        "K 缓存类型",
                        "对应 --cache-type-k，设置 Key KV 缓存精度。默认表示不传该参数。",
                        "低精度可能略增解码转换开销，但通常影响较小。",
                        "低精度可降低 KV 缓存内存占用。",
                        "KV 在 GPU 时，低精度可显著降低显存占用。",
                    ),
                    &mut self.help_popup,
                );
                Self::cache_combo(
                    ui,
                    "V 缓存类型",
                    &mut self.preset.cache_type_v,
                    help(
                        "V 缓存类型",
                        "对应 --cache-type-v，设置 Value KV 缓存精度。默认表示不传该参数。",
                        "低精度可能影响速度或质量，具体取决于模型和硬件。",
                        "低精度可降低 KV 缓存内存占用。",
                        "KV 在 GPU 时，低精度可显著降低显存占用。",
                    ),
                    &mut self.help_popup,
                );
            });
        egui::CollapsingHeader::new("GPU / 推测 / 多模态").show(ui, |ui| {
            Self::param_text(
                ui,
                "切分模式",
                &mut self.preset.split_mode,
                help(
                    "切分模式",
                    "对应 --split-mode，控制多 GPU 场景下模型张量或层如何切分。",
                    "多 GPU 通信不当时可能增加 CPU 调度等待。",
                    "影响较小，主要取决于模型装载策略。",
                    "决定显存如何分摊到多张 GPU，设置不当会导致某张卡爆显存。",
                ),
                &mut self.help_popup,
            );
            Self::param_text(
                ui,
                "张量切分",
                &mut self.preset.tensor_split,
                help(
                    "张量切分",
                    "对应 --tensor-split，手动指定多 GPU 显存或计算分配比例。",
                    "通常无直接 CPU 收益，配置复杂时可能增加调度开销。",
                    "影响较小。",
                    "用于控制每张 GPU 承担的显存比例，适合不同显存容量的多卡。",
                ),
                &mut self.help_popup,
            );
            Self::param_text(
                ui,
                "主 GPU",
                &mut self.preset.main_gpu,
                help(
                    "主 GPU",
                    "对应 --main-gpu，指定主 GPU 编号，常用于多 GPU 或切分模式。",
                    "几乎无影响。",
                    "几乎无影响。",
                    "会影响主卡承担的缓存和计算压力。",
                ),
                &mut self.help_popup,
            );
            Self::param_text(
                ui,
                "设备",
                &mut self.preset.device,
                help(
                    "设备",
                    "对应 --device，指定 llama.cpp 使用的设备。留空时使用后端默认选择。",
                    "选 CPU 设备会显著增加 CPU 负载。",
                    "CPU 路径通常更多依赖系统内存。",
                    "选择 GPU 设备会增加对应显卡显存占用。",
                ),
                &mut self.help_popup,
            );
            Self::param_text(
                ui,
                "推测类型",
                &mut self.preset.spec_type,
                help(
                    "推测类型",
                    "对应 --spec-type，设置 speculative decoding 的策略，需要草稿模型配合。",
                    "会增加草稿模型的额外计算，但可能减少主模型等待。",
                    "需要额外装载草稿模型和缓存。",
                    "草稿模型放 GPU 时会额外占用显存。",
                ),
                &mut self.help_popup,
            );
            Self::param_text(
                ui,
                "草稿最大 N",
                &mut self.preset.spec_draft_n_max,
                help(
                    "草稿最大 N",
                    "对应 --spec-draft-n-max，限制每轮最多生成多少草稿 token。",
                    "数值越大，草稿模型计算越多。",
                    "草稿缓存和临时缓冲会增加。",
                    "草稿模型在 GPU 时会增加显存峰值。",
                ),
                &mut self.help_popup,
            );
            Self::param_text(
                ui,
                "草稿最小 N",
                &mut self.preset.spec_draft_n_min,
                help(
                    "草稿最小 N",
                    "对应 --spec-draft-n-min，设置每轮至少尝试的草稿 token 数。",
                    "提高后会增加草稿模型固定计算量。",
                    "影响较小。",
                    "草稿模型在 GPU 时略增显存压力。",
                ),
                &mut self.help_popup,
            );
            Self::param_text(
                ui,
                "草稿最小概率",
                &mut self.preset.spec_draft_p_min,
                help(
                    "草稿最小概率",
                    "对应 --spec-draft-p-min，低于该概率的草稿 token 会被过滤。",
                    "影响草稿接受率，间接影响主模型和草稿模型工作量。",
                    "几乎无直接影响。",
                    "几乎无直接影响。",
                ),
                &mut self.help_popup,
            );
            Self::param_text(
                ui,
                "草稿切分概率",
                &mut self.preset.spec_draft_p_split,
                help(
                    "草稿切分概率",
                    "对应 --spec-draft-p-split，控制草稿 token 分支或切分阈值。",
                    "会影响推测解码的尝试次数和接受效率。",
                    "几乎无直接影响。",
                    "几乎无直接影响。",
                ),
                &mut self.help_popup,
            );
            Self::param_text(
                ui,
                "图像最小 token",
                &mut self.preset.image_min_tokens,
                help(
                    "图像最小 token",
                    "对应 --image-min-tokens，多模态时限制图像至少占用的 token 数。",
                    "图像 token 更多会增加后续注意力计算。",
                    "会增加上下文和 KV 缓存占用。",
                    "KV 在 GPU 时会增加显存占用。",
                ),
                &mut self.help_popup,
            );
            Self::param_text(
                ui,
                "图像最大 token",
                &mut self.preset.image_max_tokens,
                help(
                    "图像最大 token",
                    "对应 --image-max-tokens，多模态时限制图像最多占用的 token 数。",
                    "上限越高，复杂图片处理和后续推理成本越高。",
                    "上限越高，最坏情况下上下文和 KV 缓存占用越高。",
                    "KV 在 GPU 时，较高上限会增加显存峰值。",
                ),
                &mut self.help_popup,
            );
        });
        egui::CollapsingHeader::new("开关").show(ui, |ui| {
            Self::param_checkbox(
                ui,
                &mut self.preset.web_ui,
                "启用 llama.cpp Web UI",
                help(
                    "启用 llama.cpp Web UI",
                    "传入 --ui 或 --no-ui，控制 llama-server 是否提供内置网页界面。",
                    "Web UI 本身占用很低。",
                    "会增加少量服务端资源。",
                    "无影响。",
                ),
                &mut self.help_popup,
            );
            Self::param_checkbox(
                ui,
                &mut self.preset.log_timestamps,
                "日志时间戳",
                help(
                    "日志时间戳",
                    "传入 --log-timestamps 或 --no-log-timestamps，控制日志是否带时间。",
                    "几乎无影响。",
                    "日志文本会略微变长。",
                    "无影响。",
                ),
                &mut self.help_popup,
            );
            Self::param_checkbox(
                ui,
                &mut self.preset.offline,
                "离线模式",
                help(
                    "离线模式",
                    "传入 --offline，禁止服务端尝试在线行为，适合纯本地使用。",
                    "几乎无影响。",
                    "几乎无影响。",
                    "无影响。",
                ),
                &mut self.help_popup,
            );
            Self::param_checkbox(
                ui,
                &mut self.preset.verbose,
                "详细日志",
                help(
                    "详细日志",
                    "传入 --verbose，输出更详细的运行信息，便于排查参数和性能问题。",
                    "日志量变大时会有很小的 CPU 开销。",
                    "GUI 日志缓存会占用更多内存。",
                    "无影响。",
                ),
                &mut self.help_popup,
            );
            Self::param_checkbox(
                ui,
                &mut self.preset.kv_offload,
                "KV 缓存 offload",
                help(
                    "KV 缓存 offload",
                    "传入 --kv-offload 或 --no-kv-offload，控制 KV 缓存是否尽量放到 GPU。",
                    "开启后可减少 CPU 内存访问压力。",
                    "关闭时更多 KV 留在系统内存。",
                    "开启后显存占用会随上下文和并发明显增加。",
                ),
                &mut self.help_popup,
            );
            Self::param_checkbox(
                ui,
                &mut self.preset.mlock,
                "锁定内存",
                help(
                    "锁定内存",
                    "传入 --mlock，尽量锁定模型内存，减少被系统换出导致的卡顿。",
                    "可减少换页造成的 CPU 等待。",
                    "会让模型占用的系统内存更难被系统回收。",
                    "无直接影响。",
                ),
                &mut self.help_popup,
            );
            Self::param_checkbox(
                ui,
                &mut self.preset.mmap,
                "内存映射 mmap",
                help(
                    "内存映射 mmap",
                    "传入 --mmap 或 --no-mmap，控制是否用内存映射方式加载模型文件。",
                    "开启通常加载更快，运行中影响取决于磁盘和缓存命中。",
                    "开启可减少一次性读入压力，但仍会占用文件缓存。",
                    "无直接影响。",
                ),
                &mut self.help_popup,
            );
            Self::param_checkbox(
                ui,
                &mut self.preset.kv_unified,
                "统一 KV buffer",
                help(
                    "统一 KV buffer",
                    "传入 --kv-unified，使用统一 KV buffer 策略。",
                    "可能改善部分场景的调度一致性。",
                    "会改变 KV 缓存分配方式，具体占用取决于模型和并发。",
                    "KV 在 GPU 时会改变显存分配方式。",
                ),
                &mut self.help_popup,
            );
            Self::param_checkbox(
                ui,
                &mut self.preset.swa_full,
                "SWA 全尺寸缓存",
                help(
                    "SWA 全尺寸缓存",
                    "传入 --swa-full，为滑动窗口注意力使用全尺寸缓存。",
                    "可能减少特殊缓存路径的调度差异。",
                    "会增加 KV 缓存内存占用。",
                    "KV 在 GPU 时会增加显存占用。",
                ),
                &mut self.help_popup,
            );
            Self::param_checkbox(
                ui,
                &mut self.preset.cpu_moe,
                "全部 MoE 放 CPU",
                help(
                    "全部 MoE 放 CPU",
                    "传入 --cpu-moe，将 MoE 专家计算放到 CPU，常用于显存不足时。",
                    "会显著增加 CPU 计算和内存带宽压力。",
                    "会增加系统内存占用。",
                    "可明显降低 MoE 模型的显存压力。",
                ),
                &mut self.help_popup,
            );
        });
        egui::CollapsingHeader::new("额外参数").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("每行一个参数");
                Self::help_button(
                    ui,
                    help(
                        "额外参数",
                        "逐行追加到启动命令末尾，用于临时测试本界面尚未提供的 llama-server 参数。",
                        "取决于你追加的参数。",
                        "取决于你追加的参数。",
                        "取决于你追加的参数。",
                    ),
                    &mut self.help_popup,
                );
            });
            ui.add_sized(
                [360.0, 90.0],
                egui::TextEdit::multiline(&mut self.preset.extra_args),
            );
        });
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_server();

        egui::TopBottomPanel::top("top")
            .frame(
                egui::Frame::new()
                    .fill(SNOW)
                    .stroke(Stroke::new(
                        1.0,
                        rgba_to_color(self.config.appearance.top_border),
                    ))
                    .inner_margin(egui::Margin::symmetric(16, 12)),
            )
            .show(ctx, |ui| {
                self.menu_bar_ui(ui);
                ui.add_space(6.0);
                let top_left_width = 660.0;
                let top_gap = 10.0;
                let preview_right_edge = top_left_width + top_gap + (390.0 * 2.0 + 10.0);
                let export_button_right_inset = 120.0;
                let action_right_edge = preview_right_edge - export_button_right_inset;
                let action_width = action_right_edge - top_left_width - top_gap;
                ui.allocate_ui_with_layout(
                    egui::vec2(action_right_edge, 112.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading(self.title_text("本地 AI 启动器", 24.0));
                                    ui.add_space(10.0);
                                    ui.label(
                                        RichText::new("llama.cpp 模型运行控制台")
                                            .color(self.weak_text_color())
                                            .size(13.0),
                                    );
                                });
                                ui.add_space(8.0);
                                ui.set_width(660.0);
                                let mut changed = false;
                                changed |= Self::path_row(
                                    ui,
                                    "llama.cpp 目录",
                                    &mut self.config.llama_cpp_dir,
                                );
                                changed |=
                                    Self::path_row(ui, "models 目录", &mut self.config.models_dir);
                                ui.horizontal(|ui| {
                                    if Self::small_button(ui, "保存路径").clicked() || changed {
                                        let _ = save_config(&self.config);
                                    }
                                    if Self::small_button(ui, "刷新模型").clicked() {
                                        self.refresh_models();
                                    }
                                    ui.add_space(8.0);
                                    ui.label(self.status_pill());
                                });
                            });
                            ui.add_space(top_gap);
                            ui.allocate_ui_with_layout(
                                egui::vec2(action_width, 58.0),
                                egui::Layout::right_to_left(egui::Align::TOP),
                                |ui| {
                                    self.action_bar_ui(ui);
                                },
                            );
                        });
                    },
                );
            });

        egui::SidePanel::left("models")
            .resizable(true)
            .default_width(520.0)
            .width_range(420.0..=680.0)
            .frame(
                egui::Frame::new()
                    .fill(FOG)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| {
                let model_card_height = ui.available_height().max(390.0);
                Self::card_frame_with_border(rgba_to_color(self.config.appearance.model_border))
                    .show(ui, |ui| {
                        ui.set_min_height(model_card_height - 34.0);
                        ui.set_max_height(model_card_height - 34.0);
                        ui.heading(self.title_text("模型", 22.0));
                        ui.horizontal(|ui| {
                            if Self::small_button(ui, "手动添加模型").clicked() {
                                self.add_manual_model();
                            }
                            ui.label(
                                RichText::new(format!("{} 个可用", self.models.len()))
                                    .color(self.weak_text_color())
                                    .size(14.0),
                            );
                        });
                        if let Some(model) = self.selected_model() {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(format!(
                                    "已选: {}    大小: {}    视觉: {}    draft: {}",
                                    model.display_name,
                                    if model.size_label.is_empty() {
                                        "-"
                                    } else {
                                        model.size_label.as_str()
                                    },
                                    if model.mmproj.is_some() { "有" } else { "无" },
                                    self.current_draft_model()
                                        .as_deref()
                                        .map(|draft| self.draft_display_name(draft))
                                        .unwrap_or_else(|| "-".to_string())
                                ))
                                .color(self.weak_text_color())
                                .size(13.0),
                            );
                        } else {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(
                                    "请选择一个 GGUF 主模型。右键模型可重命名或打开所在位置。",
                                )
                                .color(self.weak_text_color())
                                .size(13.0),
                            );
                        }
                        ui.add_space(6.0);
                        ui.add_sized(
                            [ui.available_width(), 28.0],
                            egui::TextEdit::singleline(&mut self.search).hint_text("搜索模型"),
                        );
                        ui.add_space(5.0);
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let needle = self.search.to_lowercase();
                            let mut select_idx = None;
                            let mut rename_model: Option<ModelInfo> = None;
                            let mut open_model: Option<ModelInfo> = None;
                            let mut hide_model: Option<String> = None;
                            let mut set_draft_model: Option<ModelInfo> = None;
                            let mut move_before: Option<(String, String)> = None;
                            let visible_models: Vec<(usize, ModelInfo)> = self
                                .models
                                .iter()
                                .cloned()
                                .enumerate()
                                .filter(|(_, model)| {
                                    needle.is_empty()
                                        || model.display_name.to_lowercase().contains(&needle)
                                        || model.rel_path.to_lowercase().contains(&needle)
                                })
                                .collect();
                            for (idx, model) in visible_models {
                                if !needle.is_empty()
                                    && !model.display_name.to_lowercase().contains(&needle)
                                    && !model.rel_path.to_lowercase().contains(&needle)
                                {
                                    continue;
                                }
                                let selected = self.selected_index == Some(idx);
                                let fill = if selected { SILVER_MIST } else { FOG };
                                egui::Frame::new()
                                    .fill(fill)
                                    .stroke(Stroke::NONE)
                                    .corner_radius(CornerRadius::same(12))
                                    .inner_margin(egui::Margin::symmetric(7, 3))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let mut name = model.display_name.clone();
                                            if !model.size_label.is_empty() {
                                                name.push_str("  ");
                                                name.push_str(&model.size_label);
                                            }
                                            let response = ui
                                                .selectable_label(selected, {
                                                    let rich = RichText::new(name)
                                                        .color(self.text_color())
                                                        .size(14.0);
                                                    if self.config.appearance.bold_text {
                                                        rich.strong()
                                                    } else {
                                                        rich
                                                    }
                                                })
                                                .on_hover_cursor(CursorIcon::Grab);
                                            if response.clicked() {
                                                select_idx = Some(idx);
                                            }
                                            if response.drag_started_by(PointerButton::Primary) {
                                                self.dragging_model = Some(model.id.clone());
                                            }
                                            if response.hovered() {
                                                if let Some(dragged_id) =
                                                    self.dragging_model.clone()
                                                {
                                                    let primary_down = ui.ctx().input(|input| {
                                                        input.pointer.primary_down()
                                                    });
                                                    if primary_down && dragged_id != model.id {
                                                        move_before =
                                                            Some((dragged_id, model.id.clone()));
                                                    }
                                                }
                                            }
                                            response.context_menu(|ui| {
                                                if ui.button("修改显示名称").clicked() {
                                                    rename_model = Some(model.clone());
                                                    ui.close();
                                                }
                                                if ui.button("打开所在文件夹").clicked() {
                                                    open_model = Some(model.clone());
                                                    ui.close();
                                                }
                                                if ui.button("设置为 draft 草稿模型").clicked()
                                                {
                                                    set_draft_model = Some(model.clone());
                                                    ui.close();
                                                }
                                                if ui.button("从列表移除").clicked() {
                                                    hide_model = Some(model.id.clone());
                                                    ui.close();
                                                }
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if model.mmproj.is_some() {
                                                        Self::eye_badge(ui);
                                                    }
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(2.0);
                            }
                            if let Some(idx) = select_idx {
                                self.select_model(idx);
                            }
                            if let Some((dragged_id, target_id)) = move_before {
                                self.move_model_before(&dragged_id, &target_id);
                            }
                            let primary_down = ui.ctx().input(|input| input.pointer.primary_down());
                            if !primary_down {
                                self.dragging_model = None;
                            }
                            if let Some(model) = rename_model {
                                self.begin_rename_model(&model);
                            }
                            if let Some(model) = open_model {
                                self.open_model_in_explorer(&model);
                            }
                            if let Some(model) = set_draft_model {
                                self.set_model_as_draft(&model);
                            }
                            if let Some(id) = hide_model {
                                if !self.config.hidden_models.contains(&id) {
                                    self.config.hidden_models.push(id);
                                }
                                self.save_app_config();
                                self.refresh_models();
                            }
                        });
                    });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(FOG)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| {
                let available_width = ui.available_width();
                ui.horizontal_top(|ui| {
                    let gap = 10.0;
                    let left_width = ((available_width - gap) * 0.5).clamp(340.0, 460.0);
                    let right_width = (available_width - left_width - gap).max(340.0);
                    let column_height = ui.available_height().max(390.0);

                    ui.allocate_ui_with_layout(
                        egui::vec2(left_width, column_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            Self::fixed_size_card(
                                ui,
                                left_width,
                                column_height,
                                rgba_to_color(self.config.appearance.preset_border),
                                |ui| {
                                    self.preset_ui(ui);
                                    ui.add_space(8.0);
                                    let params_height = (ui.available_height() - 8.0).max(260.0);
                                    Self::recessed_frame().show(ui, |ui| {
                                        egui::ScrollArea::vertical()
                                            .max_height(params_height)
                                            .show(ui, |ui| {
                                                self.params_ui(ui);
                                            });
                                    });
                                },
                            );
                        },
                    );

                    ui.add_space(gap);
                    ui.allocate_ui_with_layout(
                        egui::vec2(right_width, column_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            Self::fixed_size_card(
                                ui,
                                right_width,
                                column_height,
                                rgba_to_color(self.config.appearance.preview_border),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        ui.heading(self.title_text("启动命令预览", 20.0));
                                        ui.add_space(8.0);
                                        if Self::small_button(ui, "导出 bat 启动脚本").clicked()
                                        {
                                            self.export_bat_script();
                                        }
                                    });
                                    ui.label(
                                        RichText::new("复制前可在这里核对 llama-server 参数。")
                                            .color(self.weak_text_color())
                                            .size(14.0),
                                    );
                                    ui.add_space(8.0);
                                    let mut preview = self.preview();
                                    let preview_height = (ui.available_height() - 36.0).max(300.0);
                                    ui.add_sized(
                                        [ui.available_width(), preview_height],
                                        egui::TextEdit::multiline(&mut preview)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                    for warning in &self.warnings {
                                        ui.colored_label(CAUTION, warning);
                                    }
                                },
                            );
                        },
                    );
                });
            });

        self.show_help_popup(ctx);
        self.show_rename_popup(ctx);
        self.show_draft_picker(ctx);
        self.show_server_log_window(ctx);
    }
}
