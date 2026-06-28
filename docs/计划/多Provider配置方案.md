# 多 Provider API 配置方案

> 参考：CCSwitch（farion1231/cc-switch）、cc-provider-switcher、claude-code-switch（maxgfr）
>
> 设计日期：2026-06-28

---

## 一、问题

cc-gui 当前 API 配置写死了 DeepSeek 的假设：

| 硬编码项 | 当前值 | 问题 |
|----------|--------|------|
| Auth env var | 只读写 `ANTHROPIC_AUTH_TOKEN` | Anthropic 原生用 `ANTHROPIC_API_KEY` |
| Base URL 默认值 | `https://api.deepseek.com` | Anthropic 不需要填，OpenRouter 是另一个 |
| 模型预设 | `deepseek-v4-pro[1M]` 等 3 个 | 不同 provider 模型完全不同 |
| 连接测试 | 调 DeepSeek `/v1/chat/completions` | 非 DeepSeek 用户测试失败 |

换一台电脑、换个 provider，设置面板就废了。

## 二、CCSwitch 设计参考

CCSwitch 的核心思路：

1. **Provider 预设** — 每个 provider 是一个配置模板，包含 env vars + 默认模型 + base URL
2. **Provider 切换** — 选中某个 provider 时，自动写入对应的 `~/.claude/settings.json` env 块
3. **Provider 不互斥** — 用户可以添加多个 provider，切换时热替换 env 值
4. **直接模式 vs 代理模式** — 直接模式用 provider 原生 Anthropic 兼容端点；代理模式走本地网关做模型映射

cc-gui 只需要"直接模式"，不需要代理——CC CLI 本身就能调 Anthropic 兼容端点。

## 三、Provider 配置模型

### 3.1 Provider 定义

```typescript
interface ProviderPreset {
  id: string;                    // 唯一标识，如 "anthropic"、"deepseek"、"openrouter"
  name: string;                  // 显示名
  /** 写入 settings.json env 的键值对 */
  envTemplate: {
    ANTHROPIC_AUTH_TOKEN?: "";  // 占位，值来自用户填的 API Key
    ANTHROPIC_API_KEY?: "";
    ANTHROPIC_BASE_URL: string;
    ANTHROPIC_MODEL?: string;
    ANTHROPIC_DEFAULT_OPUS_MODEL?: string;
    ANTHROPIC_DEFAULT_SONNET_MODEL?: string;
    ANTHROPIC_DEFAULT_HAIKU_MODEL?: string;
    CLAUDE_CODE_SUBAGENT_MODEL?: string;
  };
  /** 连接测试端点（可选，默认不测试） */
  testEndpoint?: string;         // 如 "https://api.deepseek.com/v1/chat/completions"
  /** 支持的模型列表 */
  models: string[];
}
```

### 3.2 内置 Provider 预设

#### Anthropic 官方

```json
{
  "id": "anthropic",
  "name": "Anthropic",
  "envTemplate": {
    "ANTHROPIC_API_KEY": "",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-8",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-6",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "claude-haiku-4-5-20251001",
    "CLAUDE_CODE_SUBAGENT_MODEL": "claude-haiku-4-5-20251001"
  },
  "testEndpoint": "https://api.anthropic.com/v1/messages",
  "models": ["claude-opus-4-8", "claude-sonnet-4-6", "claude-haiku-4-5-20251001", "claude-fable-5"]
}
```

#### DeepSeek

```json
{
  "id": "deepseek",
  "name": "DeepSeek",
  "envTemplate": {
    "ANTHROPIC_AUTH_TOKEN": "",
    "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
    "ANTHROPIC_MODEL": "deepseek-v4-pro[1M]",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "deepseek-v4-pro[1M]",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "deepseek-v4-pro[1M]",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "deepseek-v4-flash",
    "CLAUDE_CODE_SUBAGENT_MODEL": "deepseek-v4-flash"
  },
  "testEndpoint": "https://api.deepseek.com/v1/chat/completions",
  "models": ["deepseek-v4-pro[1M]", "deepseek-v4-flash", "deepseek-v4"]
}
```

#### OpenRouter

```json
{
  "id": "openrouter",
  "name": "OpenRouter",
  "envTemplate": {
    "ANTHROPIC_AUTH_TOKEN": "",
    "ANTHROPIC_BASE_URL": "https://openrouter.ai/api/anthropic",
    "ANTHROPIC_MODEL": "anthropic/claude-sonnet-4-6",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "anthropic/claude-opus-4-8",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "anthropic/claude-sonnet-4-6",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "anthropic/claude-haiku-4-5-20251001",
    "CLAUDE_CODE_SUBAGENT_MODEL": "anthropic/claude-haiku-4-5-20251001",
    "OPENROUTER_REFERRER": "cc-gui"
  },
  "models": ["anthropic/claude-sonnet-4-6", "anthropic/claude-opus-4-8", "anthropic/claude-haiku-4-5-20251001"]
}
```

#### 自定义

```json
{
  "id": "custom",
  "name": "自定义",
  "envTemplate": {
    "ANTHROPIC_BASE_URL": "",
    "ANTHROPIC_AUTH_TOKEN": "",
    "ANTHROPIC_MODEL": ""
  },
  "models": []
}
```

## 四、数据流变更

### 4.1 读取（Rust → 前端）

```
settings.json env 块
  → get_claude_settings()
    → 遍历内置 provider 预设，匹配当前 env 值
      → 命中 → 返回 providerId + 已填充值
      → 未命中 → 返回 "custom" + 从 env 中提取的值
```

