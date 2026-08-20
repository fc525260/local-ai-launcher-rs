use crate::config::{FLASH_ATTN_MODES, LOAD_MODES, SPEC_TYPES};

pub const CACHE_TYPES: &[&str] = &[
    "f32", "f16", "bf16", "q8_0", "q4_0", "q4_1", "iq4_nl", "q5_0", "q5_1",
];

pub const ROPE_SCALINGS: &[&str] = &["none", "linear", "yarn"];
pub const NUMA_TYPES: &[&str] = &["distribute", "isolate", "numactl"];
pub const FIT_MODES: &[&str] = &["on", "off"];
pub const MIROSTAT_MODES: &[&str] = &["0", "1", "2"];

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
    ThreadsBatch,
    Predict,
    Keep,
    BatchSize,
    UbatchSize,
    Parallel,
    CtxSize,
    CacheTypeK,
    CacheTypeV,
    LoadMode,
    FlashAttn,
    RopeScaling,
    RopeScale,
    RopeFreqBase,
    RopeFreqScale,
    YarnOrigCtx,
    Rpc,
    Numa,
    Fit,
    CheckTensors,
    LogFile,
    SpecType,
    SpecDraftNMax,
    SpecDraftNMin,
    SpecDraftThreads,
    SpecDraftNgl,
    SpecDraftDevice,
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
    Seed,
    Temperature,
    TopK,
    TopP,
    MinP,
    RepeatLastN,
    RepeatPenalty,
    PresencePenalty,
    FrequencyPenalty,
    Mirostat,
    MirostatLr,
    MirostatEnt,
    GrammarFile,
    JsonSchema,
    SystemPrompt,
    ReversePrompt,
    ContextShift,
    ShowTimings,
}

#[derive(Debug, Clone, Copy)]
pub struct ParameterDefinition {
    pub id: ParamId,
    pub key: &'static str,
    pub label: &'static str,
    pub flag: &'static str,
    pub short_flag: Option<&'static str>,
    pub negative_flag: Option<&'static str>,
    pub control: ControlKind,
}

impl ParameterDefinition {
    pub fn display_flag(&self) -> String {
        match self.short_flag {
            Some(short) => format!("{} ({})", self.flag, short),
            None => self.flag.to_string(),
        }
    }
}

macro_rules! parameter {
    ($id:ident, $key:literal, $label:literal, $flag:literal, $negative:literal, $short:literal, $control:expr) => {
        ParameterDefinition {
            id: ParamId::$id,
            key: $key,
            label: $label,
            flag: $flag,
            short_flag: Some($short),
            negative_flag: Some($negative),
            control: $control,
        }
    };
    ($id:ident, $key:literal, $label:literal, $flag:literal, $negative:literal, None, $control:expr) => {
        ParameterDefinition {
            id: ParamId::$id,
            key: $key,
            label: $label,
            flag: $flag,
            short_flag: None,
            negative_flag: Some($negative),
            control: $control,
        }
    };
    ($id:ident, $key:literal, $label:literal, $flag:literal, $control:expr) => {
        ParameterDefinition {
            id: ParamId::$id,
            key: $key,
            label: $label,
            flag: $flag,
            short_flag: None,
            negative_flag: None,
            control: $control,
        }
    };
    ($id:ident, $key:literal, $label:literal, $flag:literal, $short:literal, $control:expr) => {
        ParameterDefinition {
            id: ParamId::$id,
            key: $key,
            label: $label,
            flag: $flag,
            short_flag: Some($short),
            negative_flag: None,
            control: $control,
        }
    };
}

