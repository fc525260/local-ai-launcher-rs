use crate::config::{Preset, ToggleState};
use crate::discovery::ModelInfo;
use crate::parameters::{parameter, ParamId};
use std::path::{Path, PathBuf};

fn push_pair(args: &mut Vec<String>, flag: &str, value: &str) {
    if !value.trim().is_empty() {
        args.push(flag.to_string());
        args.push(value.trim().to_string());
    }
}

fn push_param_pair(args: &mut Vec<String>, id: ParamId, value: &str) {
    push_pair(args, parameter(id).flag, value);
}

fn model_arg(models_dir: &Path, rel: &str) -> String {
    let candidate = Path::new(rel);
    if candidate.is_absolute() {
        return candidate.to_string_lossy().to_string();
    }
    models_dir
        .join(rel.replace('/', "\\"))
        .to_string_lossy()
        .to_string()
}

fn is_mtp_draft(rel: &str) -> bool {
    Path::new(rel)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|name| {
            let name = name.to_lowercase();
            name.starts_with("mtp-") || name.starts_with("mtp_")
        })
        .unwrap_or(false)
}

fn push_toggle(args: &mut Vec<String>, state: ToggleState, id: ParamId) {
    let definition = parameter(id);
    match state {
        ToggleState::Default => {}
        ToggleState::Enabled => args.push(definition.flag.to_string()),
        ToggleState::Disabled => {
            if let Some(flag) = definition.negative_flag {
                args.push(flag.to_string());
            }
        }
    }
}

pub fn llama_server_path(llama_cpp_dir: &Path) -> PathBuf {
    llama_cpp_dir.join("llama-server.exe")
}

pub fn build_args(
    model: &ModelInfo,
    preset: &Preset,
    models_dir: &Path,
    use_mm: bool,
    draft_override: Option<&str>,
) -> Vec<String> {
    let mut args = Vec::new();
    args.push(
        llama_server_path(Path::new(""))
            .to_string_lossy()
            .to_string(),
    );
    args.push("--model".to_string());
    args.push(model_arg(models_dir, &model.rel_path));

    if use_mm {
        if let Some(mmproj) = &model.mmproj {
            args.push("--mmproj".to_string());
            args.push(model_arg(models_dir, mmproj));
        }
    }
    let draft_model = draft_override.or(model.draft_model.as_deref());
    if let Some(draft) = draft_model {
        args.push("--spec-draft-model".to_string());
        args.push(model_arg(models_dir, draft));
    }
    let mtp_defaults = draft_model.is_some_and(is_mtp_draft);

    push_param_pair(&mut args, ParamId::Ngl, &preset.ngl);
    if preset.cpu_moe == ToggleState::Enabled {
        push_toggle(&mut args, preset.cpu_moe, ParamId::CpuMoe);
    } else {
        push_param_pair(&mut args, ParamId::NCpuMoe, &preset.n_cpu_moe);
    }
    push_param_pair(&mut args, ParamId::Threads, &preset.threads);
    push_param_pair(&mut args, ParamId::BatchSize, &preset.batch_size);
    push_param_pair(&mut args, ParamId::UbatchSize, &preset.ubatch_size);
    push_param_pair(&mut args, ParamId::Parallel, &preset.parallel);
    push_param_pair(&mut args, ParamId::CtxSize, &preset.ctx_size);
    push_param_pair(&mut args, ParamId::Timeout, &preset.timeout);
    push_param_pair(&mut args, ParamId::Alias, &preset.alias);
    push_param_pair(&mut args, ParamId::CacheTypeK, &preset.cache_type_k);
    push_param_pair(&mut args, ParamId::CacheTypeV, &preset.cache_type_v);
    if preset.spec_type.trim().is_empty() && mtp_defaults {
        push_param_pair(&mut args, ParamId::SpecType, "draft-mtp");
    } else {
        push_param_pair(&mut args, ParamId::SpecType, &preset.spec_type);
    }
    push_param_pair(&mut args, ParamId::SpecDraftNMax, &preset.spec_draft_n_max);
    push_param_pair(&mut args, ParamId::SpecDraftNMin, &preset.spec_draft_n_min);
    push_param_pair(&mut args, ParamId::SpecDraftPMin, &preset.spec_draft_p_min);
    push_param_pair(
        &mut args,
        ParamId::SpecDraftPSplit,
        &preset.spec_draft_p_split,
    );
    if use_mm {
        push_param_pair(&mut args, ParamId::ImageMinTokens, &preset.image_min_tokens);
        push_param_pair(&mut args, ParamId::ImageMaxTokens, &preset.image_max_tokens);
    }
    push_param_pair(&mut args, ParamId::Host, &preset.host);
    push_param_pair(&mut args, ParamId::Port, &preset.port);
    push_param_pair(&mut args, ParamId::SplitMode, &preset.split_mode);
    push_param_pair(&mut args, ParamId::TensorSplit, &preset.tensor_split);
    push_param_pair(&mut args, ParamId::MainGpu, &preset.main_gpu);
    push_param_pair(&mut args, ParamId::Device, &preset.device);
    push_param_pair(&mut args, ParamId::LoadMode, &preset.load_mode);
    push_param_pair(&mut args, ParamId::FlashAttn, &preset.flash_attn);

    push_toggle(&mut args, preset.web_ui, ParamId::WebUi);
    push_toggle(&mut args, preset.log_timestamps, ParamId::LogTimestamps);
    push_toggle(&mut args, preset.offline, ParamId::Offline);
    push_toggle(&mut args, preset.verbose, ParamId::Verbose);
    push_toggle(&mut args, preset.kv_offload, ParamId::KvOffload);
    push_toggle(&mut args, preset.kv_unified, ParamId::KvUnified);
    push_toggle(&mut args, preset.swa_full, ParamId::SwaFull);
    push_toggle(&mut args, preset.jinja, ParamId::Jinja);
    push_toggle(
        &mut args,
        preset.reasoning_preserve,
        ParamId::ReasoningPreserve,
    );
    for item in preset
        .extra_params
        .iter()
        .filter(|item| item.enabled && !item.text.trim().is_empty())
    {
        if let Ok(parts) = parse_extra_args(&item.text) {
            args.extend(parts);
        }
    }
    args
}

