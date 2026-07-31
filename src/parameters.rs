use crate::config::{FLASH_ATTN_MODES, LOAD_MODES, SPEC_TYPES};

pub const CACHE_TYPES: &[&str] = &[
    "f32", "f16", "bf16", "q8_0", "q4_0", "q4_1", "iq4_nl", "q5_0", "q5_1",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    Text,
    Choice(&'static [&'static str]),
    PositiveToggle,
    TwoSidedToggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamId {
    Host,
    Port,
    Alias,
    Ngl,
    NCpuMoe,
    Threads,
    BatchSize,
    UbatchSize,
    Parallel,
    CtxSize,
    CacheTypeK,
    CacheTypeV,
    LoadMode,
    FlashAttn,
    SpecType,
    SpecDraftNMax,
    SpecDraftNMin,
    ImageMinTokens,
    ImageMaxTokens,
    Jinja,
    Timeout,
    SplitMode,
    TensorSplit,
    MainGpu,
    Device,
    SpecDraftPMin,
    SpecDraftPSplit,
    WebUi,
    LogTimestamps,
    Offline,
    Verbose,
    KvOffload,
    KvUnified,
    SwaFull,
    CpuMoe,
    ReasoningPreserve,
}

#[derive(Debug, Clone, Copy)]
pub struct ParameterDefinition {
    pub id: ParamId,
    pub key: &'static str,
    pub label: &'static str,
    pub flag: &'static str,
    pub negative_flag: Option<&'static str>,
    pub control: ControlKind,
}

macro_rules! parameter {
    ($id:ident, $key:literal, $label:literal, $flag:literal, $control:expr) => {
        ParameterDefinition {
            id: ParamId::$id,
            key: $key,
            label: $label,
            flag: $flag,
            negative_flag: None,
            control: $control,
        }
    };
    ($id:ident, $key:literal, $label:literal, $flag:literal, $negative:literal, $control:expr) => {
        ParameterDefinition {
            id: ParamId::$id,
            key: $key,
            label: $label,
            flag: $flag,
            negative_flag: Some($negative),
            control: $control,
        }
    };
}

