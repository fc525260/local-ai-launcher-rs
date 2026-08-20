use crate::command::{
    bat_script, build_args, command_preview, llama_server_path, parse_extra_args,
};
use crate::config::{
    load_config, save_config, AppConfig, ExtraParam, ExtraParamKind, ManualModel, ParamSection,
    ParameterLayout, Preset, ToggleState,
};
use crate::discovery::{
    discover_configured_models, discover_draft_models, DraftModelInfo, ModelInfo,
};
use crate::parameters::{
    parameter, parameter_by_key, ControlKind, ParamId, ParamReferenceEntry, ReferenceKind,
    EXTRA_PARAM_REFERENCE,
};
use crate::server::{self, ServerEvent, ServerProcess};
use eframe::egui::{
    self, Color32, CornerRadius, CursorIcon, FontId, PointerButton, RichText, Stroke, Vec2,
};
use std::collections::BTreeMap;
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
const PARAM_LABEL_WIDTH: f32 = 128.0;
const PARAM_CONTROL_WIDTH: f32 = 150.0;
const PARAM_ROW_HEIGHT: f32 = 26.0;

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
    visuals.window_stroke = Stroke::new(1.0_f32, SILVER_MIST);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = CornerRadius::same(6);
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
        widget.corner_radius = CornerRadius::same(4);
        widget.bg_stroke = Stroke::new(1.0_f32, SILVER_MIST);
        widget.fg_stroke = Stroke::new(1.0_f32, INK);
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
    defaults: &'static str,
    cpu: &'static str,
    memory: &'static str,
    vram: &'static str,
}