pub fn parse_extra_args(line: &str) -> Result<Vec<String>, &'static str> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape = false;

    for ch in line.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_quotes => escape = true,
            '"' => in_quotes = !in_quotes,
            ch if ch.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if escape {
        current.push('\\');
    }
    if in_quotes {
        return Err("双引号未闭合");
    }
    if !current.is_empty() {
        result.push(current);
    }
    Ok(result)
}

pub fn command_preview(args: &[String], llama_cpp_dir: &Path) -> String {
    if args.is_empty() {
        return String::new();
    }
    let mut display = args.to_vec();
    display[0] = llama_server_path(llama_cpp_dir)
        .to_string_lossy()
        .to_string();

    let mut lines = vec![format!("\"{}\"", display[0])];
    let mut idx = 1;
    while idx < display.len() {
        let current = &display[idx];
        if current.starts_with('-') && idx + 1 < display.len() && !display[idx + 1].starts_with('-')
        {
            lines.push(format!(
                "  {} {}",
                current,
                quote_arg_for_flag(current, &display[idx + 1])
            ));
            idx += 2;
        } else {
            lines.push(format!("  {}", quote_arg(current)));
            idx += 1;
        }
    }

    lines.join(" ^\n")
}

pub fn bat_script(args: &[String], llama_cpp_dir: &Path) -> String {
    let command = command_preview(args, llama_cpp_dir);
    format!(
        "@echo off\r\ncd /d \"{}\"\r\n{}\r\npause\r\n",
        llama_cpp_dir.display(),
        command
    )
}

fn quote_arg(value: &str) -> String {
    if should_quote_arg(value) {
        format!("\"{value}\"")
    } else {
        value.to_string()
    }
}

fn quote_arg_for_flag(flag: &str, value: &str) -> String {
    if matches!(flag, "--model" | "--spec-draft-model" | "--mmproj") {
        format!("\"{value}\"")
    } else {
        quote_arg(value)
    }
}

