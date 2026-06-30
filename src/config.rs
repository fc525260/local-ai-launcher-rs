use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

const CONFIG_FILE: &str = "local-ai-launcher-config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub ngl: String,
    pub n_cpu_moe: String,
    pub threads: String,
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
    pub web_ui: bool,
    pub log_timestamps: bool,
    pub offline: bool,
    pub verbose: bool,
    pub kv_offload: bool,
    pub mlock: bool,
    pub mmap: bool,
    pub kv_unified: bool,
    pub swa_full: bool,
    pub cpu_moe: bool,
    pub extra_args: String,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            ngl: "100".to_string(),
            n_cpu_moe: "999".to_string(),
            threads: "12".to_string(),
            batch_size: "512".to_string(),
            ubatch_size: "256".to_string(),
            parallel: "1".to_string(),
            ctx_size: "65536".to_string(),
            timeout: "3600".to_string(),
            alias: String::new(),
            cache_type_k: String::new(),
            cache_type_v: String::new(),
            spec_type: String::new(),
            spec_draft_n_max: String::new(),
            spec_draft_n_min: String::new(),
            spec_draft_p_min: String::new(),
            spec_draft_p_split: String::new(),
            image_min_tokens: String::new(),
            image_max_tokens: String::new(),
            host: "127.0.0.1".to_string(),
            port: "8080".to_string(),
            split_mode: String::new(),
            tensor_split: String::new(),
            main_gpu: String::new(),
            device: String::new(),
            web_ui: true,
            log_timestamps: true,
            offline: false,
            verbose: false,
            kv_offload: true,
            mlock: false,
            mmap: true,
            kv_unified: false,
            swa_full: false,
            cpu_moe: false,
            extra_args: String::new(),
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
pub struct AppConfig {
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
    let mut config: AppConfig = serde_json::from_str(&text).unwrap_or_default();
    normalize_presets(&mut config);
    config
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
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}
