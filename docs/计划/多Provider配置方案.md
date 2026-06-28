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
// 注意：这是概念模型。Rust 端实际结构为 ProviderPreset（provider.rs），
// 前端无对等结构——provider 预设逻辑全在 Rust 端，前端只维护 PROVIDER_MODELS 镜像。
interface ProviderPreset {
  id: string;                    // 唯一标识
  name: string;                  // 显示名
  envTemplate: Record<string, string>;  // env var → 默认值（"" = 由用户填写）
  testEndpoint?: string;         // 连接测试端点 URL（实际未被代码消费，仅元数据）
  testUsesAnthropicFormat: boolean;  // true=Anthropic Messages API，false=OpenAI Chat Completions
  models: string[];              // 支持的模型列表
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
    // ANTHROPIC_BASE_URL 不设——切换时 Rust scrubbing 逻辑自动清除旧 provider 残留 key
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-8",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-6",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "claude-haiku-4-5-20251001",
    "CLAUDE_CODE_SUBAGENT_MODEL": "claude-haiku-4-5-20251001"
  },
  "testEndpoint": "https://api.anthropic.com/v1/messages",
  "testUsesAnthropicFormat": true,
  // 实际健康检查用 GET /v1/models（零 token 消耗），不走 testEndpoint。testEndpoint 是元数据未消费。
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
    "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME": "deepseek-v4-pro",
    "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME": "deepseek-v4-pro",
    "CLAUDE_CODE_SUBAGENT_MODEL": "deepseek-v4-flash"
  },
  "testEndpoint": "https://api.deepseek.com/v1/chat/completions",
  "testUsesAnthropicFormat": false,
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
    "ANTHROPIC_API_KEY": "",
    "ANTHROPIC_BASE_URL": "https://openrouter.ai/api",
    "ANTHROPIC_MODEL": "anthropic/claude-sonnet-4-6",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "anthropic/claude-opus-4-8",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "anthropic/claude-sonnet-4-6",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "anthropic/claude-haiku-4-5-20251001",
    "CLAUDE_CODE_SUBAGENT_MODEL": "anthropic/claude-haiku-4-5-20251001"
  },
  "testEndpoint": "https://openrouter.ai/api/v1/chat/completions",
  "testUsesAnthropicFormat": false,
  "models": ["anthropic/claude-sonnet-4-6", "anthropic/claude-opus-4-8", "anthropic/claude-haiku-4-5-20251001"]
}
```

> **注意**：`ANTHROPIC_API_KEY` 必须显式设为空字符串 `""`（不能省略），否则 CC CLI 会优先用 `ANTHROPIC_API_KEY` 调 Anthropic 原生 API。Base URL 是 `https://openrouter.ai/api`（**不带** `/anthropic` 后缀）。不需要 `OPENROUTER_REFERRER` env var（官方文档未提及此变量）。

#### 硅基流动（SiliconFlow）

多 provider 网关，托管 DeepSeek、GLM、Qwen、Kimi 等模型。

```json
{
  "id": "siliconflow",
  "name": "硅基流动",
  "envTemplate": {
    "ANTHROPIC_AUTH_TOKEN": "",
    "ANTHROPIC_BASE_URL": "https://api.siliconflow.cn/",
    "ANTHROPIC_MODEL": "deepseek-ai/DeepSeek-V3",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "deepseek-ai/DeepSeek-V3",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "deepseek-ai/DeepSeek-V3",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "deepseek-ai/DeepSeek-V3",
    "CLAUDE_CODE_SUBAGENT_MODEL": "deepseek-ai/DeepSeek-V3"
  },
  "testEndpoint": "https://api.siliconflow.cn/v1/chat/completions",
  "testUsesAnthropicFormat": false,
  "models": ["deepseek-ai/DeepSeek-V3", "deepseek-ai/DeepSeek-R1", "Pro/zai-org/GLM-5", "Qwen/Qwen3-235B-A22B"]
}
```

> ⚠️ 注意：不支持 thinking 模式变体。

#### 智谱 GLM