const fn help(
    title: &'static str,
    purpose: &'static str,
    defaults: &'static str,
    cpu: &'static str,
    memory: &'static str,
    vram: &'static str,
) -> ParamHelp {
    ParamHelp {
        title,
        purpose,
        defaults,
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
    customize_param_layout: bool,
    dragging_param: Option<ParamRow>,
    show_param_reference: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParamRow {
    Builtin(String),
    Extra(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParamDropTarget {
    section: ParamSection,
    index: usize,
}

fn move_param_row(
    sections: &mut [(ParamSection, Vec<ParamRow>); 3],
    dragged: &ParamRow,
    target: ParamDropTarget,
) -> bool {
    if target.section == ParamSection::Extra && matches!(dragged, ParamRow::Builtin(_)) {
        return false;
    }
    let Some((source_section, source_index)) = sections.iter().find_map(|(section, rows)| {
        rows.iter()
            .position(|row| row == dragged)
            .map(|index| (*section, index))
    }) else {
        return false;
    };
    if !sections
        .iter()
        .any(|(section, _)| *section == target.section)
    {
        return false;
    }

    let before = sections.clone();
    for (_, rows) in sections.iter_mut() {
        rows.retain(|row| row != dragged);
    }
    let (_, target_rows) = sections
        .iter_mut()
        .find(|(section, _)| *section == target.section)
        .expect("target section was validated");
    let adjusted_index = if source_section == target.section && source_index < target.index {
        target.index.saturating_sub(1)
    } else {
        target.index
    };
    target_rows.insert(adjusted_index.min(target_rows.len()), dragged.clone());
    *sections != before
}

fn copy_preset_layout(source: &Preset, target: &mut Preset) {
    target.parameter_layout = source.parameter_layout.clone();
    let extra_placements: BTreeMap<u64, (ParamSection, usize)> = source
        .extra_params
        .iter()
        .map(|item| (item.id, (item.section, item.position)))
        .collect();
    for item in &mut target.extra_params {
        if let Some((section, position)) = extra_placements.get(&item.id) {
            item.section = *section;
            item.position = *position;
        }
    }
}

fn restore_preset_layout(preset: &mut Preset) {
    preset.parameter_layout = ParameterLayout::default();
    for (position, item) in preset.extra_params.iter_mut().enumerate() {
        item.section = ParamSection::Extra;
        item.position = position;
    }
}

fn drag_auto_scroll_delta(pointer_y: f32, top: f32, bottom: f32, delta_time: f32) -> f32 {
    const EDGE_SIZE: f32 = 56.0;
    const MAX_SPEED: f32 = 760.0;
    let intensity = if pointer_y < top + EDGE_SIZE {
        ((top + EDGE_SIZE - pointer_y) / EDGE_SIZE).clamp(0.0, 1.0)
    } else if pointer_y > bottom - EDGE_SIZE {
        -((pointer_y - (bottom - EDGE_SIZE)) / EDGE_SIZE).clamp(0.0, 1.0)
    } else {
        0.0
    };
    intensity * MAX_SPEED * delta_time.clamp(0.0, 0.05)
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
            customize_param_layout: false,
            dragging_param: None,
            show_param_reference: false,
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

    fn persist_active_layout(&mut self) {
        let model_id = if self.active_preset_label == "当前模型预设" {
            self.selected_model().map(|model| model.id.clone())
        } else {
            None
        };
        let stored_preset = if let Some(model_id) = model_id {
            self.config.model_presets.get_mut(&model_id)
        } else {
            self.config.global_presets.get_mut("默认")
        };
        if let Some(stored_preset) = stored_preset {
            copy_preset_layout(&self.preset, stored_preset);
        }
        self.save_app_config();
    }

    fn restore_active_layout(&mut self) {
        restore_preset_layout(&mut self.preset);
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
        ui.label(RichText::new("默认值与选项").color(INK).size(13.0).strong());
        ui.label(RichText::new(help.defaults).color(GRAPHITE).size(13.0));
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
                .corner_radius(CornerRadius::same(4))
                .min_size(egui::vec2(22.0, 22.0)),
        );
        let clicked = response.clicked();
        response.on_hover_ui(|ui| Self::help_body(ui, help));
        if clicked {
            *popup = Some(help);
        }
    }

    fn param_label(
        ui: &mut egui::Ui,
        definition: &'static crate::parameters::ParameterDefinition,
    ) -> egui::Response {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add(
                egui::Label::new(RichText::new(definition.label).color(GRAPHITE).size(13.0))
                    .selectable(false),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(definition.display_flag())
                        .color(GRAPHITE)
                        .size(10.0)
                        .monospace(),
                )
                .selectable(false),
            );
        })
        .response
    }

    fn param_row(
        ui: &mut egui::Ui,
        definition: &'static crate::parameters::ParameterDefinition,
        help: ParamHelp,
        popup: &mut Option<ParamHelp>,
        control: impl FnOnce(&mut egui::Ui),
    ) {
        ui.horizontal(|ui| {
            ui.add_sized(
                [PARAM_LABEL_WIDTH, PARAM_ROW_HEIGHT],
                |ui: &mut egui::Ui| Self::param_label(ui, definition),
            );
            let remaining = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(remaining, PARAM_ROW_HEIGHT),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    control(ui);
                    Self::help_button(ui, help, popup);
                },
            );
        });
    }

    fn param_text(
        ui: &mut egui::Ui,
        definition: &'static crate::parameters::ParameterDefinition,
        value: &mut String,
        help: ParamHelp,
        popup: &mut Option<ParamHelp>,
    ) {
        Self::param_row(ui, definition, help, popup, |ui| {
            ui.add_sized(
                [PARAM_CONTROL_WIDTH, PARAM_ROW_HEIGHT],
                egui::TextEdit::singleline(value),
            );
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

    fn show_param_reference(&mut self, ctx: &egui::Context) {
        if !self.show_param_reference {
            return;
        }
        let mut open = self.show_param_reference;
        let mut close_requested = false;
        egui::Window::new("参数速查（未内置参数）")
            .collapsible(false)
            .default_width(760.0)
            .default_height(560.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(
                        "点击\"添加\"可将参数加入\"额外参数\"区并生成对应控件；带值的参数自动生成输入框，开关生成复选框。",
                    )
                    .color(GRAPHITE)
                    .size(13.0),
                );
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for group in EXTRA_PARAM_REFERENCE {
                            egui::CollapsingHeader::new(group.title)
                                .default_open(true)
                                .show(ui, |ui| {
                                    for entry in group.entries {
                                        ui.horizontal(|ui| {
                                            if Self::small_button(ui, "添加").clicked() {
                                                self.add_reference_param(entry);
                                                close_requested = true;
                                            }
                                            let flag_width = 190.0;
                                            ui.add_sized(
                                                [flag_width, 20.0],
                                                egui::Label::new(
                                                    RichText::new(entry.display)
                                                        .color(INK)
                                                        .size(12.5)
                                                        .monospace(),
                                                ),
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(entry.description)
                                                        .color(GRAPHITE)
                                                        .size(13.0),
                                                )
                                                .truncate(),
                                            );
                                        });
                                    }
                                });
                            ui.separator();
                        }
                    });
                ui.add_space(6.0);
                if Self::small_button(ui, "关闭").clicked() {
                    close_requested = true;
                }
            });
        if close_requested {
            open = false;
        }
        self.show_param_reference = open;
    }

    fn add_reference_param(&mut self, entry: &'static ParamReferenceEntry) {
        let position = self.section_rows(ParamSection::Extra).len();
        let next_id = self
            .preset
            .extra_params
            .iter()
            .map(|item| item.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let (kind, flag, value, choices) = match entry.kind {
            ReferenceKind::Value => (
                ExtraParamKind::FlagValue,
                entry.flag,
                String::new(),
                Vec::new(),
            ),
            ReferenceKind::Toggle => (
                ExtraParamKind::FlagToggle,
                entry.flag,
                String::new(),
                Vec::new(),
            ),
            ReferenceKind::Choice(choices) => (
                ExtraParamKind::FlagChoice,
                entry.flag,
                String::new(),
                choices.iter().map(|choice| (*choice).to_string()).collect(),
            ),
        };
        self.preset.extra_params.push(ExtraParam {
            id: next_id,
            enabled: true,
            section: ParamSection::Extra,
            position,
            kind,
            flag: flag.to_string(),
            value,
            choices,
            ..ExtraParam::default()
        });
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
                    "默认：不启用。勾选后需模型目录中存在对应的 mmproj 文件。",
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
                        .corner_radius(CornerRadius::same(4))
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
                        .corner_radius(CornerRadius::same(4))
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
                .corner_radius(CornerRadius::same(4))
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
            .stroke(Stroke::new(1.0_f32, border))
            .corner_radius(CornerRadius::same(8))
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
            .corner_radius(CornerRadius::same(6))
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
        let stroke = Stroke::new(1.4_f32, INK);
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

    fn parameter_help(id: ParamId) -> ParamHelp {
        let definition = parameter(id);
        match id {
            ParamId::LoadMode => help(
                definition.label,
                "对应 --load-mode。可选择 none、mmap、mlock、mmap+mlock 或 dio。",
                "默认：llama.cpp 默认 mmap（内存映射）。选项：none=全部加载进内存；mmap=内存映射；mlock=锁定内存防换出；mmap+mlock=映射并锁定；dio=直接 I/O。",
                "加载阶段的 CPU 行为取决于所选模式。",
                "mlock 会阻止模型内存被换出，mmap 使用系统文件缓存。",
                "无直接影响。",
            ),
            ParamId::FlashAttn => help(
                definition.label,
                "对应 --flash-attn on|off|auto。-fa 是官方短名。",
                "默认：llama.cpp 默认 off（关闭）；本应用默认 on。选项：on=启用；off=禁用；auto=自动判断。",
                "通常可降低注意力计算开销。",
                "影响较小。",
                "通常可改善支持设备上的速度和显存效率。",
            ),
            ParamId::SpecType => help(
                definition.label,
                "对应 --spec-type，选项固定为当前 llama-server 支持的推测解码类型。",
                "默认：llama.cpp 默认 none（关闭推测）；检测到 MTP 草稿模型时本应用自动使用 draft-mtp。选项：none、draft、draft-mtp、prompt-lookup。",
                "会增加草稿模型计算。",
                "需要草稿模型和缓存。",
                "草稿模型放在 GPU 时会额外占用显存。",
            ),
            ParamId::ReasoningPreserve => help(
                definition.label,
                "对应 --reasoning-preserve / --no-reasoning-preserve。仅支持 preserve reasoning 的聊天模板会使用此设置。",
                "默认：llama.cpp 默认 off（关闭）。选项：默认=由聊天模板决定；启用=--reasoning-preserve；禁用=--no-reasoning-preserve。",
                "无直接影响。",
                "保留更长历史时会增加上下文占用。",
                "更长上下文可能增加 KV 显存。",
            ),
            ParamId::CpuMoe => help(
                definition.label,
                "对应 --cpu-moe，将全部 MoE 专家权重放在 CPU。启用后不再传 --n-cpu-moe。",
                "默认：llama.cpp 默认 off（全部专家在 GPU）；启用：--cpu-moe。",
                "显著增加 CPU 和内存带宽压力。",
                "增加系统内存占用。",
                "降低 MoE 权重的显存占用。",
            ),
            ParamId::NCpuMoe => help(
                definition.label,
                "对应 --n-cpu-moe，仅将前 N 个 MoE 层的专家权重放在 CPU。",
                "默认：llama.cpp 默认 999（全部层）。",
                "层数越多，CPU 压力越高。",
                "层数越多，系统内存占用越高。",
                "可降低显存占用。",
            ),
            ParamId::ImageMinTokens | ParamId::ImageMaxTokens => help(
                definition.label,
                "仅用于支持动态分辨率的视觉模型。",
                "默认：llama.cpp 默认 768。",
                "图像 token 越多，计算越多。",
                "会占用上下文和 KV 缓存。",
                "KV offload 时会增加显存占用。",
            ),
            ParamId::Jinja => help(
                definition.label,
                "对应 --jinja，启用 llama.cpp Jinja chat template 解析。",
                "默认：llama.cpp 默认不启用（off）；本应用默认 启用。",
                "模板渲染开销很小。",
                "几乎无影响。",
                "无影响。",
            ),
            ParamId::WebUi => help(
                definition.label,
                "对应 --ui / --no-ui，控制 llama-server 内置网页界面。",
                "默认：llama.cpp 默认 off（不启用 Web UI）；启用：--ui；禁用：--no-ui。",
                "影响很小。",
                "增加少量服务端资源。",
                "无影响。",
            ),
            ParamId::LogTimestamps => help(
                definition.label,
                "对应 --log-timestamps / --no-log-timestamps。",
                "默认：llama.cpp 默认 on（记录时间戳）；启用：--log-timestamps；禁用：--no-log-timestamps。",
                "几乎无影响。",
                "日志文本会略微变长。",
                "无影响。",
            ),
            ParamId::ThreadsBatch => help(
                definition.label,
                "对应 --threads-batch (-tb)，prompt 批处理时使用的线程数。",
                "默认：llama.cpp 默认与 --threads 相同。",
                "批处理阶段会按此值并行。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Predict => help(
                definition.label,
                "对应 --predict (-n)，限制本次生成的最大 token 数。",
                "默认：llama.cpp 默认 -1（无限生成）。",
                "生成长度越大耗时越长。",
                "长输出占用更多上下文。",
                "上下文增长会增加 KV 显存。",
            ),
            ParamId::Keep => help(
                definition.label,
                "对应 --keep，保留初始 prompt 的 token 数。",
                "默认：llama.cpp 默认 0（不保留）；0=不保留，-1=全部保留。",
                "无直接影响。",
                "保留越长占用上下文越多。",
                "随上下文增长增加显存。",
            ),
            ParamId::RopeScaling => help(
                definition.label,
                "对应 --rope-scaling，RoPE 频率缩放方法：none、linear 或 yarn。",
                "默认：llama.cpp 默认 linear（线性缩放）。选项：none=不缩放；linear=线性缩放；yarn=YaRN。",
                "缩放方法不同计算略有差异。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::RopeScale => help(
                definition.label,
                "对应 --rope-scale，RoPE 上下文缩放因子，将上下文扩大 N 倍。",
                "默认：llama.cpp 默认 1.0（不缩放）。",
                "几乎无影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::RopeFreqBase => help(
                definition.label,
                "对应 --rope-freq-base，RoPE 基频，用于 NTK 缩放。",
                "默认：llama.cpp 默认 10000。",
                "几乎无影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::RopeFreqScale => help(
                definition.label,
                "对应 --rope-freq-scale，RoPE 频率缩放因子，等价于扩大上下文 1/N 倍。",
                "默认：llama.cpp 默认 1.0。",
                "几乎无影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::YarnOrigCtx => help(
                definition.label,
                "对应 --yarn-orig-ctx，YaRN 原始上下文大小。",
                "默认：llama.cpp 默认 0（使用模型训练上下文大小）。",
                "几乎无影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Rpc => help(
                definition.label,
                "对应 --rpc，逗号分隔的 RPC 服务器列表（host:port），用于跨机推理。",
                "默认：llama.cpp 默认空（不使用 RPC）。",
                "网络通信增加延迟。",
                "远端内存由各节点承担。",
                "远端显存由各节点承担。",
            ),
            ParamId::Numa => help(
                definition.label,
                "对应 --numa，NUMA 优化方式：distribute、isolate 或 numactl。",
                "默认：llama.cpp 默认不启用。选项：distribute=分布式；isolate=隔离；numactl=使用 numactl。",
                "影响线程与内存亲和。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Fit => help(
                definition.label,
                "对应 --fit，是否自动调整未设置参数以适应显存；on 或 off。",
                "默认：llama.cpp 默认 off。选项：on=自动调整；off=关闭。",
                "自动调整会影响层数与上下文。",
                "无额外影响。",
                "用于适配显存。",
            ),
            ParamId::CheckTensors => help(
                definition.label,
                "对应 --check-tensors，加载时检查张量数据是否有无效值。",
                "默认：llama.cpp 默认 off（不检查）；启用：--check-tensors。",
                "加载阶段增加检查开销。",
                "几乎无影响。",
                "无影响。",
            ),
            ParamId::LogFile => help(
                definition.label,
                "对应 --log-file，将日志输出到指定文件。",
                "默认：llama.cpp 默认空（输出到控制台）。",
                "几乎无影响。",
                "日志写入磁盘。",
                "无影响。",
            ),
            ParamId::SpecDraftThreads => help(
                definition.label,
                "对应 --spec-draft-threads (-td)，草稿模型生成时的线程数。",
                "默认：llama.cpp 默认与 --threads 相同。",
                "草稿推理并行度。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::SpecDraftNgl => help(
                definition.label,
                "对应 --spec-draft-ngl (-ngld)，草稿模型放入 VRAM 的层数。",
                "默认：llama.cpp 默认与 --ngl 相同。",
                "影响草稿推理速度。",
                "无额外影响。",
                "层数越多显存占用越高。",
            ),
            ParamId::SpecDraftDevice => help(
                definition.label,
                "对应 --spec-draft-device (-devd)，草稿模型使用的设备列表。",
                "默认：llama.cpp 默认与 --device 相同。",
                "影响草稿推理速度。",
                "无额外影响。",
                "随设备与层数变化。",
            ),
            ParamId::Seed => help(
                definition.label,
                "对应 --seed (-s)，随机种子。固定后可复现输出。",
                "默认：llama.cpp 默认 -1（随机）。",
                "几乎无影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Temperature => help(
                definition.label,
                "对应 --temperature (-temp)，采样温度。",
                "默认：llama.cpp 默认 0.80。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::TopK => help(
                definition.label,
                "对应 --top-k，Top-k 采样。",
                "默认：llama.cpp 默认 40，0=禁用。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::TopP => help(
                definition.label,
                "对应 --top-p，Top-p 核采样。",
                "默认：llama.cpp 默认 0.95，1.0=禁用。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::MinP => help(
                definition.label,
                "对应 --min-p，Min-p 采样。",
                "默认：llama.cpp 默认 0.05，0.0=禁用。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::RepeatLastN => help(
                definition.label,
                "对应 --repeat-last-n，重复惩罚考虑的最近 token 数。",
                "默认：llama.cpp 默认 64，0=禁用。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::RepeatPenalty => help(
                definition.label,
                "对应 --repeat-penalty，重复惩罚系数。",
                "默认：llama.cpp 默认 1.00（1.0=禁用）。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::PresencePenalty => help(
                definition.label,
                "对应 --presence-penalty，存在惩罚（Alpha）。",
                "默认：llama.cpp 默认 0.00。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::FrequencyPenalty => help(
                definition.label,
                "对应 --frequency-penalty，频率惩罚。",
                "默认：llama.cpp 默认 0.00。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Mirostat => help(
                definition.label,
                "对应 --mirostat：0 禁用，1 Mirostat，2 Mirostat 2.0。",
                "默认：llama.cpp 默认 0（禁用）。选项：0=禁用；1=Mirostat；2=Mirostat 2.0。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::MirostatLr => help(
                definition.label,
                "对应 --mirostat-lr，Mirostat 学习率（eta）。",
                "默认：llama.cpp 默认 0.10。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::MirostatEnt => help(
                definition.label,
                "对应 --mirostat-ent，Mirostat 目标熵（tau）。",
                "默认：llama.cpp 默认 5.00。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::GrammarFile => help(
                definition.label,
                "对应 --grammar-file，从文件读取 BNF 语法约束生成。",
                "默认：llama.cpp 默认空（不使用语法约束）。",
                "约束校验增加少量开销。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::JsonSchema => help(
                definition.label,
                "对应 --json-schema (-j)，用 JSON Schema 约束生成，例如 {} 表示任意 JSON 对象。",
                "默认：llama.cpp 默认空（不使用）。",
                "约束校验增加少量开销。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::SystemPrompt => help(
                definition.label,
                "对应 --system-prompt (-sys)，系统提示词（适用于支持聊天模板的模型）。",
                "默认：llama.cpp 默认空（不设置）。",
                "无直接影响。",
                "提示词占用上下文。",
                "随上下文增长增加显存。",
            ),
            ParamId::ReversePrompt => help(
                definition.label,
                "对应 --reverse-prompt (-r)，遇到该提示词时停止生成。",
                "默认：llama.cpp 默认空（不设置）。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::ContextShift => help(
                definition.label,
                "对应 --context-shift / --no-context-shift，无限生成时是否使用上下文移位。",
                "默认：llama.cpp 默认启用；启用：--context-shift；禁用：--no-context-shift。",
                "无直接影响。",
                "影响长对话的上下文管理。",
                "无影响。",
            ),
            ParamId::ShowTimings => help(
                definition.label,
                "对应 --show-timings / --no-show-timings，每次响应后显示计时信息。",
                "默认：llama.cpp 默认 off（关闭）；启用：--show-timings；禁用：--no-show-timings。",
                "几乎无影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Host => help(
                definition.label,
                "对应 --host，服务监听地址。",
                "默认：llama.cpp 默认 127.0.0.1。",
                "影响较小。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Port => help(
                definition.label,
                "对应 --port，服务监听端口。",
                "默认：llama.cpp 默认 8080（本应用默认 9931）。",
                "影响较小。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Alias => help(
                definition.label,
                "对应 --alias，模型别名，用于 API 中标识模型。",
                "默认：llama.cpp 默认使用模型文件名。",
                "无直接影响。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Ngl => help(
                definition.label,
                "对应 --gpu-layers (-ngl)，放入 GPU 的层数。",
                "默认：llama.cpp 默认 0（全部在 CPU）；-1=全部放入 GPU。",
                "GPU 层越多，CPU 负载越低。",
                "影响较小。",
                "层数越多显存占用越高。",
            ),
            ParamId::Threads => help(
                definition.label,
                "对应 --threads (-t)，生成阶段的 CPU 线程数。",
                "默认：llama.cpp 默认使用 CPU 物理核心数。",
                "线程越多，CPU 负载越高。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::BatchSize => help(
                definition.label,
                "对应 --batch-size (-b)，prompt 批处理大小。",
                "默认：llama.cpp 默认 2048。",
                "批处理阶段计算更多。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::UbatchSize => help(
                definition.label,
                "对应 --ubatch-size (-ub)，微批大小。",
                "默认：llama.cpp 默认 512。",
                "微批越大计算越集中。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Parallel => help(
                definition.label,
                "对应 --parallel (-np)，并发请求槽位数量。",
                "默认：llama.cpp 默认 1。",
                "并发槽位越多，总计算越高。",
                "每个槽位需要独立上下文和 KV 缓存。",
                "槽位越多 KV 显存占用越高。",
            ),
            ParamId::CtxSize => help(
                definition.label,
                "对应 --ctx-size (-c)，上下文长度。",
                "默认：llama.cpp 默认 4096。",
                "上下文越长，计算越慢。",
                "上下文占用与长度成正比。",
                "KV 缓存显存与上下文长度成正比。",
            ),
            ParamId::CacheTypeK => help(
                definition.label,
                "对应 --cache-type-k (-ctk)，K 缓存量化类型。",
                "默认：llama.cpp 默认 f16。选项：f32、f16、bf16、q8_0、q4_0、q4_1、iq4_nl、q5_0、q5_1。",
                "无直接影响。",
                "量化程度越高内存占用越低。",
                "量化程度越高显存占用越低。",
            ),
            ParamId::CacheTypeV => help(
                definition.label,
                "对应 --cache-type-v (-ctv)，V 缓存量化类型。",
                "默认：llama.cpp 默认 f16。选项：f32、f16、bf16、q8_0、q4_0、q4_1、iq4_nl、q5_0、q5_1。",
                "无直接影响。",
                "量化程度越高内存占用越低。",
                "量化程度越高显存占用越低。",
            ),
            ParamId::Timeout => help(
                definition.label,
                "对应 --timeout，请求超时秒数。",
                "默认：llama.cpp 默认 600 秒。",
                "影响较小。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::SplitMode => help(
                definition.label,
                "对应 --split-mode (-sm)，模型切分方式。",
                "默认：llama.cpp 默认 layer。",
                "影响 GPU 间负载分配。",
                "无额外影响。",
                "影响各 GPU 显存占用。",
            ),
            ParamId::TensorSplit => help(
                definition.label,
                "对应 --tensor-split (-ts)，各 GPU 的显存分配比例（逗号分隔）。",
                "默认：llama.cpp 默认按层自动均衡。",
                "影响较小。",
                "无额外影响。",
                "按比例分配各 GPU 显存。",
            ),
            ParamId::MainGpu => help(
                definition.label,
                "对应 --main-gpu (-mg)，主 GPU 编号。",
                "默认：llama.cpp 默认 GPU 0。",
                "无直接影响。",
                "无额外影响。",
                "主 GPU 承担主要计算与 KV。",
            ),
            ParamId::Device => help(
                definition.label,
                "对应 --device (-dev)，指定参与推理的设备列表。",
                "默认：llama.cpp 默认全部可用设备。",
                "设备越少，单设备负载越高。",
                "无额外影响。",
                "仅使用列出的设备显存。",
            ),
            ParamId::SpecDraftNMax => help(
                definition.label,
                "对应 --spec-draft-n-max，推测解码最大草稿 token 数。",
                "默认：llama.cpp 默认 16。",
                "草稿越长，验证阶段计算越多。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::SpecDraftNMin => help(
                definition.label,
                "对应 --spec-draft-n-min，推测解码最小草稿 token 数。",
                "默认：llama.cpp 默认 5。",
                "草稿越短，加速收益越小。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::SpecDraftPMin => help(
                definition.label,
                "对应 --spec-draft-p-min，草稿采样最小概率。",
                "默认：llama.cpp 默认 0.1。",
                "影响草稿多样性。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::SpecDraftPSplit => help(
                definition.label,
                "对应 --spec-draft-p-split，草稿采样的概率切分。",
                "默认：llama.cpp 默认 0.1。",
                "影响草稿多样性。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Offline => help(
                definition.label,
                "对应 --offline，离线模式，禁用联网检查。",
                "默认：llama.cpp 默认 off（可联网）；启用：--offline。",
                "影响较小。",
                "无额外影响。",
                "无影响。",
            ),
            ParamId::Verbose => help(
                definition.label,
                "对应 --verbose (-v)，输出详细日志。",
                "默认：llama.cpp 默认 off；启用：--verbose。",
                "日志输出更多，影响很小。",
                "日志文本变长。",
                "无影响。",
            ),
            ParamId::KvOffload => help(
                definition.label,
                "对应 --kv-offload / --no-kv-offload，控制 KV 缓存 offload 到 CPU。",
                "默认：llama.cpp 默认 on（启用 offload）；启用：--kv-offload；禁用：--no-kv-offload。",
                "offload 后部分计算移到 CPU。",
                "offload 时 KV 缓存占用系统内存。",
                "offload 可降低显存占用。",
            ),
            ParamId::KvUnified => help(
                definition.label,
                "对应 --kv-unified，将 K/V 缓存合并为统一 buffer。",
                "默认：llama.cpp 默认 off；启用：--kv-unified。",
                "影响较小。",
                "统一 buffer 便于共享和调整。",
                "可更灵活地利用显存。",
            ),
            ParamId::SwaFull => help(
                definition.label,
                "对应 --swa-full，SWA（滑动窗口注意力）使用完整缓存。",
                "默认：llama.cpp 默认 off；启用：--swa-full。",
                "无直接影响。",
                "SWA 完整缓存占用更多内存。",
                "SWA 完整缓存占用更多显存。",
            ),
        }
    }

    fn param_choice(
        ui: &mut egui::Ui,
        definition: &'static crate::parameters::ParameterDefinition,
        value: &mut String,
        choices: &[&str],
        help: ParamHelp,
        popup: &mut Option<ParamHelp>,
    ) {
        let selected = if value.is_empty() {
            "默认".to_string()
        } else {
            value.clone()
        };
        Self::param_row(ui, definition, help, popup, |ui| {
            egui::ComboBox::from_id_salt(definition.key)
                .width(PARAM_CONTROL_WIDTH)
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    ui.selectable_value(value, String::new(), "默认");
                    for choice in choices {
                        ui.selectable_value(value, (*choice).to_string(), *choice);
                    }
                });
        });
    }

    fn param_toggle(
        ui: &mut egui::Ui,
        definition: &'static crate::parameters::ParameterDefinition,
        value: &mut ToggleState,
        two_sided: bool,
        help: ParamHelp,
        popup: &mut Option<ParamHelp>,
    ) {
        let selected = match value {
            ToggleState::Default => "默认",
            ToggleState::Enabled => "启用",
            ToggleState::Disabled => "禁用",
        };
        Self::param_row(ui, definition, help, popup, |ui| {
            egui::ComboBox::from_id_salt(definition.key)
                .width(PARAM_CONTROL_WIDTH)
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    ui.selectable_value(value, ToggleState::Default, "默认");
                    ui.selectable_value(value, ToggleState::Enabled, "启用");
                    if two_sided {
                        ui.selectable_value(value, ToggleState::Disabled, "禁用");
                    }
                });
        });
    }

    fn render_builtin_param(&mut self, ui: &mut egui::Ui, id: ParamId) {
        let definition = parameter(id);
        let param_help = Self::parameter_help(id);
        macro_rules! text_param {
            ($field:ident) => {
                Self::param_text(
                    ui,
                    definition,
                    &mut self.preset.$field,
                    param_help,
                    &mut self.help_popup,
                )
            };
        }
        macro_rules! choice_param {
            ($field:ident) => {
                if let ControlKind::Choice(choices) = definition.control {
                    Self::param_choice(
                        ui,
                        definition,
                        &mut self.preset.$field,
                        choices,
                        param_help,
                        &mut self.help_popup,
                    )
                }
            };
        }
        macro_rules! toggle_param {
            ($field:ident) => {
                Self::param_toggle(
                    ui,
                    definition,
                    &mut self.preset.$field,
                    matches!(definition.control, ControlKind::TwoSidedToggle),
                    param_help,
                    &mut self.help_popup,
                )
            };
        }
        match id {
            ParamId::Host => text_param!(host),
            ParamId::Port => text_param!(port),
            ParamId::Alias => text_param!(alias),
            ParamId::Ngl => text_param!(ngl),
            ParamId::NCpuMoe => {
                ui.add_enabled_ui(self.preset.cpu_moe != ToggleState::Enabled, |ui| {
                    Self::param_text(
                        ui,
                        definition,
                        &mut self.preset.n_cpu_moe,
                        param_help,
                        &mut self.help_popup,
                    );
                });
            }
            ParamId::Threads => text_param!(threads),
            ParamId::ThreadsBatch => text_param!(threads_batch),
            ParamId::Predict => text_param!(predict),
            ParamId::Keep => text_param!(keep),
            ParamId::BatchSize => text_param!(batch_size),
            ParamId::UbatchSize => text_param!(ubatch_size),
            ParamId::Parallel => text_param!(parallel),
            ParamId::CtxSize => text_param!(ctx_size),
            ParamId::CacheTypeK => choice_param!(cache_type_k),
            ParamId::CacheTypeV => choice_param!(cache_type_v),
            ParamId::LoadMode => choice_param!(load_mode),
            ParamId::FlashAttn => choice_param!(flash_attn),
            ParamId::RopeScaling => choice_param!(rope_scaling),
            ParamId::RopeScale => text_param!(rope_scale),
            ParamId::RopeFreqBase => text_param!(rope_freq_base),
            ParamId::RopeFreqScale => text_param!(rope_freq_scale),
            ParamId::YarnOrigCtx => text_param!(yarn_orig_ctx),
            ParamId::Rpc => text_param!(rpc),
            ParamId::Numa => choice_param!(numa),
            ParamId::Fit => choice_param!(fit),
            ParamId::CheckTensors => toggle_param!(check_tensors),
            ParamId::LogFile => text_param!(log_file),
            ParamId::SpecType => choice_param!(spec_type),
            ParamId::SpecDraftNMax => text_param!(spec_draft_n_max),
            ParamId::SpecDraftNMin => text_param!(spec_draft_n_min),
            ParamId::SpecDraftThreads => text_param!(spec_draft_threads),
            ParamId::SpecDraftNgl => text_param!(spec_draft_ngl),
            ParamId::SpecDraftDevice => text_param!(spec_draft_device),
            ParamId::ImageMinTokens => text_param!(image_min_tokens),
            ParamId::ImageMaxTokens => text_param!(image_max_tokens),
            ParamId::Jinja => toggle_param!(jinja),
            ParamId::Timeout => text_param!(timeout),
            ParamId::SplitMode => text_param!(split_mode),
            ParamId::TensorSplit => text_param!(tensor_split),
            ParamId::MainGpu => text_param!(main_gpu),
            ParamId::Device => text_param!(device),
            ParamId::SpecDraftPMin => text_param!(spec_draft_p_min),
            ParamId::SpecDraftPSplit => text_param!(spec_draft_p_split),
            ParamId::Seed => text_param!(seed),
            ParamId::Temperature => text_param!(temperature),
            ParamId::TopK => text_param!(top_k),
            ParamId::TopP => text_param!(top_p),
            ParamId::MinP => text_param!(min_p),
            ParamId::RepeatLastN => text_param!(repeat_last_n),
            ParamId::RepeatPenalty => text_param!(repeat_penalty),
            ParamId::PresencePenalty => text_param!(presence_penalty),
            ParamId::FrequencyPenalty => text_param!(frequency_penalty),
            ParamId::Mirostat => choice_param!(mirostat),
            ParamId::MirostatLr => text_param!(mirostat_lr),
            ParamId::MirostatEnt => text_param!(mirostat_ent),
            ParamId::GrammarFile => text_param!(grammar_file),
            ParamId::JsonSchema => text_param!(json_schema),
            ParamId::SystemPrompt => text_param!(system_prompt),
            ParamId::ReversePrompt => text_param!(reverse_prompt),
            ParamId::WebUi => toggle_param!(web_ui),
            ParamId::LogTimestamps => toggle_param!(log_timestamps),
            ParamId::Offline => toggle_param!(offline),
            ParamId::Verbose => toggle_param!(verbose),
            ParamId::KvOffload => toggle_param!(kv_offload),
            ParamId::KvUnified => toggle_param!(kv_unified),
            ParamId::SwaFull => toggle_param!(swa_full),
            ParamId::CpuMoe => toggle_param!(cpu_moe),
            ParamId::ReasoningPreserve => toggle_param!(reasoning_preserve),
            ParamId::ContextShift => toggle_param!(context_shift),
            ParamId::ShowTimings => toggle_param!(show_timings),
        }
    }

    fn section_rows(&self, section: ParamSection) -> Vec<ParamRow> {
        let builtins: &[String] = match section {
            ParamSection::Common => &self.preset.parameter_layout.common,
            ParamSection::Other => &self.preset.parameter_layout.other,
            ParamSection::Extra => &[],
        };
        let mut rows: Vec<ParamRow> = builtins
            .iter()
            .filter(|key| parameter_by_key(key).is_some())
            .cloned()
            .map(ParamRow::Builtin)
            .collect();
        let mut extras: Vec<(usize, usize, u64)> = self
            .preset
            .extra_params
            .iter()
            .enumerate()
            .filter(|(_, item)| item.section == section)
            .map(|(index, item)| (item.position, index, item.id))
            .collect();
        extras.sort_by_key(|(position, index, _)| (*position, *index));
        for (position, _, id) in extras {
            rows.insert(position.min(rows.len()), ParamRow::Extra(id));
        }
        rows
    }

    fn param_row_label(&self, row: &ParamRow) -> String {
        match row {
            ParamRow::Builtin(key) => parameter_by_key(key)
                .map(|definition| format!("{}  {}", definition.label, definition.display_flag()))
                .unwrap_or_else(|| key.clone()),
            ParamRow::Extra(id) => self
                .preset
                .extra_params
                .iter()
                .find(|item| item.id == *id)
                .map(|item| match item.kind {
                    ExtraParamKind::FreeText => {
                        if item.text.trim().is_empty() {
                            "未填写额外参数".to_string()
                        } else {
                            item.text.clone()
                        }
                    }
                    ExtraParamKind::FlagValue
                    | ExtraParamKind::FlagToggle
                    | ExtraParamKind::FlagChoice => {
                        if item.flag.trim().is_empty() {
                            "未填写额外参数".to_string()
                        } else {
                            item.flag.clone()
                        }
                    }
                })
                .unwrap_or_else(|| "额外参数".to_string()),
        }
    }

    fn render_extra_param(&mut self, ui: &mut egui::Ui, id: u64) {
        let Some(index) = self
            .preset
            .extra_params
            .iter()
            .position(|item| item.id == id)
        else {
            return;
        };
        let mut remove = false;
        ui.horizontal(|ui| {
            let item = &mut self.preset.extra_params[index];
            ui.checkbox(&mut item.enabled, "")
                .on_hover_text("启用额外参数");
            match item.kind {
                ExtraParamKind::FreeText => {
                    let edit_width = (ui.available_width() - 36.0).max(100.0);
                    ui.add_sized(
                        [edit_width, 28.0],
                        egui::TextEdit::singleline(&mut item.text)
                            .hint_text("例如 --flash-attn on"),
                    );
                }
                ExtraParamKind::FlagValue => {
                    ui.add(
                        egui::Label::new(
                            RichText::new(item.flag.clone())
                                .color(GRAPHITE)
                                .size(12.5)
                                .monospace(),
                        )
                        .selectable(false),
                    );
                    let edit_width = (ui.available_width() - 36.0 - 60.0).max(100.0);
                    ui.add_sized(
                        [edit_width, 28.0],
                        egui::TextEdit::singleline(&mut item.value).hint_text("参数值"),
                    );
                }
                ExtraParamKind::FlagToggle => {
                    ui.add(
                        egui::Label::new(
                            RichText::new(item.flag.clone())
                                .color(GRAPHITE)
                                .size(12.5)
                                .monospace(),
                        )
                        .selectable(false),
                    );
                    ui.label(RichText::new("勾选时输出该参数").color(GRAPHITE).size(12.0));
                }
                ExtraParamKind::FlagChoice => {
                    ui.add(
                        egui::Label::new(
                            RichText::new(item.flag.clone())
                                .color(GRAPHITE)
                                .size(12.5)
                                .monospace(),
                        )
                        .selectable(false),
                    );
                    let selected = if item.value.is_empty() {
                        "默认".to_string()
                    } else {
                        item.value.clone()
                    };
                    egui::ComboBox::from_id_salt(("extra-choice", item.id))
                        .width(150.0)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut item.value, String::new(), "默认");
                            for choice in &item.choices {
                                ui.selectable_value(
                                    &mut item.value,
                                    choice.clone(),
                                    choice.clone(),
                                );
                            }
                        });
                }
            }
            if ui.button("×").on_hover_text("删除额外参数").clicked() {
                remove = true;
            }
        });
        if let Some(item) = self.preset.extra_params.get(index) {
            if item.kind == ExtraParamKind::FreeText && !item.text.trim().is_empty() {
                if let Err(message) = parse_extra_args(&item.text) {
                    ui.colored_label(CAUTION, message);
                }
            }
        }
        if remove {
            self.preset.extra_params.remove(index);
        }
    }

    fn render_normal_section(&mut self, ui: &mut egui::Ui, title: &str, section: ParamSection) {
        let rows = self.section_rows(section);
        let default_open = section != ParamSection::Other;
        egui::CollapsingHeader::new(title)
            .default_open(default_open)
            .show(ui, |ui| {
                if section == ParamSection::Extra {
                    ui.horizontal(|ui| {
                        if Self::small_button(ui, "+ 添加参数").clicked() {
                            let next_id = self
                                .preset
                                .extra_params
                                .iter()
                                .map(|item| item.id)
                                .max()
                                .unwrap_or(0)
                                .saturating_add(1);
                            self.preset.extra_params.push(ExtraParam {
                                id: next_id,
                                section: ParamSection::Extra,
                                position: rows.len(),
                                ..ExtraParam::default()
                            });
                        }
                        if Self::small_button(ui, "参数速查").clicked() {
                            self.show_param_reference = true;
                        }
                        Self::help_button(
                            ui,
                            help(
                                "额外参数",
                                "每项是一行自由命令文本。启用的条目会在内置参数之后按顺序追加。",
                                "默认：不添加任何额外参数。",
                                "取决于参数。",
                                "取决于参数。",
                                "取决于参数。",
                            ),
                            &mut self.help_popup,
                        );
                    });
                }
                for row in rows {
                    match row {
                        ParamRow::Builtin(key) => {
                            if let Some(definition) = parameter_by_key(&key) {
                                self.render_builtin_param(ui, definition.id);
                            }
                        }
                        ParamRow::Extra(id) => self.render_extra_param(ui, id),
                    }
                    ui.separator();
                }
            });
    }

    fn render_layout_section(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        section: ParamSection,
        drop_target: &mut Option<ParamDropTarget>,
    ) {
        let rows = self.section_rows(section);
        let frame = egui::Frame::new()
            .fill(SNOW)
            .stroke(Stroke::new(1.0_f32, SILVER_MIST))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(egui::Margin::same(8));
        frame.show(ui, |ui| {
            ui.style_mut().interaction.selectable_labels = false;
            let header = ui.horizontal(|ui| {
                ui.add(egui::Label::new(RichText::new(title).strong()).selectable(false));
                ui.add(
                    egui::Label::new(RichText::new("拖放到此处").color(GRAPHITE).size(12.0))
                        .selectable(false),
                );
            });
            let pointer_position = ui.ctx().pointer_interact_pos();
            if let (Some(dragged), Some(pointer)) = (self.dragging_param.as_ref(), pointer_position)
            {
                if header.response.rect.contains(pointer)
                    && !(section == ParamSection::Extra && matches!(dragged, ParamRow::Builtin(_)))
                {
                    *drop_target = Some(ParamDropTarget { section, index: 0 });
                    let y = header.response.rect.bottom();
                    ui.painter().line_segment(
                        [
                            egui::pos2(header.response.rect.left(), y),
                            egui::pos2(header.response.rect.right(), y),
                        ],
                        Stroke::new(2.0_f32, AZURE),
                    );
                }
            }
            for (index, row) in rows.iter().enumerate() {
                let label = self.param_row_label(&row);
                let row_response = ui.horizontal(|ui| {
                    let handle = ui
                        .add(
                            egui::Label::new(
                                RichText::new("::").color(GRAPHITE).monospace().strong(),
                            )
                            .selectable(false)
                            .sense(egui::Sense::drag()),
                        )
                        .on_hover_cursor(CursorIcon::Grab)
                        .on_hover_text("拖动参数位置");
                    ui.add(egui::Label::new(label).selectable(false));
                    handle
                });
                if row_response.inner.drag_started_by(PointerButton::Primary) {
                    self.dragging_param = Some(row.clone());
                }
                if self.dragging_param.as_ref() == Some(row) {
                    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                    let row_rect = row_response.response.rect;
                    let selected_rect = egui::Rect::from_min_max(
                        egui::pos2(row_rect.left() - 3.0, row_rect.top() - 3.0),
                        egui::pos2(ui.max_rect().right(), row_rect.bottom() + 3.0),
                    );
                    ui.painter().rect_stroke(
                        selected_rect,
                        CornerRadius::same(4),
                        Stroke::new(2.0_f32, AZURE),
                        egui::StrokeKind::Inside,
                    );
                }
                if let (Some(dragged), Some(pointer)) =
                    (self.dragging_param.as_ref(), pointer_position)
                {
                    if row_response.response.rect.contains(pointer)
                        && !(section == ParamSection::Extra
                            && matches!(dragged, ParamRow::Builtin(_)))
                    {
                        let insert_after = pointer.y > row_response.response.rect.center().y;
                        let target_index = index + usize::from(insert_after);
                        *drop_target = Some(ParamDropTarget {
                            section,
                            index: target_index,
                        });
                        let y = if insert_after {
                            row_response.response.rect.bottom()
                        } else {
                            row_response.response.rect.top()
                        };
                        ui.painter().line_segment(
                            [
                                egui::pos2(row_response.response.rect.left(), y),
                                egui::pos2(row_response.response.rect.right(), y),
                            ],
                            Stroke::new(2.0_f32, AZURE),
                        );
                    }
                }
                ui.separator();
            }
            let append_target =
                ui.allocate_response(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
            if let (Some(dragged), Some(pointer)) = (self.dragging_param.as_ref(), pointer_position)
            {
                if append_target.rect.contains(pointer)
                    && !(section == ParamSection::Extra && matches!(dragged, ParamRow::Builtin(_)))
                {
                    *drop_target = Some(ParamDropTarget {
                        section,
                        index: rows.len(),
                    });
                    let y = append_target.rect.top();
                    ui.painter().line_segment(
                        [
                            egui::pos2(append_target.rect.left(), y),
                            egui::pos2(append_target.rect.right(), y),
                        ],
                        Stroke::new(2.0_f32, AZURE),
                    );
                }
            }
        });
    }

    fn apply_param_move(&mut self, dragged: ParamRow, target: ParamDropTarget) {
        let mut sections = [
            (
                ParamSection::Common,
                self.section_rows(ParamSection::Common),
            ),
            (ParamSection::Other, self.section_rows(ParamSection::Other)),
            (ParamSection::Extra, self.section_rows(ParamSection::Extra)),
        ];
        if !move_param_row(&mut sections, &dragged, target) {
            return;
        }

        self.preset.parameter_layout.common = sections[0]
            .1
            .iter()
            .filter_map(|row| match row {
                ParamRow::Builtin(key) => Some(key.clone()),
                ParamRow::Extra(_) => None,
            })
            .collect();
        self.preset.parameter_layout.other = sections[1]
            .1
            .iter()
            .filter_map(|row| match row {
                ParamRow::Builtin(key) => Some(key.clone()),
                ParamRow::Extra(_) => None,
            })
            .collect();
        for (section, rows) in sections {
            for (position, row) in rows.into_iter().enumerate() {
                if let ParamRow::Extra(id) = row {
                    if let Some(item) = self
                        .preset
                        .extra_params
                        .iter_mut()
                        .find(|item| item.id == id)
                    {
                        item.section = section;
                        item.position = position;
                    }
                }
            }
        }
    }

    fn params_toolbar_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new(if self.customize_param_layout {
                    "自定义参数布局"
                } else {
                    "参数"
                })
                .size(17.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.customize_param_layout {
                    if Self::small_button(ui, "完成").clicked() {
                        self.customize_param_layout = false;
                        self.dragging_param = None;
                        self.persist_active_layout();
                    }
                    if Self::small_button(ui, "恢复默认").clicked() {
                        self.restore_active_layout();
                    }
                } else if Self::small_button(ui, "自定义布局").clicked() {
                    self.customize_param_layout = true;
                }
            });
        });
    }

    fn params_content_ui(&mut self, ui: &mut egui::Ui) {
        if self.customize_param_layout {
            if self.dragging_param.is_some() {
                let viewport = ui.clip_rect();
                if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                    if pointer.x >= viewport.left() - 24.0 && pointer.x <= viewport.right() + 24.0 {
                        let delta_time = ui.ctx().input(|input| input.stable_dt);
                        let scroll_delta = drag_auto_scroll_delta(
                            pointer.y,
                            viewport.top(),
                            viewport.bottom(),
                            delta_time,
                        );
                        if scroll_delta != 0.0 {
                            ui.scroll_with_delta_animation(
                                egui::vec2(0.0, scroll_delta),
                                egui::style::ScrollAnimation::none(),
                            );
                            ui.ctx().request_repaint();
                        }
                    }
                }
            }
            let mut drop_target = None;
            self.render_layout_section(ui, "常用", ParamSection::Common, &mut drop_target);
            ui.add_space(8.0);
            self.render_layout_section(ui, "其他参数", ParamSection::Other, &mut drop_target);
            ui.add_space(8.0);
            self.render_layout_section(ui, "额外参数", ParamSection::Extra, &mut drop_target);
            if ui
                .ctx()
                .input(|input| input.pointer.button_released(PointerButton::Primary))
            {
                if let (Some(dragged), Some(target)) = (self.dragging_param.take(), drop_target) {
                    self.apply_param_move(dragged, target);
                } else {
                    self.dragging_param = None;
                }
            }
        } else {
            self.render_normal_section(ui, "常用", ParamSection::Common);
            self.render_normal_section(ui, "其他参数", ParamSection::Other);
            self.render_normal_section(ui, "额外参数", ParamSection::Extra);
        }
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
                        1.0_f32,
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
                let model_card_height = ui.available_height().max(120.0);
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
                                    .corner_radius(CornerRadius::same(6))
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
                    let column_height = ui.available_height().max(160.0);

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
                                        self.params_toolbar_ui(ui);
                                        ui.add_space(6.0);
                                        ui.separator();
                                        ui.add_space(4.0);
                                        let content_height =
                                            (params_height - ui.min_rect().height()).max(180.0);
                                        egui::ScrollArea::vertical()
                                            .max_height(content_height)
                                            .show(ui, |ui| {
                                                self.params_content_ui(ui);
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
                                    let preview_height = (ui.available_height() - 36.0).max(120.0);
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
        self.show_param_reference(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin(name: &str) -> ParamRow {
        ParamRow::Builtin(name.to_string())
    }

    fn sample_sections() -> [(ParamSection, Vec<ParamRow>); 3] {
        [
            (
                ParamSection::Common,
                vec![builtin("a"), builtin("b"), builtin("c")],
            ),
            (ParamSection::Other, vec![builtin("d")]),
            (ParamSection::Extra, Vec::new()),
        ]
    }

    #[test]
    fn moves_rows_up_and_down_in_the_same_section() {
        let mut sections = sample_sections();
        assert!(move_param_row(
            &mut sections,
            &builtin("a"),
            ParamDropTarget {
                section: ParamSection::Common,
                index: 3,
            },
        ));
        assert_eq!(
            sections[0].1,
            vec![builtin("b"), builtin("c"), builtin("a")]
        );

        assert!(move_param_row(
            &mut sections,
            &builtin("a"),
            ParamDropTarget {
                section: ParamSection::Common,
                index: 0,
            },
        ));
        assert_eq!(
            sections[0].1,
            vec![builtin("a"), builtin("b"), builtin("c")]
        );
    }

    #[test]
    fn moves_rows_between_sections_and_appends_to_empty_section() {
        let mut sections = sample_sections();
        assert!(move_param_row(
            &mut sections,
            &builtin("b"),
            ParamDropTarget {
                section: ParamSection::Other,
                index: 1,
            },
        ));
        assert_eq!(sections[0].1, vec![builtin("a"), builtin("c")]);
        assert_eq!(sections[1].1, vec![builtin("d"), builtin("b")]);

        sections[0].1.push(ParamRow::Extra(7));
        assert!(move_param_row(
            &mut sections,
            &ParamRow::Extra(7),
            ParamDropTarget {
                section: ParamSection::Extra,
                index: 0,
            },
        ));
        assert_eq!(sections[2].1, vec![ParamRow::Extra(7)]);
    }

    #[test]
    fn rejects_invalid_moves_and_missing_rows() {
        let mut sections = sample_sections();
        let original = sections.clone();
        assert!(!move_param_row(
            &mut sections,
            &builtin("a"),
            ParamDropTarget {
                section: ParamSection::Extra,
                index: 0,
            },
        ));
        assert!(!move_param_row(
            &mut sections,
            &builtin("missing"),
            ParamDropTarget {
                section: ParamSection::Other,
                index: 0,
            },
        ));
        assert_eq!(sections, original);
    }

    #[test]
    fn a_move_collapses_duplicate_source_rows() {
        let mut sections = sample_sections();
        sections[1].1.push(builtin("a"));
        assert!(move_param_row(
            &mut sections,
            &builtin("a"),
            ParamDropTarget {
                section: ParamSection::Other,
                index: 0,
            },
        ));
        let count = sections
            .iter()
            .flat_map(|(_, rows)| rows)
            .filter(|row| **row == builtin("a"))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn copying_layout_does_not_overwrite_parameter_values() {
        let mut source = Preset::default();
        source.port = "9000".to_string();
        source.parameter_layout.common.swap(0, 1);
        source.extra_params.push(ExtraParam {
            id: 7,
            text: "--source".to_string(),
            section: ParamSection::Common,
            position: 1,
            ..ExtraParam::default()
        });
        let mut target = Preset::default();
        target.port = "8081".to_string();
        target.extra_params.push(ExtraParam {
            id: 7,
            text: "--target".to_string(),
            ..ExtraParam::default()
        });

        copy_preset_layout(&source, &mut target);

        assert_eq!(target.port, "8081");
        assert_eq!(target.extra_params[0].text, "--target");
        assert_eq!(target.extra_params[0].section, ParamSection::Common);
        assert_eq!(target.extra_params[0].position, 1);
        assert_eq!(target.parameter_layout, source.parameter_layout);
    }

    #[test]
    fn restoring_one_preset_does_not_change_another() {
        let mut active = Preset::default();
        active.parameter_layout.common.swap(0, 1);
        active.extra_params.push(ExtraParam {
            id: 1,
            section: ParamSection::Common,
            position: 0,
            ..ExtraParam::default()
        });
        let other = active.clone();

        restore_preset_layout(&mut active);

        assert_eq!(active.parameter_layout, ParameterLayout::default());
        assert_eq!(active.extra_params[0].section, ParamSection::Extra);
        assert_ne!(active.parameter_layout, other.parameter_layout);
        assert_eq!(other.extra_params[0].section, ParamSection::Common);
    }

    #[test]
    fn drag_auto_scrolls_toward_nearby_edges() {
        let top_delta = drag_auto_scroll_delta(4.0, 0.0, 300.0, 0.01);
        let bottom_delta = drag_auto_scroll_delta(296.0, 0.0, 300.0, 0.01);

        assert!(top_delta > 0.0);
        assert!(bottom_delta < 0.0);
        assert_eq!(drag_auto_scroll_delta(150.0, 0.0, 300.0, 0.01), 0.0);
        assert!(top_delta.abs() <= 7.6);
        assert!(bottom_delta.abs() <= 7.6);
    }
}
