/// Provider preset definitions for Claude Code API configuration.
/// Each provider knows which env vars to write to ~/.claude/settings.json
/// and how to test connectivity.

#[derive(Debug, Clone)]
pub struct ProviderPreset {
    pub id: &'static str,
    pub name: &'static str,
    /// env var → default value ("" means placeholder filled by user's API key)
    pub env_template: &'static [(&'static str, &'static str)],
    /// Connection test endpoint (None = no test available)
    pub test_endpoint: Option<&'static str>,
    /// Whether the test endpoint uses Anthropic Messages API format (x-api-key header)
    /// vs OpenAI Chat Completions format (Authorization: Bearer header)
    pub test_uses_anthropic_format: bool,
    /// Supported model names for the dropdown
    pub models: &'static [&'static str],
}

/// All built-in provider presets (all have native Anthropic-compatible endpoints, no proxy needed).
pub const PROVIDERS: &[ProviderPreset] = &[
    // ── Anthropic Official ──
    ProviderPreset {
        id: "anthropic",
        name: "Anthropic",
        env_template: &[
            ("ANTHROPIC_API_KEY", ""),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL", "claude-opus-4-8"),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL", "claude-sonnet-4-6"),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL", "claude-haiku-4-5-20251001"),
            ("CLAUDE_CODE_SUBAGENT_MODEL", "claude-haiku-4-5-20251001"),
        ],
        test_endpoint: Some("https://api.anthropic.com/v1/messages"),
        test_uses_anthropic_format: true,
        models: &["claude-opus-4-8", "claude-sonnet-4-6", "claude-haiku-4-5-20251001", "claude-fable-5"],
    },
    // ── DeepSeek ──
    ProviderPreset {
        id: "deepseek",
        name: "DeepSeek",
        env_template: &[
            ("ANTHROPIC_AUTH_TOKEN", ""),
            ("ANTHROPIC_API_KEY", ""),           // 显式清空，防止残留 Anthropic key 被 CC 优先使用
            ("ANTHROPIC_BASE_URL", "https://api.deepseek.com/anthropic"),
            ("ANTHROPIC_MODEL", "deepseek-v4-pro[1M]"),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL", "deepseek-v4-pro[1M]"),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL_NAME", "deepseek-v4-pro"),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL", "deepseek-v4-pro[1M]"),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL_NAME", "deepseek-v4-pro"),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL", "deepseek-v4-flash"),
            ("CLAUDE_CODE_SUBAGENT_MODEL", "deepseek-v4-flash"),
        ],
        test_endpoint: Some("https://api.deepseek.com/v1/chat/completions"),
        test_uses_anthropic_format: false,
        models: &["deepseek-v4-pro[1M]", "deepseek-v4-flash", "deepseek-v4"],
    },
    // ── OpenRouter ──
    ProviderPreset {
        id: "openrouter",
        name: "OpenRouter",
        env_template: &[
            ("ANTHROPIC_AUTH_TOKEN", ""),
            ("ANTHROPIC_API_KEY", ""),           // 必须显式清空
            ("ANTHROPIC_BASE_URL", "https://openrouter.ai/api"),
            ("ANTHROPIC_MODEL", "anthropic/claude-sonnet-4-6"),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL", "anthropic/claude-opus-4-8"),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL", "anthropic/claude-sonnet-4-6"),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL", "anthropic/claude-haiku-4-5-20251001"),
            ("CLAUDE_CODE_SUBAGENT_MODEL", "anthropic/claude-haiku-4-5-20251001"),
        ],
        test_endpoint: Some("https://openrouter.ai/api/v1/chat/completions"),
        test_uses_anthropic_format: false,
        models: &["anthropic/claude-sonnet-4-6", "anthropic/claude-opus-4-8", "anthropic/claude-haiku-4-5-20251001"],
    },
    // ── 硅基流动（SiliconFlow）──
    ProviderPreset {
        id: "siliconflow",
        name: "硅基流动",
        env_template: &[
            ("ANTHROPIC_AUTH_TOKEN", ""),
            ("ANTHROPIC_API_KEY", ""),           // 显式清空
            ("ANTHROPIC_BASE_URL", "https://api.siliconflow.cn/"),
            ("ANTHROPIC_MODEL", "deepseek-ai/DeepSeek-V3"),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL", "deepseek-ai/DeepSeek-V3"),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL", "deepseek-ai/DeepSeek-V3"),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL", "deepseek-ai/DeepSeek-V3"),
            ("CLAUDE_CODE_SUBAGENT_MODEL", "deepseek-ai/DeepSeek-V3"),
        ],
        test_endpoint: Some("https://api.siliconflow.cn/v1/chat/completions"),
        test_uses_anthropic_format: false,
        models: &["deepseek-ai/DeepSeek-V3", "deepseek-ai/DeepSeek-R1", "Pro/zai-org/GLM-5", "Qwen/Qwen3-235B-A22B"],
    },
    // ── 智谱 GLM ──
    ProviderPreset {
        id: "zhipu",
        name: "智谱 GLM",
        env_template: &[
            ("ANTHROPIC_AUTH_TOKEN", ""),
            ("ANTHROPIC_API_KEY", ""),           // 显式清空
            ("ANTHROPIC_BASE_URL", "https://open.bigmodel.cn/api/anthropic"),
            ("ANTHROPIC_MODEL", "glm-5"),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL", "glm-5"),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL", "glm-5"),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL", "glm-5"),
            ("CLAUDE_CODE_SUBAGENT_MODEL", "glm-5"),
        ],
        test_endpoint: Some("https://open.bigmodel.cn/api/paas/v4/chat/completions"),
        test_uses_anthropic_format: false,
        models: &["glm-5", "glm-5.1", "glm-4.7"],
    },
    // ── Kimi ──
    ProviderPreset {
        id: "kimi",
        name: "Kimi",
        env_template: &[
            ("ANTHROPIC_AUTH_TOKEN", ""),
            ("ANTHROPIC_API_KEY", ""),           // 显式清空
            ("ANTHROPIC_BASE_URL", "https://api.moonshot.cn/anthropic"),
            ("ANTHROPIC_MODEL", "kimi-k2.5"),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL", "kimi-k2.5"),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL", "kimi-k2.5"),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL", "kimi-k2.5"),
            ("CLAUDE_CODE_SUBAGENT_MODEL", "kimi-k2.5"),
            ("ENABLE_TOOL_SEARCH", "false"),
        ],
        test_endpoint: Some("https://api.moonshot.cn/v1/chat/completions"),
        test_uses_anthropic_format: false,
        models: &["kimi-k2.5", "kimi-k2.6"],
    },
    // ── MiniMax ──
    ProviderPreset {
        id: "minimax",
        name: "MiniMax",
        env_template: &[
            ("ANTHROPIC_AUTH_TOKEN", ""),
            ("ANTHROPIC_API_KEY", ""),           // 显式清空
            ("ANTHROPIC_BASE_URL", "https://api.minimaxi.com/anthropic"),
            ("ANTHROPIC_MODEL", "minimax-m2.7"),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL", "minimax-m2.7"),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL", "minimax-m2.7"),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL", "minimax-m2.7"),
            ("CLAUDE_CODE_SUBAGENT_MODEL", "minimax-m2.7"),
        ],
        test_endpoint: Some("https://api.minimaxi.com/v1/chat/completions"),
        test_uses_anthropic_format: false,
        models: &["minimax-m2.7"],
    },
];