```json
{
  "id": "zhipu",
  "name": "智谱 GLM",
  "envTemplate": {
    "ANTHROPIC_AUTH_TOKEN": "",
    "ANTHROPIC_BASE_URL": "https://open.bigmodel.cn/api/anthropic",
    "ANTHROPIC_MODEL": "glm-5",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "glm-5",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "glm-5",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "glm-5",
    "CLAUDE_CODE_SUBAGENT_MODEL": "glm-5"
  },
  "testEndpoint": "https://open.bigmodel.cn/api/paas/v4/chat/completions",
  "testUsesAnthropicFormat": false,
  "models": ["glm-5", "glm-5.1", "glm-4.7"]
}
```

#### Kimi（月之暗面）

```json
{
  "id": "kimi",
  "name": "Kimi",
  "envTemplate": {
    "ANTHROPIC_AUTH_TOKEN": "",
    "ANTHROPIC_BASE_URL": "https://api.moonshot.cn/anthropic",
    "ANTHROPIC_MODEL": "kimi-k2.5",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "kimi-k2.5",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "kimi-k2.5",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "kimi-k2.5",
    "CLAUDE_CODE_SUBAGENT_MODEL": "kimi-k2.5",
    "ENABLE_TOOL_SEARCH": "false"
  },
  "testEndpoint": "https://api.moonshot.cn/v1/chat/completions",
  "testUsesAnthropicFormat": false,
  "models": ["kimi-k2.5", "kimi-k2.6"]
}
```

> ⚠️ 注意：需设置 `ENABLE_TOOL_SEARCH: false` 避免循环中消耗过多 token。Kimi 官方另推荐 `https://api.kimi.com/coding/` 端点（Kimi Code 订阅用户），`api.moonshot.cn/anthropic` 为按量付费端点。另外 CC v2.1.69+ 的 ToolSearch 功能在非 Anthropic 原生端点上可能导致 400 错误，**所有第三方 provider（2-7）都可能需要 `ENABLE_TOOL_SEARCH=false`**。

#### MiniMax

```json
{
  "id": "minimax",
  "name": "MiniMax",
  "envTemplate": {
    "ANTHROPIC_AUTH_TOKEN": "",
    "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/anthropic",
    "ANTHROPIC_MODEL": "minimax-m2.7",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "minimax-m2.7",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "minimax-m2.7",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "minimax-m2.7",
    "CLAUDE_CODE_SUBAGENT_MODEL": "minimax-m2.7"
  },
  "testEndpoint": "https://api.minimaxi.com/v1/chat/completions",
  "testUsesAnthropicFormat": false,
  "models": ["minimax-m2.7"]
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

- **切换 provider 时，先清除旧 provider 独有的 key，再写入新 provider 的 key**。
  例如 DeepSeek → Anthropic：必须清除 `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_MODEL`，
  否则 CLI 读到残留的 `ANTHROPIC_BASE_URL=https://api.deepseek.com` 会把 Anthropic 请求发到 DeepSeek。
  清除规则：新 provider 的 `envTemplate` 中不存在的 key，且该 key 在旧 provider 中存在 → 从 settings.json 删除。
- **用户自定义的 key 永远不清除**（如 `HTTPS_PROXY` 等不在任何 provider 预设中的 key）。注意 `ENABLE_TOOL_SEARCH` 是 Kimi 预设的一部分（managed key），切换离开 Kimi 时会被清除。
- **读 env 时 provider 识别是启发式的**——看 env 块中是否有 `ANTHROPIC_API_KEY`（Anthropic）、`ANTHROPIC_BASE_URL` 含 `deepseek`（DeepSeek）等。**启发式可能误判，允许用户手动选择 provider 覆盖自动识别**
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
  <span>🟢 DeepSeek</span>
  <div>
    <button>🔵 Anthropic 官方</button>
    <button>🟢 DeepSeek</button>
    <button>🟣 OpenRouter</button>
    <button>🔷 硅基流动</button>
    <button>🔶 智谱 GLM</button>
    <button>🟠 Kimi</button>
    <button>🟡 MiniMax</button>
    <button>⚙️ 自定义</button>
  </div>