fn should_quote_arg(value: &str) -> bool {
    value.contains(' ') || value.contains('&') || value.contains('(') || value.contains(')')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_extra_args_by_whitespace() {
        assert_eq!(parse_extra_args("-c 1").unwrap(), vec!["-c", "1"]);
    }

    #[test]
    fn keeps_quoted_extra_arg_values_together() {
        assert_eq!(
            parse_extra_args("--alias \"local model\"").unwrap(),
            vec!["--alias", "local model"]
        );
    }

    #[test]
    fn rejects_unclosed_extra_arg_quotes() {
        assert_eq!(
            parse_extra_args("--alias \"local model"),
            Err("双引号未闭合")
        );
    }

    #[test]
    fn previews_split_extra_args_without_wrapping_pair_as_single_arg() {
        let preview = command_preview(
            &[
                "llama-server.exe".to_string(),
                "--model".to_string(),
                "model.gguf".to_string(),
                "--ctx-size".to_string(),
                "1".to_string(),
            ],
            Path::new("C:\\llama"),
        );
        assert!(preview.contains("  --ctx-size 1"));
        assert!(!preview.contains("\"--ctx-size 1\""));
    }

    #[test]
    fn quotes_model_paths_after_model_flags() {
        let preview = command_preview(
            &[
                "llama-server.exe".to_string(),
                "--model".to_string(),
                "model.gguf".to_string(),
                "--spec-draft-model".to_string(),
                "draft.gguf".to_string(),
                "--mmproj".to_string(),
                "mmproj.gguf".to_string(),
            ],
            Path::new("C:\\llama"),
        );
        assert!(preview.contains("  --model \"model.gguf\""));
        assert!(preview.contains("  --spec-draft-model \"draft.gguf\""));
        assert!(preview.contains("  --mmproj \"mmproj.gguf\""));
    }

    #[test]
    fn build_args_uses_long_flags_and_draft_override() {
        let model = ModelInfo {
            id: "main".to_string(),
            rel_path: "main.gguf".to_string(),
            display_name: "main".to_string(),
            size_label: String::new(),
            mmproj: None,
            draft_model: Some("auto-draft.gguf".to_string()),
        };
        let args = build_args(
            &model,
            &Preset::default(),
            Path::new("C:\\models"),
            false,
            Some("manual-draft.gguf"),
        );

        assert!(args
            .windows(2)
            .any(|pair| pair == ["--model", "C:\\models\\main.gguf"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--spec-draft-model", "C:\\models\\manual-draft.gguf"]));
        assert!(args.contains(&"--gpu-layers".to_string()));
        assert!(!args.contains(&"-md".to_string()));
        assert!(!args.contains(&"-m".to_string()));
    }

    #[test]
    fn mtp_draft_adds_only_speculative_type_when_empty() {
        let model = ModelInfo {
            id: "main".to_string(),
            rel_path: "main.gguf".to_string(),
            display_name: "main".to_string(),
            size_label: String::new(),
            mmproj: None,
            draft_model: Some("mtp-draft.gguf".to_string()),
        };
        let args = build_args(
            &model,
            &Preset::default(),
            Path::new("C:\\models"),
            false,
            None,
        );

        assert!(args
            .windows(2)
            .any(|pair| pair == ["--spec-type", "draft-mtp"]));
        assert!(!args.contains(&"--spec-draft-n-max".to_string()));
        assert!(!args.contains(&"--spec-draft-p-min".to_string()));
        assert!(args.contains(&"--jinja".to_string()));
    }

    #[test]
    fn user_speculative_values_override_mtp_defaults() {
        let model = ModelInfo {
            id: "main".to_string(),
            rel_path: "main.gguf".to_string(),
            display_name: "main".to_string(),
            size_label: String::new(),
            mmproj: None,
            draft_model: Some("mtp-draft.gguf".to_string()),
        };
        let preset = Preset {
            spec_type: "draft-simple".to_string(),
            spec_draft_n_max: "5".to_string(),
            spec_draft_p_min: "0.5".to_string(),
            ..Default::default()
        };
        let args = build_args(&model, &preset, Path::new("C:\\models"), false, None);

        assert!(args
            .windows(2)
            .any(|pair| pair == ["--spec-type", "draft-simple"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--spec-draft-n-max", "5"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--spec-draft-p-min", "0.5"]));
    }

    #[test]
    fn emits_current_load_flash_and_reasoning_parameters() {
        let model = ModelInfo {
            id: "main".to_string(),
            rel_path: "main.gguf".to_string(),
            display_name: "main".to_string(),
            size_label: String::new(),
            mmproj: None,
            draft_model: None,
        };
        let preset = Preset {
            load_mode: "mmap+mlock".to_string(),
            flash_attn: "on".to_string(),
            reasoning_preserve: ToggleState::Enabled,
            ..Default::default()
        };

        let args = build_args(&model, &preset, Path::new("C:\\models"), false, None);

        assert!(args
            .windows(2)
            .any(|pair| pair == ["--load-mode", "mmap+mlock"]));
        assert!(args.windows(2).any(|pair| pair == ["--flash-attn", "on"]));
        assert!(args.contains(&"--reasoning-preserve".to_string()));
        assert!(!args.contains(&"--mmap".to_string()));
        assert!(!args.contains(&"--mlock".to_string()));
    }

    #[test]
    fn all_cpu_moe_suppresses_layer_count() {
        let model = ModelInfo {
            id: "main".to_string(),
            rel_path: "main.gguf".to_string(),
            display_name: "main".to_string(),
            size_label: String::new(),
            mmproj: None,
            draft_model: None,
        };
        let preset = Preset {
            n_cpu_moe: "12".to_string(),
            cpu_moe: ToggleState::Enabled,
            ..Default::default()
        };

        let args = build_args(&model, &preset, Path::new("C:\\models"), false, None);

        assert!(args.contains(&"--cpu-moe".to_string()));
        assert!(!args.contains(&"--n-cpu-moe".to_string()));
    }

    #[test]
    fn emits_every_supported_speculative_type() {
        let model = ModelInfo {
            id: "main".to_string(),
            rel_path: "main.gguf".to_string(),
            display_name: "main".to_string(),
            size_label: String::new(),
            mmproj: None,
            draft_model: None,
        };

        for spec_type in crate::config::SPEC_TYPES {
            let preset = Preset {
                spec_type: (*spec_type).to_string(),
                ..Default::default()
            };
            let args = build_args(&model, &preset, Path::new("C:\\models"), false, None);
            assert!(args
                .windows(2)
                .any(|pair| pair == ["--spec-type", *spec_type]));
        }
    }
}
