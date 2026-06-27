use crate::config::Preset;
use crate::discovery::ModelInfo;
use std::path::{Path, PathBuf};

fn push_pair(args: &mut Vec<String>, flag: &str, value: &str) {
    if !value.trim().is_empty() {
        args.push(flag.to_string());
        args.push(value.trim().to_string());
    }
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

pub fn llama_server_path(llama_cpp_dir: &Path) -> PathBuf {
    llama_cpp_dir.join("llama-server.exe")
}

pub fn build_args(
    model: &ModelInfo,
    preset: &Preset,
    models_dir: &Path,
    use_mm: bool,
) -> Vec<String> {
    let mut args = Vec::new();
    args.push(
        llama_server_path(Path::new(""))
            .to_string_lossy()
            .to_string(),
    );
    args.push("-m".to_string());
    args.push(model_arg(models_dir, &model.rel_path));

    if use_mm {
        if let Some(mmproj) = &model.mmproj {
            args.push("--mmproj".to_string());
            args.push(model_arg(models_dir, mmproj));
        }
    }
    if let Some(draft) = &model.draft_model {
        args.push("-md".to_string());
        args.push(model_arg(models_dir, draft));
    }

    push_pair(&mut args, "-ngl", &preset.ngl);
    push_pair(&mut args, "--n-cpu-moe", &preset.n_cpu_moe);
    push_pair(&mut args, "-t", &preset.threads);
    push_pair(&mut args, "-b", &preset.batch_size);
    push_pair(&mut args, "-ub", &preset.ubatch_size);
    push_pair(&mut args, "-np", &preset.parallel);
    push_pair(&mut args, "-c", &preset.ctx_size);
    push_pair(&mut args, "--timeout", &preset.timeout);
    push_pair(&mut args, "--alias", &preset.alias);
    push_pair(&mut args, "--cache-type-k", &preset.cache_type_k);
    push_pair(&mut args, "--cache-type-v", &preset.cache_type_v);
    push_pair(&mut args, "--spec-type", &preset.spec_type);
    push_pair(&mut args, "--spec-draft-n-max", &preset.spec_draft_n_max);
    push_pair(&mut args, "--spec-draft-n-min", &preset.spec_draft_n_min);
    push_pair(&mut args, "--spec-draft-p-min", &preset.spec_draft_p_min);
    push_pair(
        &mut args,
        "--spec-draft-p-split",
        &preset.spec_draft_p_split,
    );
    if use_mm {
        push_pair(&mut args, "--image-min-tokens", &preset.image_min_tokens);
        push_pair(&mut args, "--image-max-tokens", &preset.image_max_tokens);
    }
    push_pair(&mut args, "--host", &preset.host);
    push_pair(&mut args, "--port", &preset.port);
    push_pair(&mut args, "--split-mode", &preset.split_mode);
    push_pair(&mut args, "--tensor-split", &preset.tensor_split);
    push_pair(&mut args, "--main-gpu", &preset.main_gpu);
    push_pair(&mut args, "--device", &preset.device);

    if preset.web_ui {
        args.push("--ui".to_string());
    } else {
        args.push("--no-ui".to_string());
    }
    if preset.log_timestamps {
        args.push("--log-timestamps".to_string());
    } else {
        args.push("--no-log-timestamps".to_string());
    }
    if preset.offline {
        args.push("--offline".to_string());
    }
    if preset.verbose {
        args.push("--verbose".to_string());
    }
    if preset.kv_offload {
        args.push("--kv-offload".to_string());
    } else {
        args.push("--no-kv-offload".to_string());
    }
    if preset.mlock {
        args.push("--mlock".to_string());
    }
    if preset.mmap {
        args.push("--mmap".to_string());
    } else {
        args.push("--no-mmap".to_string());
    }
    if preset.kv_unified {
        args.push("--kv-unified".to_string());
    }
    if preset.swa_full {
        args.push("--swa-full".to_string());
    }
    if preset.cpu_moe {
        args.push("--cpu-moe".to_string());
    }
    for part in preset
        .extra_args
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .flat_map(split_extra_args)
    {
        args.push(part);
    }
    args
}

fn split_extra_args(line: &str) -> Vec<String> {
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
    if !current.is_empty() {
        result.push(current);
    }
    result
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
    if matches!(flag, "-m" | "-md" | "--mmproj") {
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
        assert_eq!(split_extra_args("-c 1"), vec!["-c", "1"]);
    }

    #[test]
    fn keeps_quoted_extra_arg_values_together() {
        assert_eq!(
            split_extra_args("--alias \"local model\""),
            vec!["--alias", "local model"]
        );
    }

    #[test]
    fn previews_split_extra_args_without_wrapping_pair_as_single_arg() {
        let preview = command_preview(
            &[
                "llama-server.exe".to_string(),
                "-m".to_string(),
                "model.gguf".to_string(),
                "-c".to_string(),
                "1".to_string(),
            ],
            Path::new("C:\\llama"),
        );
        assert!(preview.contains("  -c 1"));
        assert!(!preview.contains("\"-c 1\""));
    }

    #[test]
    fn quotes_model_paths_after_model_flags() {
        let preview = command_preview(
            &[
                "llama-server.exe".to_string(),
                "-m".to_string(),
                "model.gguf".to_string(),
                "-md".to_string(),
                "draft.gguf".to_string(),
                "--mmproj".to_string(),
                "mmproj.gguf".to_string(),
            ],
            Path::new("C:\\llama"),
        );
        assert!(preview.contains("  -m \"model.gguf\""));
        assert!(preview.contains("  -md \"draft.gguf\""));
        assert!(preview.contains("  --mmproj \"mmproj.gguf\""));
    }
}