</div>
```

### 5.3 切换 Provider 时

1. 保存当前 provider 的配置（写入 localStorage，见第十章）
2. 从 localStorage 恢复目标 provider 的 apiKey / baseUrl / model（无记录则用默认值）
3. 模型下拉更新为新 provider 的模型列表
4. 校验恢复的 model 在目标列表中，不在则 fallback 第一个

## 六、Rust 端变更

### 6.1 ClaudeSettings 结构（实际实现）

```rust
struct ClaudeSettings {
    provider_id: String,       // 新增：识别的 provider
    api_key: String,
    base_url: String,
    model: String,
    models: Vec<String>,       // 新增：该 provider 的模型列表
    effort: String,
    permission_mode: String,
}
```

### 6.2 内置预设（7 个 + custom fallback）

```rust
const PROVIDERS: &[ProviderPreset] = &[
    ProviderPreset { id: "anthropic", name: "Anthropic", ... },
    ProviderPreset { id: "deepseek", name: "DeepSeek", ... },
    ProviderPreset { id: "openrouter", name: "OpenRouter", ... },
    ProviderPreset { id: "siliconflow", name: "硅基流动", ... },
    ProviderPreset { id: "zhipu", name: "智谱 GLM", ... },
    ProviderPreset { id: "kimi", name: "Kimi", ... },
    ProviderPreset { id: "minimax", name: "MiniMax", ... },
];
// "custom" 不是预设——find_provider 返回 None，set_claude_settings 走 custom 分支
```

### 6.3 连接测试

`connect_llm` 根据 `test_uses_anthropic_format` 分流：
- `true`（Anthropic）→ `GET /v1/models` + `x-api-key` header（零 token 消耗）
- `false`（DeepSeek/OpenRouter 等）→ `POST {base}/v1/chat/completions` + `Authorization: Bearer` header

> 注意：Rust 端 `test_endpoint` 字段是元数据，实际未被 `connect_llm` 消费——端点 URL 硬编码在 `session.rs` 的 `test_connection` / `test_connection_anthropic` 中。

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

### 8.1 已完成（P0 — Provider 切换）

| 步 | 内容 | 文件 | 状态 |
|----|------|------|------|
| 1 | Rust 定义 `ProviderPreset` 结构 + 7 个内置预设 | `src-tauri/src/provider.rs`（新文件） | ✅ |
| 2 | 改 `get_claude_settings` 返回 provider_id + models | `src-tauri/src/lib.rs` | ✅ |
| 3 | 改 `set_claude_settings` 接受 provider_id + 清理逻辑 | `src-tauri/src/lib.rs` | ✅ |
| 4 | 改 `connect_llm`：Anthropic Messages API vs OpenAI Chat Completions 双路径 | `src-tauri/src/lib.rs` | ✅ |
| 5 | 前端 `settings.ts` store 加 providerId / models 字段 | `src/stores/settings.ts` | ✅ |
| 6 | 前端 `SettingsPanel.vue` 加 provider 下拉 + 动态模型列表 | `src/components/settings/SettingsPanel.vue` | ✅ |
| 7 | 前端 `tauri-bridge.ts` 更新接口 | `src/lib/tauri-bridge.ts` | ✅ |
| 8 | i18n provider 名称 + 品牌 logo | `src/locales/` | ✅ |

### 8.2 待实施（P1 — 配置持久化）

| 步 | 内容 | 文件 | 状态 |
|----|------|------|------|
| 9 | Rust 新增 `save_provider_config` / `load_provider_configs` 两个 command | `src-tauri/src/lib.rs` | ✅ |
| 10 | 注册新命令到 `generate_handler![]` | `src-tauri/src/lib.rs` | ✅ |
| 11 | 前端 `tauri-bridge.ts` 加 `saveProviderConfig` / `loadProviderConfigs` | `src/lib/tauri-bridge.ts` | ✅ |
| 12 | 前端 `settings.ts` 加 `PROVIDER_BASE_URLS`、`providerConfigs`、`saveCurrentConfig`、`restoreConfig`、自动持久化 watch | `src/stores/settings.ts` | ✅ |
| 13 | 前端 `SettingsPanel.vue` 改 `switchProvider` 流程 | `src/components/settings/SettingsPanel.vue` | ✅ |
| 14 | 补单元测试 + Mock | `src/stores/settings.test.ts`, `tauri-mock.ts` | ✅ |

## 九、联网验证结果

| 项 | 验证结论 | 来源 |
|----|---------|------|
| Anthropic 原生需要 `ANTHROPIC_BASE_URL` 吗？ | **不需要**。默认 `https://api.anthropic.com`，不设即可。仅当路由到第三方 endpoint 时才设。设为 `""` 可清除旧值 | [GitHub Issue #216](https://github.com/anthropics/claude-code/issues/216) |
| OpenRouter 的 Base URL 是什么？ | `https://openrouter.ai/api`（**不是** `/anthropic` 后缀！）| [OpenRouter 官方指南](https://openrouter.ai/blog/tutorials/claude-code-openrouter/) |
| OpenRouter 需要 `OPENROUTER_REFERRER` 吗？ | **不需要**。此 env var 不存在于 CC CLI 或 OpenRouter 官方文档 | 同上 |
| OpenRouter 的特殊要求？ | **`ANTHROPIC_API_KEY` 必须显式设为 `""`**，否则 CC CLI 会优先用它调 Anthropic 原生 | 同上 |
| DeepSeek `[1M]` 后缀？ | **实测通过**。CC CLI 内部剥离后再发请求 | cc-gui 实测 |
| 连接测试 Anthropic 格式兼容吗？ | **不兼容**。当前 `connect_llm` 用 OpenAI 格式（`/v1/chat/completions` + `Authorization: Bearer`），Anthropic 用 Messages API（`x-api-key` + `anthropic-version` header）。需拆分为**两个独立测试函数**或标记 Anthropic 为"测试不可用" | [Anthropic API 文档](https://platform.claude.com/docs/en/build-with-claude/working-with-messages) |
| Anthropic 健康检查端点？ | `GET /v1/models` + `x-api-key` header，零 token 消耗 | [GitHub PR #263](https://github.com/dhyansraj/mcp-mesh/pull/263) |

## 十、配置持久化（跨切换保留 API Key）

> 2026-06-28 新增。步骤 1-8 已完成 Provider 切换功能，但**切换时旧 provider 的 key 会被 Rust 端清除**，导致切回来后 key 丢失。

### 10.1 问题

`set_claude_settings` 在切换 provider 时会删除旧 provider 的专属 env var（如 Anthropic → DeepSeek 时删 `ANTHROPIC_API_KEY`、写 `ANTHROPIC_AUTH_TOKEN`）。用户切回 Anthropic 时，`settings.json` 里已经没有 `ANTHROPIC_API_KEY` 了，得重新填。

### 10.2 方案：SQLite 持久化

利用已有 `settings` 表（key-value，`db.rs`），每个 provider 存一行：

```
key:   "provider_config:deepseek"
value: {"apiKey":"sk-xxx","baseUrl":"https://api.deepseek.com","model":"deepseek-v4-pro[1M]"}
```

新增 2 个 Rust 命令，前端通过 bridge 调用。~30 行 Rust + ~55 行 TS，4 个文件。

**数据模型**：

```typescript
interface ProviderConfig {
  apiKey: string;
  baseUrl: string;
  model: string;
}
// 存在 SQLite settings 表，key = "provider_config:{providerId}"
```

**核心逻辑**：

- **切换时**：`saveCurrentConfig()` → `restoreConfig(id)` → 切 providerId。Vue 3 同步批处理
- **编辑时**：`watch([apiKey, baseUrl, model])` 自动写 SQLite
- **启动时**：`loadProviderConfigs()` 加载全部 → `getClaudeSettings()` 返回后合并当前值

### 10.3 修改文件

#### Rust 端：`src-tauri/src/lib.rs`（+25 行）

两个新 Tauri command：

```rust
#[tauri::command]
fn save_provider_config(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    api_key: String,
    base_url: String,
    model: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let key = format!("provider_config:{}", provider_id);
    let value = serde_json::json!({ "apiKey": api_key, "baseUrl": base_url, "model": model }).to_string();
    db.conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn load_provider_configs(
    state: tauri::State<'_, AppState>,
) -> Result<HashMap<String, ProviderConfig>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.conn.prepare(
        "SELECT key, value FROM settings WHERE key LIKE 'provider_config:%'"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let value: String = row.get(1)?;
        Ok((key, value))
    }).map_err(|e| e.to_string())?;
    let mut configs = HashMap::new();
    for row in rows {
        let (key, value) = row.map_err(|e| e.to_string())?;
        let id = key.strip_prefix("provider_config:").unwrap_or(&key).to_string();
        if let Ok(cfg) = serde_json::from_str::<ProviderConfig>(&value) {
            configs.insert(id, cfg);
        }
    }
    Ok(configs)
}
```

另外在 `lib.rs` 顶部新增 `ProviderConfig` 结构体（serde 反序列化用）：

```rust
#[derive(Deserialize)]
struct ProviderConfig {
    #[serde(rename = "apiKey")]
    api_key: String,
    #[serde(rename = "baseUrl")]
    base_url: String,
    model: String,
}
```

并在 `main.rs` 的 `generate_handler![]` 宏中注册两个新命令。

#### 前端 `src/lib/tauri-bridge.ts`（+12 行）

```typescript
export async function saveProviderConfig(
  providerId: string, apiKey: string, baseUrl: string, model: string
): Promise<void> {
  await invoke("save_provider_config", { providerId, apiKey, baseUrl, model });
}

export async function loadProviderConfigs(): Promise<Record<string, ProviderConfig>> {
  return invoke("load_provider_configs");
}
```

#### 前端 `src/stores/settings.ts`（+35 行）

- `PROVIDER_BASE_URLS` 常量（8 provider 默认 URL，用户面值；注意 DeepSeek 在 UI 显示 `https://api.deepseek.com` 但 Rust env_template 实际写入 `https://api.deepseek.com/anthropic`，这是已有行为）
- `providerConfigs: ref<Record<string, ProviderConfig>>({})` — 启动时从 `loadProviderConfigs()` 加载
- `saveCurrentConfig()` — 调用 `saveProviderConfig(providerId, apiKey, baseUrl, model)`
- `restoreConfig(id)` — 从 `providerConfigs` 查找；无记录则用 `PROVIDER_BASE_URLS` 默认值
- `watch([apiKey, baseUrl, model])` — 编辑时自动 `saveCurrentConfig()`
- 启动流程：`loadProviderConfigs()` → `getClaudeSettings()` → 合并当前配置到 `providerConfigs`

#### 前端 `src/components/settings/SettingsPanel.vue`（改 5 行）

`switchProvider(id)` 改为：

1. `settings.saveCurrentConfig()` — 存旧 provider（此时 providerId 还是旧值）
2. `settings.restoreConfig(id)` — 恢复 apiKey/baseUrl/model（先恢复值，同步从内存 map 读）
3. 设 `providerId = id`、更新 `models`（最后切 providerId，watcher 拿到完整正确值）
4. 校验恢复的 model 在目标模型列表中，不在则 fallback 第一个

> ⚠️ **关键**：必须先 restoreConfig 再改 providerId。否则 watcher 会在中间态触发 `setClaudeSettings(旧apiKey, 旧baseUrl, 旧model, providerId=新)`，把旧 provider 的 key 错误写入新 provider 的 env var。

### 10.4 开关流程

```
用户选 Anthropic → switchProvider("anthropic")
  → saveCurrentConfig()           // 旧 deepseek → SQLite（providerId 仍是 deepseek）
  → restoreConfig("anthropic")    // 从内存 map 恢复 apiKey/baseUrl/model
  → providerId = "anthropic"      // 最后切 providerId，watcher 拿到完整正确值
  → model 校验（不在列表 → 第一个）
  → watcher 触发 → setClaudeSettings(正确的 Anthropic apiKey/baseUrl/model)
  → Rust 写 env template 到 settings.json
```

### 10.5 升级路径（未来：CCSwitch 式完整管理 UI）

1. SQLite 独立表 `provider_configs`（id / label / provider_id / api_key / base_url / model / is_active）替代 settings 表 key-value
2. Rust 端 list/save/delete/activate 4 个 commands
3. Vue 端 ProviderConfigCards 组件（卡片网格、新增/编辑/删除/激活）
4. 数据迁移：`settings` 表 `provider_config:*` 行 → 新表

## 十一、不做的事

- ❌ 本地代理/网关（CCSwitch 的代理模式）—— CC CLI 直接调 Anthropic 兼容端点即可
- ❌ 模型映射/别名—— CC CLI 自身处理
- ❌ 多 provider 同时启用—— 一次只用一个
- ❌ API Key 加密存储—— 当前 settings.json 明文，后续版本考虑