### 4.2 写入（前端 → Rust）

```
用户选 provider
  → 从预设取出 envTemplate
  → 用用户填的 apiKey 替换 "" 占位
  → set_claude_settings(env_kvs)
    → 合并到 settings.json env 块（替换而非覆盖）
```

### 4.3 关键规则

- **写 env 时只替换预设定义的 key**，不清除用户自定义的其他 env var（如 `HTTPS_PROXY`）
- **读 env 时 provider 识别是启发式的**——看 env 块中是否有 `ANTHROPIC_API_KEY`（Anthropic）、`ANTHROPIC_BASE_URL` 含 `deepseek`（DeepSeek）等
- **自定义 provider 保留所有 env 值不变**，用户手动编辑

## 五、前端 UI 变更

### 5.1 设置面板布局

当前三列（CC 配置 / CC 管理 / cc-gui 设置）中，"CC 配置"改为：

```
┌─ CC 配置 ──────────────────────────┐
│ Provider: [Anthropic ▾]           │  ← 新增：provider 下拉
│                                    │
│ API Key: [sk-…]                   │  ← 标签根据 provider 变化
│                                    │     Anthropic → "API Key"
│                                    │     DeepSeek → "API Key"
│                                    │     OpenRouter → "API Key"
│                                    │
│ Model: [claude-sonnet-4-6 ▾]     │  ← 模型列表根据 provider 变化
│                                    │
│ [测试连接]                         │  ← 端点根据 provider 变化
└────────────────────────────────────┘
```

### 5.2 Provider 下拉

```html
<div class="settings-dropdown">
  <!-- 选中项显示 provider 图标 + 名称 -->
  <span>🔵 Anthropic</span>  <!-- 或 🟢 DeepSeek / 🟣 OpenRouter / ⚙️ 自定义 -->
  
  <!-- 下拉选项 -->
  <div>
    <button>🔵 Anthropic</button>
    <button>🟢 DeepSeek</button>
    <button>🟣 OpenRouter</button>
    <button>⚙️ 自定义</button>
  </div>
</div>
```

### 5.3 切换 Provider 时

1. 保存当前 provider 的配置（写入 settings.json）
2. 清空 API Key / Model 输入框
3. 加载新 provider 的预设值
4. 模型下拉更新为新 provider 的模型列表

## 六、Rust 端变更

### 6.1 ClaudeSettings 结构

```rust
struct ClaudeSettings {
    provider_id: String,      // 新增：识别的 provider
    api_key: String,
    base_url: String,
    model: String,
    // 新增：subagent 模型
    subagent_model: String,
    effort: String,
    permission_mode: String,
}
```

### 6.2 内置预设

```rust
// 用 const 定义，编译期内嵌
const PROVIDERS: &[ProviderPreset] = &[
    ProviderPreset { id: "anthropic", name: "Anthropic", ... },
    ProviderPreset { id: "deepseek", name: "DeepSeek", ... },
    ProviderPreset { id: "openrouter", name: "OpenRouter", ... },
];
```

### 6.3 连接测试

当前 `connect_llm` 硬编码调 DeepSeek `/v1/chat/completions`。改为根据 provider 的 `testEndpoint` 调对应端点。

Anthropic 原生调 `/v1/messages`，DeepSeek/OpenRouter 调 `/v1/chat/completions`。

## 七、兼容性

### 7.1 升级路径

老用户 `settings.json` 里有：

```json
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "sk-xxx",
    "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic"
  }
}
```

首次打开新版本时：
1. 检测到 `ANTHROPIC_BASE_URL` 含 `deepseek` → 自动识别为 DeepSeek provider
2. API Key / Model 值自动填充当前值
3. 用户体验无中断

### 7.2 settings.json 写入策略

- **不清除未知 key**：只写入新 provider 定义的 key + 用户填的值，保留现有的 `HTTPS_PROXY`、`ENABLE_TOOL_SEARCH` 等
- **不覆盖未改值**：用户切换 provider 但还没填 API Key 时，不写入空值
- **自定义模式保留所有 env**：识别为自定义时，读写不限制具体 key

## 八、实施步骤

| 步 | 内容 | 文件 |
|----|------|------|
| 1 | Rust 定义 `ProviderPreset` 结构 + 3 个内置预设 | `src-tauri/src/provider.rs`（新文件） |
| 2 | 改 `get_claude_settings` 返回 provider_id | `src-tauri/src/lib.rs` |
| 3 | 改 `set_claude_settings` 接受 provider_id + env map | `src-tauri/src/lib.rs` |
| 4 | 改 `connect_llm` 根据 provider 选测试端点 | `src-tauri/src/lib.rs` |
| 5 | 前端 `settings.ts` store 加 providerId 字段 | `src/stores/settings.ts` |
| 6 | 前端 `SettingsPanel.vue` 加 provider 下拉 + 动态模型列表 | `src/components/settings/SettingsPanel.vue` |
| 7 | 前端 `tauri-bridge.ts` 更新接口 | `src/lib/tauri-bridge.ts` |
| 8 | i18n provider 名称 | `src/locales/` |

## 九、不做的事

- ❌ 本地代理/网关（CCSwitch 的代理模式）—— CC CLI 直接调 Anthropic 兼容端点即可
- ❌ 模型映射/别名—— CC CLI 自身处理
- ❌ 多 provider 同时启用—— 一次只用一个
- ❌ API Key 加密存储—— 当前 settings.json 明文，后续版本考虑