/// Find a provider by id. Returns None for "custom" or unknown ids.
pub fn find_provider(id: &str) -> Option<&'static ProviderPreset> {
    PROVIDERS.iter().find(|p| p.id == id)
}

/// Detect which provider best matches the current settings.json env block.
/// Returns the provider id, or "custom" if no match.
pub fn detect_provider(env: &serde_json::Value) -> &'static str {
    // Heuristic: check distinctive env vars for each provider
    let base_url = env["ANTHROPIC_BASE_URL"].as_str().unwrap_or("");

    // Anthropic: ANTHROPIC_API_KEY present (even empty) AND no ANTHROPIC_AUTH_TOKEN
    // 兜底空 key 场景——切到 Anthropic 但还没填 key 时，ANTHROPIC_API_KEY 字段存在为 ""
    // 此时必须仍识别为 Anthropic，否则切回其他 provider 时 scrubbing 逻辑会跳过清理
    let has_api_key_field = env.get("ANTHROPIC_API_KEY").is_some();
    let has_auth_token = env.get("ANTHROPIC_AUTH_TOKEN")
        .and_then(|v| v.as_str())
        .map_or(false, |s| !s.is_empty());
    let base_url_ok = base_url.is_empty() || base_url.contains("api.anthropic.com");
    if has_api_key_field && !has_auth_token && base_url_ok {
        return "anthropic";
    }
    if base_url.contains("deepseek") { return "deepseek"; }
    if base_url.contains("openrouter.ai") { return "openrouter"; }
    if base_url.contains("siliconflow") { return "siliconflow"; }
    if base_url.contains("bigmodel.cn") { return "zhipu"; }
    if base_url.contains("moonshot") { return "kimi"; }
    if base_url.contains("minimaxi") { return "minimax"; }

    "custom"
}