pub const PARAMETERS: &[ParameterDefinition] = &[
    parameter!(Host, "host", "监听地址", "--host", ControlKind::Text),
    parameter!(Port, "port", "端口", "--port", ControlKind::Text),
    parameter!(Alias, "alias", "模型别名", "--alias", ControlKind::Text),
    parameter!(
        Ngl,
        "ngl",
        "GPU 层数",
        "--gpu-layers",
        "-ngl",
        ControlKind::Text
    ),
    parameter!(
        NCpuMoe,
        "n_cpu_moe",
        "CPU MoE 层数",
        "--n-cpu-moe",
        "-ncmoe",
        ControlKind::Text
    ),
    parameter!(
        Threads,
        "threads",
        "线程数",
        "--threads",
        "-t",
        ControlKind::Text
    ),
    parameter!(
        ThreadsBatch,
        "threads_batch",
        "批处理线程数",
        "--threads-batch",
        "-tb",
        ControlKind::Text
    ),
    parameter!(
        Predict,
        "predict",
        "预测 token 数",
        "--predict",
        "-n",
        ControlKind::Text
    ),
    parameter!(Keep, "keep", "保留前缀 token", "--keep", ControlKind::Text),
    parameter!(
        BatchSize,
        "batch_size",
        "批大小",
        "--batch-size",
        "-b",
        ControlKind::Text
    ),
    parameter!(
        UbatchSize,
        "ubatch_size",
        "微批大小",
        "--ubatch-size",
        "-ub",
        ControlKind::Text
    ),
    parameter!(
        Parallel,
        "parallel",
        "并发槽位",
        "--parallel",
        "-np",
        ControlKind::Text
    ),
    parameter!(
        CtxSize,
        "ctx_size",
        "上下文长度",
        "--ctx-size",
        "-c",
        ControlKind::Text
    ),
    parameter!(
        CacheTypeK,
        "cache_type_k",
        "K 缓存类型",
        "--cache-type-k",
        "-ctk",
        ControlKind::Choice(CACHE_TYPES)
    ),
    parameter!(
        CacheTypeV,
        "cache_type_v",
        "V 缓存类型",
        "--cache-type-v",
        "-ctv",
        ControlKind::Choice(CACHE_TYPES)
    ),
    parameter!(
        LoadMode,
        "load_mode",
        "模型加载模式",
        "--load-mode",
        "-lm",
        ControlKind::Choice(LOAD_MODES)
    ),
    parameter!(
        FlashAttn,
        "flash_attn",
        "Flash Attention",
        "--flash-attn",
        "-fa",
        ControlKind::Choice(FLASH_ATTN_MODES)
    ),
    parameter!(
        RopeScaling,
        "rope_scaling",
        "RoPE 缩放方式",
        "--rope-scaling",
        ControlKind::Choice(ROPE_SCALINGS)
    ),
    parameter!(
        RopeScale,
        "rope_scale",
        "RoPE 上下文缩放",
        "--rope-scale",
        ControlKind::Text
    ),
    parameter!(
        RopeFreqBase,
        "rope_freq_base",
        "RoPE 基频",
        "--rope-freq-base",
        ControlKind::Text
    ),
    parameter!(
        RopeFreqScale,
        "rope_freq_scale",
        "RoPE 频率缩放",
        "--rope-freq-scale",
        ControlKind::Text
    ),
    parameter!(
        YarnOrigCtx,
        "yarn_orig_ctx",
        "YaRN 原始上下文",
        "--yarn-orig-ctx",
        ControlKind::Text
    ),
    parameter!(Rpc, "rpc", "RPC 服务器", "--rpc", ControlKind::Text),
    parameter!(
        Numa,
        "numa",
        "NUMA 优化",
        "--numa",
        ControlKind::Choice(NUMA_TYPES)
    ),
    parameter!(
        Fit,
        "fit",
        "显存自适应",
        "--fit",
        ControlKind::Choice(FIT_MODES)
    ),
    parameter!(
        CheckTensors,
        "check_tensors",
        "检查张量数据",
        "--check-tensors",
        ControlKind::PositiveToggle
    ),
    parameter!(
        LogFile,
        "log_file",
        "日志文件",
        "--log-file",
        ControlKind::Text
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
        SpecDraftThreads,
        "spec_draft_threads",
        "草稿线程数",
        "--spec-draft-threads",
        "-td",
        ControlKind::Text
    ),
    parameter!(
        SpecDraftNgl,
        "spec_draft_ngl",
        "草稿 GPU 层数",
        "--spec-draft-ngl",
        "-ngld",
        ControlKind::Text
    ),
    parameter!(
        SpecDraftDevice,
        "spec_draft_device",
        "草稿设备",
        "--spec-draft-device",
        "-devd",
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
        "-sm",
        ControlKind::Text
    ),
    parameter!(
        TensorSplit,
        "tensor_split",
        "张量切分",
        "--tensor-split",
        "-ts",
        ControlKind::Text
    ),
    parameter!(
        MainGpu,
        "main_gpu",
        "主 GPU",
        "--main-gpu",
        "-mg",
        ControlKind::Text
    ),
    parameter!(
        Device,
        "device",
        "设备",
        "--device",
        "-dev",
        ControlKind::Text
    ),
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
        "启用 WebUI",
        "--ui",
        "--no-ui",
        None,
        ControlKind::TwoSidedToggle
    ),
    parameter!(
        LogTimestamps,
        "log_timestamps",
        "日志时间戳",
        "--log-timestamps",
        "--no-log-timestamps",
        None,
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
        "-v",
        ControlKind::PositiveToggle
    ),
    parameter!(
        KvOffload,
        "kv_offload",
        "KV 缓存 offload",
        "--kv-offload",
        "--no-kv-offload",
        "-kvo",
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
        "-cmoe",
        ControlKind::PositiveToggle
    ),
    parameter!(
        ReasoningPreserve,
        "reasoning_preserve",
        "保留思维链",
        "--reasoning-preserve",
        "--no-reasoning-preserve",
        None,
        ControlKind::TwoSidedToggle
    ),
    parameter!(Seed, "seed", "随机种子", "--seed", "-s", ControlKind::Text),
    parameter!(
        Temperature,
        "temperature",
        "温度",
        "--temperature",
        "-temp",
        ControlKind::Text
    ),
    parameter!(TopK, "top_k", "Top-k", "--top-k", ControlKind::Text),
    parameter!(TopP, "top_p", "Top-p", "--top-p", ControlKind::Text),
    parameter!(MinP, "min_p", "Min-p", "--min-p", ControlKind::Text),
    parameter!(
        RepeatLastN,
        "repeat_last_n",
        "重复惩罚范围",
        "--repeat-last-n",
        ControlKind::Text
    ),
    parameter!(
        RepeatPenalty,
        "repeat_penalty",
        "重复惩罚系数",
        "--repeat-penalty",
        ControlKind::Text
    ),
    parameter!(
        PresencePenalty,
        "presence_penalty",
        "存在惩罚",
        "--presence-penalty",
        ControlKind::Text
    ),
    parameter!(
        FrequencyPenalty,
        "frequency_penalty",
        "频率惩罚",
        "--frequency-penalty",
        ControlKind::Text
    ),
    parameter!(
        Mirostat,
        "mirostat",
        "Mirostat",
        "--mirostat",
        ControlKind::Choice(MIROSTAT_MODES)
    ),
    parameter!(
        MirostatLr,
        "mirostat_lr",
        "Mirostat 学习率",
        "--mirostat-lr",
        ControlKind::Text
    ),
    parameter!(
        MirostatEnt,
        "mirostat_ent",
        "Mirostat 目标熵",
        "--mirostat-ent",
        ControlKind::Text
    ),
    parameter!(
        GrammarFile,
        "grammar_file",
        "语法文件",
        "--grammar-file",
        ControlKind::Text
    ),
    parameter!(
        JsonSchema,
        "json_schema",
        "JSON Schema",
        "--json-schema",
        "-j",
        ControlKind::Text
    ),
    parameter!(
        SystemPrompt,
        "system_prompt",
        "系统提示词",
        "--system-prompt",
        "-sys",
        ControlKind::Text
    ),
    parameter!(
        ReversePrompt,
        "reverse_prompt",
        "停止提示词",
        "--reverse-prompt",
        "-r",
        ControlKind::Text
    ),
    parameter!(
        ContextShift,
        "context_shift",
        "上下文移位",
        "--context-shift",
        "--no-context-shift",
        None,
        ControlKind::TwoSidedToggle
    ),
    parameter!(
        ShowTimings,
        "show_timings",
        "显示计时信息",
        "--show-timings",
        "--no-show-timings",
        None,
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

pub enum ReferenceKind {
    Value,
    Toggle,
    Choice(&'static [&'static str]),
}

