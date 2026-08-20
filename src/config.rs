use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

const CONFIG_FILE: &str = "local-ai-launcher-config.json";
const CONFIG_VERSION: u32 = 3;

pub const LOAD_MODES: &[&str] = &["none", "mmap", "mlock", "mmap+mlock", "dio"];
pub const FLASH_ATTN_MODES: &[&str] = &["on", "off", "auto"];
pub const SPEC_TYPES: &[&str] = &[
    "none",
    "draft-simple",
    "draft-eagle3",
    "draft-mtp",
    "draft-dflash",
    "draft-dspark",
    "ngram-simple",
    "ngram-map-k",
    "ngram-map-k4v",
    "ngram-mod",
    "ngram-cache",
];

pub const DEFAULT_COMMON_PARAM_IDS: &[&str] = &[
    "host",
    "port",
    "alias",
    "ngl",
    "n_cpu_moe",
    "threads",
    "batch_size",
    "ubatch_size",
    "parallel",
    "ctx_size",
    "cache_type_k",
    "cache_type_v",
    "load_mode",
    "flash_attn",
    "spec_type",
    "spec_draft_n_max",
    "spec_draft_n_min",
    "spec_draft_p_min",
    "spec_draft_p_split",
    "image_min_tokens",
    "image_max_tokens",
    "jinja",
];

