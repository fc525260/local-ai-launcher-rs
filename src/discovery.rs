use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{AppConfig, ManualModel};

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub rel_path: String,
    pub display_name: String,
    pub size_label: String,
    pub mmproj: Option<String>,
    pub draft_model: Option<String>,
}

fn is_main_candidate(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if name.contains("mmproj") || name.contains("projector") || name.contains("encoder") {
        return false;
    }
    !(name.starts_with("mtp-")
        || name.starts_with("mtp_")
        || name.starts_with("draft-")
        || name.starts_with("draft_"))
}

fn is_mmproj(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
        .contains("mmproj")
}

fn is_draft(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    name.starts_with("mtp-")
        || name.starts_with("mtp_")
        || name.starts_with("draft-")
        || name.starts_with("draft_")
}

fn size_label(bytes: u64) -> String {
    let gb = bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    if gb >= 1.0 {
        format!("{gb:.1}GB")
    } else {
        format!("{:.0}MB", bytes as f64 / 1024.0 / 1024.0)
    }
}

fn model_id(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn rel_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn manual_model_info(model: &ManualModel) -> ModelInfo {
    let metadata = fs::metadata(&model.path).ok();
    let display_name = if model.display_name.trim().is_empty() {
        model
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("manual-model")
            .to_string()
    } else {
        model.display_name.clone()
    };
    ModelInfo {
        id: model_id(&model.path),
        rel_path: model.path.to_string_lossy().replace('\\', "/"),
        display_name,
        size_label: metadata.map(|m| size_label(m.len())).unwrap_or_default(),
        mmproj: None,
        draft_model: None,
    }
}

fn collect_gguf(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_gguf(&path, out);
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("gguf"))
        {
            out.push(path);
        }
    }
}

pub fn discover_models(models_dir: &Path) -> Vec<ModelInfo> {
    let mut files = Vec::new();
    if models_dir.exists() {
        collect_gguf(models_dir, &mut files);
    }
    files.sort();

    let mut result = Vec::new();
    for path in files.iter().filter(|p| is_main_candidate(p)) {
        let parent = path.parent().unwrap_or(models_dir);
        let siblings: Vec<PathBuf> = files
            .iter()
            .filter(|p| p.parent() == Some(parent))
            .cloned()
            .collect();
        let mmproj = siblings
            .iter()
            .find(|p| is_mmproj(p))
            .map(|p| rel_path(p, models_dir));
        let draft_model = siblings
            .iter()
            .find(|p| *p != path && is_draft(p))
            .map(|p| rel_path(p, models_dir));
        let metadata = fs::metadata(path).ok();
        result.push(ModelInfo {
            id: model_id(path),
            rel_path: rel_path(path, models_dir),
            display_name: path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("model")
                .to_string(),
            size_label: metadata.map(|m| size_label(m.len())).unwrap_or_default(),
            mmproj,
            draft_model,
        });
    }
    result
}

pub fn discover_configured_models(config: &AppConfig) -> Vec<ModelInfo> {
    let hidden: std::collections::BTreeSet<String> = config
        .hidden_models
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    let mut result: Vec<ModelInfo> = discover_models(&config.models_dir)
        .into_iter()
        .filter(|model| !hidden.contains(&model.id))
        .map(|mut model| {
            if let Some(alias) = config.model_aliases.get(&model.id) {
                if !alias.trim().is_empty() {
                    model.display_name = alias.clone();
                }
            }
            model
        })
        .collect();

    for manual in &config.manual_models {
        let mut model = manual_model_info(manual);
        if hidden.contains(&model.id) || result.iter().any(|existing| existing.id == model.id) {
            continue;
        }
        if let Some(alias) = config.model_aliases.get(&model.id) {
            if !alias.trim().is_empty() {
                model.display_name = alias.clone();
            }
        }
        result.push(model);
    }

    result.sort_by(|a, b| {
        let a_order = config.model_order.iter().position(|id| id == &a.id);
        let b_order = config.model_order.iter().position(|id| id == &b.id);
        match (a_order, b_order) {
            (Some(a_idx), Some(b_idx)) => a_idx.cmp(&b_idx),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.display_name.cmp(&b.display_name),
        }
    });
    result
}