pub struct ParamReferenceEntry {
    pub flag: &'static str,
    pub display: &'static str,
    pub description: &'static str,
    pub kind: ReferenceKind,
}

pub struct ParamReferenceGroup {
    pub title: &'static str,
    pub entries: &'static [ParamReferenceEntry],
}

const fn reference(
    flag: &'static str,
    display: &'static str,
    description: &'static str,
    kind: ReferenceKind,
) -> ParamReferenceEntry {
    ParamReferenceEntry {
        flag,
        display,
        description,
        kind,
    }
}

pub const EXTRA_PARAM_REFERENCE: &[ParamReferenceGroup] = &[
    ParamReferenceGroup {
        title: "通用参数",
        entries: &[
            reference(
                "-C, --cpu-mask",
                "--cpu-mask",
                "CPU 亲和掩码（十六进制）",
                ReferenceKind::Value,
            ),
            reference(
                "-Cr, --cpu-range",
                "--cpu-range",
                "CPU 核心范围，如 0-3",
                ReferenceKind::Value,
            ),
            reference(
                "--cpu-strict <0|1>",
                "--cpu-strict",
                "严格绑定 CPU",
                ReferenceKind::Choice(&["0", "1"]),
            ),
            reference(
                "--prio N",
                "--prio",
                "优先级：-1 低，0 正常，1 中，2 高，3 实时",
                ReferenceKind::Value,
            ),
            reference(
                "--poll <0...100>",
                "--poll",
                "等待工作时的轮询级别",
                ReferenceKind::Value,
            ),
            reference(
                "-Cb, --cpu-mask-batch",
                "--cpu-mask-batch",
                "批处理 CPU 亲和掩码",
                ReferenceKind::Value,
            ),
            reference(
                "-Crb, --cpu-range-batch",
                "--cpu-range-batch",
                "批处理 CPU 范围",
                ReferenceKind::Value,
            ),
            reference(
                "--cpu-strict-batch <0|1>",
                "--cpu-strict-batch",
                "批处理严格绑定 CPU",
                ReferenceKind::Choice(&["0", "1"]),
            ),
            reference(
                "--prio-batch N",
                "--prio-batch",
                "批处理优先级",
                ReferenceKind::Value,
            ),
            reference(
                "--poll-batch <0|1>",
                "--poll-batch",
                "批处理轮询",
                ReferenceKind::Choice(&["0", "1"]),
            ),
            reference(
                "--perf, --no-perf",
                "--perf",
                "启用内部性能计时",
                ReferenceKind::Toggle,
            ),
            reference(
                "-e, --escape, --no-escape",
                "--escape",
                "处理转义序列（如 \\n）",
                ReferenceKind::Toggle,
            ),
            reference(
                "--yarn-ext-factor N",
                "--yarn-ext-factor",
                "YaRN 外推混合因子（-1.0 自动）",
                ReferenceKind::Value,
            ),
            reference(
                "--yarn-attn-factor N",
                "--yarn-attn-factor",
                "YaRN 注意力缩放因子（-1.0 自动）",
                ReferenceKind::Value,
            ),
            reference(
                "--yarn-beta-slow N",
                "--yarn-beta-slow",
                "YaRN 高修正维度（-1.0 自动）",
                ReferenceKind::Value,
            ),
            reference(
                "--yarn-beta-fast N",
                "--yarn-beta-fast",
                "YaRN 低修正维度（-1.0 自动）",
                ReferenceKind::Value,
            ),
            reference(
                "--repack, --no-repack",
                "--repack",
                "权重重打包",
                ReferenceKind::Toggle,
            ),
            reference(
                "--no-host",
                "--no-host",
                "绕过主机缓冲区",
                ReferenceKind::Toggle,
            ),
            reference(
                "--list-devices",
                "--list-devices",
                "列出可用设备并退出",
                ReferenceKind::Toggle,
            ),
            reference(
                "-ot, --override-tensor",
                "--override-tensor",
                "覆盖张量缓冲区类型，如 weight=f16",
                ReferenceKind::Value,
            ),
            reference(
                "-fitt, --fit-target",
                "--fit-target",
                "每设备预留显存 MiB，默认 1024",
                ReferenceKind::Value,
            ),
            reference(
                "-fitc, --fit-ctx",
                "--fit-ctx",
                "fit 最小上下文，默认 4096",
                ReferenceKind::Value,
            ),
            reference(
                "--override-kv",
                "--override-kv",
                "覆盖模型元数据，如 tokenizer.ggml.add_bos_token=bool:false",
                ReferenceKind::Value,
            ),
            reference(
                "--op-offload, --no-op-offload",
                "--op-offload",
                "主机张量操作卸载到设备",
                ReferenceKind::Toggle,
            ),
            reference(
                "--lora FNAME",
                "--lora",
                "加载 LoRA 适配器（可多个，逗号分隔）",
                ReferenceKind::Value,
            ),
            reference(
                "--lora-scaled",
                "--lora-scaled",
                "带缩放因子的 LoRA 适配器",
                ReferenceKind::Value,
            ),
            reference(
                "--control-vector FNAME",
                "--control-vector",
                "添加控制向量（可多个）",
                ReferenceKind::Value,
            ),
            reference(
                "--control-vector-scaled",
                "--control-vector-scaled",
                "带缩放因子的控制向量",
                ReferenceKind::Value,
            ),
            reference(
                "--control-vector-layer-range",
                "--control-vector-layer-range",
                "控制向量作用的层范围",
                ReferenceKind::Value,
            ),
            reference(
                "-mu, --model-url",
                "--model-url",
                "模型下载 URL",
                ReferenceKind::Value,
            ),
            reference(
                "-dr, --docker-repo",
                "--docker-repo",
                "Docker Hub 模型仓库",
                ReferenceKind::Value,
            ),
            reference(
                "-hf, --hf-repo",
                "--hf-repo",
                "Hugging Face 仓库自动下载 GGUF",
                ReferenceKind::Value,
            ),
            reference(
                "-hff, --hf-file",
                "--hf-file",
                "HF 仓库中的具体文件名",
                ReferenceKind::Value,
            ),
            reference(
                "-hft, --hf-token",
                "--hf-token",
                "HF 访问令牌（默认读 HF_TOKEN）",
                ReferenceKind::Value,
            ),
            reference(
                "--log-disable",
                "--log-disable",
                "禁用日志",
                ReferenceKind::Toggle,
            ),
            reference(
                "--log-colors [on|off|auto]",
                "--log-colors",
                "彩色日志",
                ReferenceKind::Choice(&["on", "off", "auto"]),
            ),
            reference(
                "-lv, --verbosity N",
                "--verbosity",
                "日志详细程度：0 通用，1 错误，2 警告，3 信息，4 跟踪，5 调试",
                ReferenceKind::Value,
            ),
            reference(
                "--log-prefix, --no-log-prefix",
                "--log-prefix",
                "日志前缀",
                ReferenceKind::Toggle,
            ),
            reference(
                "-ctkd, --cache-type-k-draft",
                "--cache-type-k-draft",
                "草稿模型 K 缓存类型",
                ReferenceKind::Choice(CACHE_TYPES),
            ),
            reference(
                "-ctvd, --cache-type-v-draft",
                "--cache-type-v-draft",
                "草稿模型 V 缓存类型",
                ReferenceKind::Choice(CACHE_TYPES),
            ),
        ],
    },
    ParamReferenceGroup {
        title: "采样参数",
        entries: &[
            reference(
                "--samplers",
                "--samplers",
                "按顺序使用的采样器，分号分隔",
                ReferenceKind::Value,
            ),
            reference(
                "--sampler-seq",
                "--sampler-seq",
                "简化采样器序列（单字符），默认 edskypmxt",
                ReferenceKind::Value,
            ),
            reference(
                "--ignore-eos",
                "--ignore-eos",
                "忽略 EOS 继续生成",
                ReferenceKind::Toggle,
            ),
            reference(
                "--top-nsigma N",
                "--top-nsigma",
                "Top-n-sigma 采样，默认 -1.0 禁用",
                ReferenceKind::Value,
            ),
            reference(
                "--xtc-probability N",
                "--xtc-probability",
                "XTC 概率，默认 0.0 禁用",
                ReferenceKind::Value,
            ),
            reference(
                "--xtc-threshold N",
                "--xtc-threshold",
                "XTC 阈值，默认 0.10",
                ReferenceKind::Value,
            ),
            reference(
                "--typical-p N",
                "--typical-p",
                "局部典型采样 p，默认 1.00 禁用",
                ReferenceKind::Value,
            ),
            reference(
                "--dry-multiplier N",
                "--dry-multiplier",
                "DRY 采样乘数，默认 0.00 禁用",
                ReferenceKind::Value,
            ),
            reference(
                "--dry-base N",
                "--dry-base",
                "DRY 基值，默认 1.75",
                ReferenceKind::Value,
            ),
            reference(
                "--dry-allowed-length N",
                "--dry-allowed-length",
                "DRY 允许长度，默认 2",
                ReferenceKind::Value,
            ),
            reference(
                "--dry-penalty-last-n N",
                "--dry-penalty-last-n",
                "DRY 惩罚范围，默认 64",
                ReferenceKind::Value,
            ),
            reference(
                "--dry-sequence-breaker",
                "--dry-sequence-breaker",
                "DRY 序列分隔符，用 none 清除",
                ReferenceKind::Value,
            ),
            reference(
                "--adaptive-target N",
                "--adaptive-target",
                "自适应 p 目标概率 0~1，负值禁用",
                ReferenceKind::Value,
            ),
            reference(
                "--adaptive-decay N",
                "--adaptive-decay",
                "自适应 p 衰减率 0~0.99",
                ReferenceKind::Value,
            ),
            reference(
                "--dynatemp-range N",
                "--dynatemp-range",
                "动态温度范围，默认 0.00 禁用",
                ReferenceKind::Value,
            ),
            reference(
                "--dynatemp-exp N",
                "--dynatemp-exp",
                "动态温度指数，默认 1.00",
                ReferenceKind::Value,
            ),
            reference(
                "-l, --logit-bias",
                "--logit-bias",
                "token logit 偏置，如 15043+1 提高、15043-1 降低",
                ReferenceKind::Value,
            ),
            reference(
                "--grammar GRAMMAR",
                "--grammar",
                "BNF 语法约束生成",
                ReferenceKind::Value,
            ),
            reference(
                "-jf, --json-schema-file",
                "--json-schema-file",
                "从文件读取 JSON Schema",
                ReferenceKind::Value,
            ),
            reference(
                "-bs, --backend-sampling",
                "--backend-sampling",
                "启用后端采样（实验性）",
                ReferenceKind::Toggle,
            ),
        ],
    },
    ParamReferenceGroup {
        title: "推测解码参数",
        entries: &[
            reference(
                "-hfd, --hf-repo-draft",
                "--hf-repo-draft",
                "草稿模型 Hugging Face 仓库",
                ReferenceKind::Value,
            ),
            reference(
                "-tbd, --threads-batch-draft",
                "--threads-batch-draft",
                "草稿模型批处理线程数",
                ReferenceKind::Value,
            ),
            reference(
                "-Cd, --cpu-mask-draft",
                "--cpu-mask-draft",
                "草稿模型 CPU 亲和掩码",
                ReferenceKind::Value,
            ),
            reference(
                "-Crd, --cpu-range-draft",
                "--cpu-range-draft",
                "草稿模型 CPU 范围",
                ReferenceKind::Value,
            ),
            reference(
                "--cpu-strict-draft <0|1>",
                "--cpu-strict-draft",
                "草稿模型严格绑定 CPU",
                ReferenceKind::Choice(&["0", "1"]),
            ),
            reference(
                "--prio-draft N",
                "--prio-draft",
                "草稿模型优先级",
                ReferenceKind::Value,
            ),
            reference(
                "--poll-draft <0|1>",
                "--poll-draft",
                "草稿模型轮询",
                ReferenceKind::Choice(&["0", "1"]),
            ),
            reference(
                "-Cbd, --cpu-mask-batch-draft",
                "--cpu-mask-batch-draft",
                "草稿模型批处理 CPU 掩码",
                ReferenceKind::Value,
            ),
            reference(
                "--cpu-strict-batch-draft",
                "--cpu-strict-batch-draft",
                "草稿模型批处理严格绑定",
                ReferenceKind::Choice(&["0", "1"]),
            ),
            reference(
                "--prio-batch-draft N",
                "--prio-batch-draft",
                "草稿模型批处理优先级",
                ReferenceKind::Value,
            ),
            reference(
                "--poll-batch-draft <0|1>",
                "--poll-batch-draft",
                "草稿模型批处理轮询",
                ReferenceKind::Choice(&["0", "1"]),
            ),
            reference(
                "-otd, --override-tensor-draft",
                "--override-tensor-draft",
                "覆盖草稿模型张量缓冲类型",
                ReferenceKind::Value,
            ),
            reference(
                "-cmoed, --cpu-moe-draft",
                "--cpu-moe-draft",
                "草稿模型全部 MoE 权重放 CPU",
                ReferenceKind::Toggle,
            ),
            reference(
                "-ncmoed, --n-cpu-moe-draft",
                "--n-cpu-moe-draft",
                "草稿模型前 N 层 MoE 权重放 CPU",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-draft-backend-sampling",
                "--spec-draft-backend-sampling",
                "草稿采样卸载到后端",
                ReferenceKind::Toggle,
            ),
            reference(
                "--spec-ngram-mod-n-min",
                "--spec-ngram-mod-n-min",
                "ngram-mod 最小 token 数，默认 48",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-ngram-mod-n-max",
                "--spec-ngram-mod-n-max",
                "ngram-mod 最大 token 数，默认 64",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-ngram-mod-n-match",
                "--spec-ngram-mod-n-match",
                "ngram-mod 查找长度，默认 24",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-ngram-simple-size-n",
                "--spec-ngram-simple-size-n",
                "ngram-simple 的 n，默认 12",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-ngram-simple-size-m",
                "--spec-ngram-simple-size-m",
                "ngram-simple 的 m，默认 48",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-ngram-simple-min-hits",
                "--spec-ngram-simple-min-hits",
                "ngram-simple 最小命中，默认 1",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-ngram-map-k-size-n",
                "--spec-ngram-map-k-size-n",
                "map-k 的 n，默认 12",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-ngram-map-k-size-m",
                "--spec-ngram-map-k-size-m",
                "map-k 的 m，默认 48",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-ngram-map-k-min-hits",
                "--spec-ngram-map-k-min-hits",
                "map-k 最小命中，默认 1",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-ngram-map-k4v-size-n",
                "--spec-ngram-map-k4v-size-n",
                "map-k4v 的 n，默认 12",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-ngram-map-k4v-size-m",
                "--spec-ngram-map-k4v-size-m",
                "map-k4v 的 m，默认 48",
                ReferenceKind::Value,
            ),
            reference(
                "--spec-ngram-map-k4v-min-hits",
                "--spec-ngram-map-k4v-min-hits",
                "map-k4v 最小命中，默认 1",
                ReferenceKind::Value,
            ),
        ],
    },
    ParamReferenceGroup {
        title: "示例特定参数",
        entries: &[
            reference(
                "--server-base URL",
                "--server-base",
                "连接已有服务器而不是启动新服务",
                ReferenceKind::Value,
            ),
            reference(
                "--verbose-prompt",
                "--verbose-prompt",
                "生成前打印详细提示词",
                ReferenceKind::Toggle,
            ),
            reference(
                "--display-prompt, --no-display-prompt",
                "--display-prompt",
                "生成时显示提示词",
                ReferenceKind::Toggle,
            ),
            reference(
                "-co, --color [on|off|auto]",
                "--color",
                "彩色输出",
                ReferenceKind::Choice(&["on", "off", "auto"]),
            ),
            reference(
                "-ctxcp, --ctx-checkpoints N",
                "--ctx-checkpoints",
                "每槽位上下文检查点数，默认 32",
                ReferenceKind::Value,
            ),
            reference(
                "-cram, --cache-ram N",
                "--cache-ram",
                "最大缓存 MiB，默认 8192，-1 无限制，0 禁用",
                ReferenceKind::Value,
            ),
            reference(
                "-sysf, --system-prompt-file",
                "--system-prompt-file",
                "从文件读取系统提示词",
                ReferenceKind::Value,
            ),
            reference(
                "-sp, --special",
                "--special",
                "启用特殊 token 输出",
                ReferenceKind::Toggle,
            ),
            reference(
                "-cnv, --conversation, --no-conversation",
                "--conversation",
                "对话模式",
                ReferenceKind::Toggle,
            ),
            reference(
                "-st, --single-turn",
                "--single-turn",
                "仅单轮对话后退出",
                ReferenceKind::Toggle,
            ),
            reference(
                "-mli, --multiline-input",
                "--multiline-input",
                "允许多行输入",
                ReferenceKind::Toggle,
            ),
            reference(
                "--warmup, --no-warmup",
                "--warmup",
                "空跑预热",
                ReferenceKind::Toggle,
            ),
            reference(
                "-mmu, --mmproj-url",
                "--mmproj-url",
                "多模态投影仪下载 URL",
                ReferenceKind::Value,
            ),
            reference(
                "--mmproj-auto, --no-mmproj",
                "--mmproj-auto",
                "自动使用多模态投影仪",
                ReferenceKind::Toggle,
            ),
            reference(
                "--mmproj-offload, --no-mmproj-offload",
                "--mmproj-offload",
                "投影仪卸载到 GPU",
                ReferenceKind::Toggle,
            ),
            reference(
                "--image, --audio, --video",
                "--image",
                "输入媒体文件（可多个，逗号分隔）",
                ReferenceKind::Value,
            ),
            reference(
                "--audio FILE",
                "--audio",
                "输入音频文件",
                ReferenceKind::Value,
            ),
            reference(
                "--video FILE",
                "--video",
                "输入视频文件",
                ReferenceKind::Value,
            ),
            reference(
                "-o, --output, --output-file",
                "--output",
                "输出文件路径",
                ReferenceKind::Value,
            ),
            reference(
                "--chat-template-kwargs",
                "--chat-template-kwargs",
                "聊天模板额外 JSON 参数",
                ReferenceKind::Value,
            ),
            reference(
                "--chat-template",
                "--chat-template",
                "自定义聊天模板",
                ReferenceKind::Value,
            ),
            reference(
                "--chat-template-file",
                "--chat-template-file",
                "聊天模板文件",
                ReferenceKind::Value,
            ),
            reference(
                "--skip-chat-parsing",
                "--skip-chat-parsing",
                "强制使用纯内容解析器",
                ReferenceKind::Toggle,
            ),
            reference(
                "--simple-io",
                "--simple-io",
                "基本 IO，提高子进程兼容性",
                ReferenceKind::Toggle,
            ),
            reference(
                "--reasoning-format",
                "--reasoning-format",
                "思维标签解析格式",
                ReferenceKind::Choice(&["none", "deepseek", "deepseek-legacy"]),
            ),
            reference(
                "-rea, --reasoning [on|off|auto]",
                "--reasoning",
                "是否在聊天中使用推理",
                ReferenceKind::Choice(&["on", "off", "auto"]),
            ),
            reference(
                "--reasoning-effort",
                "--reasoning-effort",
                "推理努力级别",
                ReferenceKind::Choice(&[
                    "default", "minimal", "low", "medium", "high", "xhigh", "max",
                ]),
            ),
            reference(
                "--reasoning-budget N",
                "--reasoning-budget",
                "推理 token 预算：-1 无限制，0 立即结束",
                ReferenceKind::Value,
            ),
            reference(
                "--reasoning-budget-message",
                "--reasoning-budget-message",
                "预算耗尽时注入的消息",
                ReferenceKind::Value,
            ),
            reference(
                "--log-prompts-dir PATH",
                "--log-prompts-dir",
                "将提示词记录到目录（调试用）",
                ReferenceKind::Value,
            ),
            reference(
                "--gpt-oss-20b/120b-default",
                "--gpt-oss-20b-default",
                "一键使用 GPT-OSS 模型（可联网下载）",
                ReferenceKind::Toggle,
            ),
            reference(
                "--vision-gemma-4b/12b-default",
                "--vision-gemma-4b-default",
                "一键使用 Gemma 3 QAT 视觉模型",
                ReferenceKind::Toggle,
            ),
            reference(
                "--spec-default",
                "--spec-default",
                "启用默认推测解码配置",
                ReferenceKind::Toggle,
            ),
        ],
    },
];

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