pub const DEFAULT_OTHER_PARAM_IDS: &[&str] = &[
    "timeout",
    "split_mode",
    "tensor_split",
    "main_gpu",
    "device",
    "web_ui",
    "log_timestamps",
    "offline",
    "verbose",
    "kv_offload",
    "kv_unified",
    "swa_full",
    "cpu_moe",
    "reasoning_preserve",
    "threads_batch",
    "predict",
    "keep",
    "rope_scaling",
    "rope_scale",
    "rope_freq_base",
    "rope_freq_scale",
    "yarn_orig_ctx",
    "rpc",
    "numa",
    "fit",
    "check_tensors",
    "log_file",
    "spec_draft_threads",
    "spec_draft_ngl",
    "spec_draft_device",
    "seed",
    "temperature",
    "top_k",
    "top_p",
    "min_p",
    "repeat_last_n",
    "repeat_penalty",
    "presence_penalty",
    "frequency_penalty",
    "mirostat",
    "mirostat_lr",
    "mirostat_ent",
    "grammar_file",
    "json_schema",
    "system_prompt",
    "reverse_prompt",
    "context_shift",
    "show_timings",
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToggleState {
    #[default]
    Default,
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamSection {
    Common,
    Other,
    #[default]
    Extra,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtraParamKind {
    #[default]
    FreeText,
    FlagValue,
    FlagToggle,
    FlagChoice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtraParam {
    pub id: u64,
    pub text: String,
    pub enabled: bool,
    pub section: ParamSection,
    pub position: usize,
    pub kind: ExtraParamKind,
    pub flag: String,
    pub value: String,
    pub choices: Vec<String>,
}

impl Default for ExtraParam {
    fn default() -> Self {
        Self {
            id: 0,
            text: String::new(),
            enabled: true,
            section: ParamSection::Extra,
            position: usize::MAX,
            kind: ExtraParamKind::FreeText,
            flag: String::new(),
            value: String::new(),
            choices: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParameterLayout {
    pub common: Vec<String>,
    pub other: Vec<String>,
}

impl Default for ParameterLayout {
    fn default() -> Self {
        Self {
            common: DEFAULT_COMMON_PARAM_IDS
                .iter()
                .map(|id| (*id).to_string())
                .collect(),
            other: DEFAULT_OTHER_PARAM_IDS
                .iter()
                .map(|id| (*id).to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preset {
    pub ngl: String,
    pub n_cpu_moe: String,
    pub threads: String,
    pub threads_batch: String,
    pub predict: String,
    pub keep: String,
    pub batch_size: String,
    pub ubatch_size: String,
    pub parallel: String,
    pub ctx_size: String,
    pub timeout: String,
    pub alias: String,
    pub cache_type_k: String,
    pub cache_type_v: String,
    pub spec_type: String,
    pub spec_draft_n_max: String,
    pub spec_draft_n_min: String,
    pub spec_draft_threads: String,
    pub spec_draft_ngl: String,
    pub spec_draft_device: String,
    pub spec_draft_p_min: String,
    pub spec_draft_p_split: String,
    pub image_min_tokens: String,
    pub image_max_tokens: String,
    pub host: String,
    pub port: String,
    pub split_mode: String,
    pub tensor_split: String,
    pub main_gpu: String,
    pub device: String,
    pub load_mode: String,
    pub flash_attn: String,
    pub rope_scaling: String,
    pub rope_scale: String,
    pub rope_freq_base: String,
    pub rope_freq_scale: String,
    pub yarn_orig_ctx: String,
    pub rpc: String,
    pub numa: String,
    pub fit: String,
    pub log_file: String,
    pub seed: String,
    pub temperature: String,
    pub top_k: String,
    pub top_p: String,
    pub min_p: String,
    pub repeat_last_n: String,
    pub repeat_penalty: String,
    pub presence_penalty: String,
    pub frequency_penalty: String,
    pub mirostat: String,
    pub mirostat_lr: String,
    pub mirostat_ent: String,
    pub grammar_file: String,
    pub json_schema: String,
    pub system_prompt: String,
    pub reverse_prompt: String,
    pub web_ui: ToggleState,
    pub log_timestamps: ToggleState,
    pub offline: ToggleState,
    pub verbose: ToggleState,
    pub kv_offload: ToggleState,
    pub kv_unified: ToggleState,
    pub swa_full: ToggleState,
    pub cpu_moe: ToggleState,
    pub jinja: ToggleState,
    pub reasoning_preserve: ToggleState,
    pub check_tensors: ToggleState,
    pub context_shift: ToggleState,
    pub show_timings: ToggleState,
    pub parameter_layout: ParameterLayout,
    pub extra_params: Vec<ExtraParam>,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            ngl: "100".to_string(),
            n_cpu_moe: "999".to_string(),
            threads: "12".to_string(),
            threads_batch: String::new(),
            predict: String::new(),
            keep: String::new(),
            batch_size: "512".to_string(),
            ubatch_size: "256".to_string(),
            parallel: "1".to_string(),
            ctx_size: "65536".to_string(),
            timeout: String::new(),
            alias: String::new(),
            cache_type_k: String::new(),
            cache_type_v: String::new(),
            spec_type: String::new(),
            spec_draft_n_max: String::new(),
            spec_draft_n_min: String::new(),
            spec_draft_threads: String::new(),
            spec_draft_ngl: String::new(),
            spec_draft_device: String::new(),
            spec_draft_p_min: String::new(),
            spec_draft_p_split: String::new(),
            image_min_tokens: String::new(),
            image_max_tokens: String::new(),
            host: "127.0.0.1".to_string(),
            port: "9931".to_string(),
            split_mode: String::new(),
            tensor_split: String::new(),
            main_gpu: String::new(),
            device: String::new(),
            load_mode: "mmap".to_string(),
            flash_attn: "on".to_string(),
            rope_scaling: String::new(),
            rope_scale: String::new(),
            rope_freq_base: String::new(),
            rope_freq_scale: String::new(),
            yarn_orig_ctx: String::new(),
            rpc: String::new(),
            numa: String::new(),
            fit: String::new(),
            log_file: String::new(),
            seed: String::new(),
            temperature: String::new(),
            top_k: String::new(),
            top_p: String::new(),
            min_p: String::new(),
            repeat_last_n: String::new(),
            repeat_penalty: String::new(),
            presence_penalty: String::new(),
            frequency_penalty: String::new(),
            mirostat: String::new(),
            mirostat_lr: String::new(),
            mirostat_ent: String::new(),
            grammar_file: String::new(),
            json_schema: String::new(),
            system_prompt: String::new(),
            reverse_prompt: String::new(),
            web_ui: ToggleState::Default,
            log_timestamps: ToggleState::Default,
            offline: ToggleState::Default,
            verbose: ToggleState::Default,
            kv_offload: ToggleState::Default,
            kv_unified: ToggleState::Default,
            swa_full: ToggleState::Default,
            cpu_moe: ToggleState::Default,
            jinja: ToggleState::Enabled,
            reasoning_preserve: ToggleState::Default,
            check_tensors: ToggleState::Default,
            context_shift: ToggleState::Default,
            show_timings: ToggleState::Default,
            parameter_layout: ParameterLayout::default(),
            extra_params: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    pub top_border: [u8; 4],
    pub model_border: [u8; 4],
    pub preset_border: [u8; 4],
    pub preview_border: [u8; 4],
    pub panel_text: [u8; 4],
    pub weak_text: [u8; 4],
    pub bold_text: bool,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            top_border: [232, 232, 237, 255],
            model_border: [232, 232, 237, 255],
            preset_border: [232, 232, 237, 255],
            preview_border: [232, 232, 237, 255],
            panel_text: [29, 29, 31, 255],
            weak_text: [112, 112, 112, 255],
            bold_text: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub config_version: u32,
    pub llama_cpp_dir: PathBuf,
    pub models_dir: PathBuf,
    pub selected_model: String,
    pub selected_preset: String,
    pub global_presets: BTreeMap<String, Preset>,
    #[serde(default)]
    pub model_aliases: BTreeMap<String, String>,
    #[serde(default)]
    pub hidden_models: Vec<String>,
    #[serde(default)]
    pub manual_models: Vec<ManualModel>,
    #[serde(default)]
    pub model_order: Vec<String>,
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub model_presets: BTreeMap<String, Preset>,
    #[serde(default)]
    pub draft_models: Vec<String>,
    #[serde(default)]
    pub model_draft_overrides: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualModel {
    pub path: PathBuf,
    pub display_name: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut global_presets = BTreeMap::new();
        global_presets.insert("默认".to_string(), Preset::default());

        Self {
            config_version: CONFIG_VERSION,
            llama_cpp_dir: PathBuf::new(),
            models_dir: PathBuf::new(),
            selected_model: String::new(),
            selected_preset: "默认".to_string(),
            global_presets,
            model_aliases: BTreeMap::new(),
            hidden_models: Vec::new(),
            manual_models: Vec::new(),
            model_order: Vec::new(),
            appearance: AppearanceConfig::default(),
            model_presets: BTreeMap::new(),
            draft_models: Vec::new(),
            model_draft_overrides: BTreeMap::new(),
        }
    }
}

pub fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join(CONFIG_FILE)
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        return AppConfig::default();
    }
    let Ok(text) = fs::read_to_string(path) else {
        return AppConfig::default();
    };
    let Ok(mut value) = serde_json::from_str::<Value>(&text) else {
        return AppConfig::default();
    };
    migrate_config_value(&mut value);
    let mut config: AppConfig = serde_json::from_value(value).unwrap_or_default();
    normalize_presets(&mut config);
    config
}

fn migrate_config_value(value: &mut Value) {
    let Some(root) = value.as_object_mut() else {
        return;
    };
    let legacy_layout = root.remove("parameter_layout").unwrap_or_else(|| {
        serde_json::to_value(ParameterLayout::default()).expect("default layout serializes")
    });
    for key in ["global_presets", "model_presets"] {
        if let Some(presets) = root.get_mut(key).and_then(Value::as_object_mut) {
            for preset in presets.values_mut() {
                if let Some(fields) = preset.as_object_mut() {
                    fields
                        .entry("parameter_layout".to_string())
                        .or_insert_with(|| legacy_layout.clone());
                }
                migrate_preset_value(preset);
            }
        }
    }
    root.insert("config_version".to_string(), Value::from(CONFIG_VERSION));
}

fn migrate_preset_value(value: &mut Value) {
    let Some(preset) = value.as_object_mut() else {
        return;
    };

    for field in [
        "web_ui",
        "log_timestamps",
        "offline",
        "verbose",
        "kv_offload",
        "kv_unified",
        "swa_full",
        "cpu_moe",
        "jinja",
    ] {
        let Some(old) = preset.get(field).and_then(Value::as_bool) else {
            continue;
        };
        let state = if old { "enabled" } else { "default" };
        preset.insert(field.to_string(), Value::from(state));
    }

    if !preset.contains_key("load_mode") {
        let legacy_mmap = preset.remove("mmap").and_then(|value| value.as_bool());
        let legacy_mlock = preset.remove("mlock").and_then(|value| value.as_bool());
        if legacy_mmap.is_some() || legacy_mlock.is_some() {
            let load_mode = match (legacy_mmap.unwrap_or(true), legacy_mlock.unwrap_or(false)) {
                (false, false) => "none",
                (true, false) => "mmap",
                (false, true) => "mlock",
                (true, true) => "mmap+mlock",
            };
            preset.insert("load_mode".to_string(), Value::from(load_mode));
        }
    } else {
        preset.remove("mmap");
        preset.remove("mlock");
    }

    if !preset.contains_key("extra_params") {
        let rows = preset
            .remove("extra_args")
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .enumerate()
            .map(|(index, line)| {
                serde_json::json!({
                    "id": index as u64 + 1,
                    "text": line,
                    "enabled": true,
                    "section": "extra",
                    "position": index
                })
            })
            .collect();
        preset.insert("extra_params".to_string(), Value::Array(rows));
    } else {
        preset.remove("extra_args");
    }
}

fn normalize_presets(config: &mut AppConfig) {
    let default = config
        .global_presets
        .get("默认")
        .or_else(|| config.global_presets.get("平衡"))
        .cloned()
        .unwrap_or_default();
    config.global_presets.clear();
    config.global_presets.insert("默认".to_string(), default);
    config.selected_preset = "默认".to_string();
    config.config_version = CONFIG_VERSION;
    for preset in config
        .global_presets
        .values_mut()
        .chain(config.model_presets.values_mut())
    {
        normalize_parameter_layout(&mut preset.parameter_layout);
        normalize_extra_param_ids(preset);
        normalize_preset_choices(preset);
    }
}

fn normalize_preset_choices(preset: &mut Preset) {
    if !preset.load_mode.is_empty() && !LOAD_MODES.contains(&preset.load_mode.as_str()) {
        preset.load_mode.clear();
    }
    if !preset.flash_attn.is_empty() && !FLASH_ATTN_MODES.contains(&preset.flash_attn.as_str()) {
        preset.flash_attn.clear();
    }
    if !preset.spec_type.is_empty() && !SPEC_TYPES.contains(&preset.spec_type.as_str()) {
        preset.spec_type.clear();
    }
    use crate::parameters::{FIT_MODES, MIROSTAT_MODES, NUMA_TYPES, ROPE_SCALINGS};
    if !preset.rope_scaling.is_empty() && !ROPE_SCALINGS.contains(&preset.rope_scaling.as_str()) {
        preset.rope_scaling.clear();
    }
    if !preset.numa.is_empty() && !NUMA_TYPES.contains(&preset.numa.as_str()) {
        preset.numa.clear();
    }
    if !preset.fit.is_empty() && !FIT_MODES.contains(&preset.fit.as_str()) {
        preset.fit.clear();
    }
    if !preset.mirostat.is_empty() && !MIROSTAT_MODES.contains(&preset.mirostat.as_str()) {
        preset.mirostat.clear();
    }
}

fn normalize_parameter_layout(layout: &mut ParameterLayout) {
    let known: std::collections::BTreeSet<&str> = DEFAULT_COMMON_PARAM_IDS
        .iter()
        .chain(DEFAULT_OTHER_PARAM_IDS.iter())
        .copied()
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    layout
        .common
        .retain(|id| known.contains(id.as_str()) && seen.insert(id.clone()));
    layout
        .other
        .retain(|id| known.contains(id.as_str()) && seen.insert(id.clone()));
    for id in DEFAULT_COMMON_PARAM_IDS {
        if seen.insert((*id).to_string()) {
            layout.common.push((*id).to_string());
        }
    }
    for id in DEFAULT_OTHER_PARAM_IDS {
        if seen.insert((*id).to_string()) {
            layout.other.push((*id).to_string());
        }
    }
}

fn normalize_extra_param_ids(preset: &mut Preset) {
    let mut used = std::collections::BTreeSet::new();
    let mut next_id = preset
        .extra_params
        .iter()
        .map(|item| item.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    for item in &mut preset.extra_params {
        if item.id == 0 || !used.insert(item.id) {
            item.id = next_id;
            used.insert(next_id);
            next_id = next_id.saturating_add(1);
        }
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_preset_without_losing_command_semantics() {
        let mut value = serde_json::json!({
            "mmap": true,
            "mlock": true,
            "web_ui": false,
            "log_timestamps": true,
            "offline": false,
            "kv_offload": false,
            "jinja": true,
            "extra_args": "--foo 1\n\n--bar \"two words\""
        });

        migrate_preset_value(&mut value);
        let preset: Preset = serde_json::from_value(value).expect("migrated preset");

        assert_eq!(preset.load_mode, "mmap+mlock");
        assert_eq!(preset.web_ui, ToggleState::Default);
        assert_eq!(preset.log_timestamps, ToggleState::Enabled);
        assert_eq!(preset.offline, ToggleState::Default);
        assert_eq!(preset.kv_offload, ToggleState::Default);
        assert_eq!(preset.jinja, ToggleState::Enabled);
        assert_eq!(preset.extra_params.len(), 2);
        assert_eq!(preset.extra_params[1].text, "--bar \"two words\"");
    }

    #[test]
    fn normalizes_layout_and_appends_new_parameters() {
        let mut layout = ParameterLayout {
            common: vec!["port".into(), "port".into(), "unknown".into()],
            other: vec!["host".into()],
        };

        normalize_parameter_layout(&mut layout);

        assert_eq!(layout.common[0], "port");
        assert_eq!(layout.other[0], "host");
        assert_eq!(
            layout.common.len() + layout.other.len(),
            DEFAULT_COMMON_PARAM_IDS.len() + DEFAULT_OTHER_PARAM_IDS.len()
        );
    }

    #[test]
    fn migrates_root_layout_into_default_and_model_presets() {
        let mut value = serde_json::json!({
            "config_version": 2,
            "parameter_layout": {
                "common": ["port", "host"],
                "other": ["ngl"]
            },
            "global_presets": {
                "默认": {}
            },
            "model_presets": {
                "model-a": {}
            }
        });

        migrate_config_value(&mut value);
        assert!(value.get("parameter_layout").is_none());
        let mut config: AppConfig = serde_json::from_value(value).expect("migrated config");
        normalize_presets(&mut config);

        let default_layout = &config.global_presets["默认"].parameter_layout;
        let model_layout = &config.model_presets["model-a"].parameter_layout;
        assert_eq!(default_layout.common[0..2], ["port", "host"]);
        assert_eq!(model_layout.common[0..2], ["port", "host"]);
        assert_eq!(default_layout.other[0], "ngl");
        assert_eq!(model_layout.other[0], "ngl");
    }

    #[test]
    fn preset_layouts_are_independent() {
        let mut config = AppConfig::default();
        let mut model = Preset::default();
        model.parameter_layout.common.swap(0, 1);
        config.model_presets.insert("model-a".to_string(), model);

        assert_ne!(
            config.global_presets["默认"].parameter_layout,
            config.model_presets["model-a"].parameter_layout
        );
    }

    #[test]
    fn migrates_every_legacy_load_mode_combination() {
        for (mmap, mlock, expected) in [
            (false, false, "none"),
            (true, false, "mmap"),
            (false, true, "mlock"),
            (true, true, "mmap+mlock"),
        ] {
            let mut value = serde_json::json!({"mmap": mmap, "mlock": mlock});
            migrate_preset_value(&mut value);
            assert_eq!(value["load_mode"], expected);
        }
    }

    #[test]
    fn new_preset_only_enables_jinja() {
        let preset = Preset::default();

        assert_eq!(preset.jinja, ToggleState::Enabled);
        assert_eq!(preset.web_ui, ToggleState::Default);
        assert_eq!(preset.log_timestamps, ToggleState::Default);
        assert_eq!(preset.kv_offload, ToggleState::Default);
        assert_eq!(preset.load_mode, "mmap");
        assert_eq!(preset.flash_attn, "on");
    }
}