pub const PARAMETERS: &[ParameterDefinition] = &[
    parameter!(Host, "host", "监听地址", "--host", ControlKind::Text),
    parameter!(Port, "port", "端口", "--port", ControlKind::Text),
    parameter!(Alias, "alias", "模型别名", "--alias", ControlKind::Text),
    parameter!(Ngl, "ngl", "GPU 层数", "--gpu-layers", ControlKind::Text),
    parameter!(
        NCpuMoe,
        "n_cpu_moe",
        "CPU MoE 层数",
        "--n-cpu-moe",
        ControlKind::Text
    ),
    parameter!(Threads, "threads", "线程数", "--threads", ControlKind::Text),
    parameter!(
        BatchSize,
        "batch_size",
        "批大小",
        "--batch-size",
        ControlKind::Text
    ),
    parameter!(
        UbatchSize,
        "ubatch_size",
        "微批大小",
        "--ubatch-size",
        ControlKind::Text
    ),
    parameter!(
        Parallel,
        "parallel",
        "并发槽位",
        "--parallel",
        ControlKind::Text
    ),
    parameter!(
        CtxSize,
        "ctx_size",
        "上下文长度",
        "--ctx-size",
        ControlKind::Text
    ),
    parameter!(
        CacheTypeK,
        "cache_type_k",
        "K 缓存类型",
        "--cache-type-k",
        ControlKind::Choice(CACHE_TYPES)
    ),
    parameter!(
        CacheTypeV,
        "cache_type_v",
        "V 缓存类型",
        "--cache-type-v",
        ControlKind::Choice(CACHE_TYPES)
    ),
    parameter!(
        LoadMode,
        "load_mode",
        "模型加载模式",
        "--load-mode",
        ControlKind::Choice(LOAD_MODES)
    ),
    parameter!(
        FlashAttn,
        "flash_attn",
        "Flash Attention",
        "--flash-attn",
        ControlKind::Choice(FLASH_ATTN_MODES)
    ),
    parameter!(
        SpecType,
        "spec_type",
        "推测类型",
        "--spec-type",
        ControlKind::Choice(SPEC_TYPES)
    ),
    parameter!(
        SpecDraftNMax,
        "spec_draft_n_max",
        "草稿最大 N",
        "--spec-draft-n-max",
        ControlKind::Text
    ),
    parameter!(
        SpecDraftNMin,
        "spec_draft_n_min",
        "草稿最小 N",
        "--spec-draft-n-min",
        ControlKind::Text
    ),
    parameter!(
        ImageMinTokens,
        "image_min_tokens",
        "图像最小 token",
        "--image-min-tokens",
        ControlKind::Text
    ),
    parameter!(
        ImageMaxTokens,
        "image_max_tokens",
        "图像最大 token",
        "--image-max-tokens",
        ControlKind::Text
    ),
    parameter!(
        Jinja,
        "jinja",
        "启用 Jinja 模板",
        "--jinja",
        ControlKind::PositiveToggle
    ),
    parameter!(
        Timeout,
        "timeout",
        "超时秒数",
        "--timeout",
        ControlKind::Text
    ),
    parameter!(
        SplitMode,
        "split_mode",
        "切分模式",
        "--split-mode",
        ControlKind::Text
    ),
    parameter!(
        TensorSplit,
        "tensor_split",
        "张量切分",
        "--tensor-split",
        ControlKind::Text
    ),
    parameter!(
        MainGpu,
        "main_gpu",
        "主 GPU",
        "--main-gpu",
        ControlKind::Text
    ),
    parameter!(Device, "device", "设备", "--device", ControlKind::Text),
    parameter!(
        SpecDraftPMin,
        "spec_draft_p_min",
        "草稿最小概率",
        "--spec-draft-p-min",
        ControlKind::Text
    ),
    parameter!(
        SpecDraftPSplit,
        "spec_draft_p_split",
        "草稿切分概率",
        "--spec-draft-p-split",
        ControlKind::Text
    ),
    parameter!(
        WebUi,
        "web_ui",
        "启用 llama.cpp Web UI",
        "--ui",
        "--no-ui",
        ControlKind::TwoSidedToggle
    ),
    parameter!(
        LogTimestamps,
        "log_timestamps",
        "日志时间戳",
        "--log-timestamps",
        "--no-log-timestamps",
        ControlKind::TwoSidedToggle
    ),
    parameter!(
        Offline,
        "offline",
        "离线模式",
        "--offline",
        ControlKind::PositiveToggle
    ),
    parameter!(
        Verbose,
        "verbose",
        "详细日志",
        "--verbose",
        ControlKind::PositiveToggle
    ),
    parameter!(
        KvOffload,
        "kv_offload",
        "KV 缓存 offload",
        "--kv-offload",
        "--no-kv-offload",
        ControlKind::TwoSidedToggle
    ),
    parameter!(
        KvUnified,
        "kv_unified",
        "统一 KV buffer",
        "--kv-unified",
        ControlKind::PositiveToggle
    ),
    parameter!(
        SwaFull,
        "swa_full",
        "SWA 全尺寸缓存",
        "--swa-full",
        ControlKind::PositiveToggle
    ),
    parameter!(
        CpuMoe,
        "cpu_moe",
        "全部 MoE 放 CPU",
        "--cpu-moe",
        ControlKind::PositiveToggle
    ),
    parameter!(
        ReasoningPreserve,
        "reasoning_preserve",
        "保留思维链",
        "--reasoning-preserve",
        "--no-reasoning-preserve",
        ControlKind::TwoSidedToggle
    ),
];

pub fn parameter_by_key(key: &str) -> Option<&'static ParameterDefinition> {
    PARAMETERS.iter().find(|definition| definition.key == key)
}

pub fn parameter(id: ParamId) -> &'static ParameterDefinition {
    PARAMETERS
        .iter()
        .find(|definition| definition.id == id)
        .expect("all parameter ids have definitions")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DEFAULT_COMMON_PARAM_IDS, DEFAULT_OTHER_PARAM_IDS};
    use std::collections::BTreeSet;

    #[test]
    fn registry_matches_default_layout_exactly() {
        let registry: BTreeSet<&str> = PARAMETERS.iter().map(|item| item.key).collect();
        let layout: BTreeSet<&str> = DEFAULT_COMMON_PARAM_IDS
            .iter()
            .chain(DEFAULT_OTHER_PARAM_IDS.iter())
            .copied()
            .collect();

        assert_eq!(registry.len(), PARAMETERS.len());
        assert_eq!(registry, layout);
    }

    #[test]
    fn only_two_sided_toggles_have_negative_flags() {
        for item in PARAMETERS {
            assert_eq!(
                item.negative_flag.is_some(),
                matches!(item.control, ControlKind::TwoSidedToggle),
                "{}",
                item.key
            );
        }
    }
}
