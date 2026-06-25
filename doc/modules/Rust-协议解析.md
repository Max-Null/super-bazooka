# Rust-协议解析

> Claude Code CLI stream-json 的 NDJSON 协议解析 — StreamEvent 枚举（system/assistant/user/result/control_request）、ContentBlock 枚举（text/thinking/tool_use）、StreamLine 解析器、StreamFrontendEvent 转前端事件。

## 功能说明

- NDJSON 行解析（`serde_json::from_str`）
- StreamEvent 枚举：system（含 subtype）、assistant（含 ContentBlock 数组）、user（含 tool_result）、result（含 usage/cost 统计）
- ContentBlock 解析：text（文本）、thinking（思考链）、tool_use（工具调用）、tool_result（工具结果）
- session_id 捕获：从 system/init 事件提取 claude session UUID
- MCP 服务器捕获：优先读取原生 `mcp_servers` 字段，fallback 解析 `mcp__<server>__<tool>` 前缀工具名
- StreamFrontendEvent 转换：处理 stream_event 增量（text_delta / input_json_delta）、assistant 完整事件、result 终态、control_request 审批
- 双 API 兼容：Anthropic API 发送 text_delta 增量 + assistant 完整事件（前端去重），DeepSeek API 仅发送 assistant 完整事件

## 事件解析流程

```mermaid
graph TD
    A[NDJSON 行] --> B{StreamLine::parse}
    B -->|成功| C[StreamLine { inner: Value }]
    B -->|失败| E[emit stream-error]
    C --> D{inner.type?}

    D -->|system/init| F[capture_session_id + capture_mcp_servers]
    F --> G[emit session-created]

    D -->|stream_event| H{event.type?}
    H -->|content_block_delta text_delta| I[text 增量追加]
    H -->|content_block_delta input_json_delta| J[tool_input 增量]
    H -->|content_block_start tool_use| K[tool_start 通知]
    I --> M[emit stream-event]
    J --> M
    K --> M

    D -->|assistant| L[提取 text/thinking/tool_use 块]
    L --> M

    D -->|result| N[提取 duration_ms/usage/cost]
    N --> M

    D -->|control_request| O[审批请求 subtype/tool_name/tool_input]
    O --> M
```

## 公开 API

| 类型 | 名称 | 说明 |
|------|------|------|
| enum | StreamEvent | CLI stream-json 事件枚举：System / Assistant / User / Result |
| struct | AssistantMessage | 助手消息：content(Vec\<ContentBlock\>) / model / stop_reason / stop_sequence / id |
| enum | ContentBlock | 内容块枚举：Text / Thinking / ToolUse |
| struct | UserMessage | 用户消息：content(Vec\<UserContentBlock\>) / parent_tool_use_id |
| enum | UserContentBlock | 用户内容块：ToolResult { tool_use_id / content / is_error } |
| struct | StreamLine | NDJSON 行解析器（inner: Value） |
| method | StreamLine::parse | 解析 NDJSON 行为 StreamLine |
| method | StreamLine::capture_session_id | 从 system/init 事件提取 session_id |
| method | StreamLine::capture_mcp_servers | 提取 MCP 服务器列表（原生字段优先 → mcp__ 前缀解析 fallback） |
| method | StreamLine::to_frontend_event | 转换为 StreamFrontendEvent（核心转换逻辑，处理所有事件类型） |
| struct | StreamFrontendEvent | 前端事件：event_type / session_id / text / thinking / tool_use / control_request / is_final / error / duration_ms / input_tokens / output_tokens / cost_usd |
| struct | ControlRequest | 控制请求：subtype / tool_name / tool_input |

## 配置属性

本模块无对外配置属性。

## 代码示例

### StreamLine 核心解析与转换

```rust
// protocol.rs
impl StreamLine {
    pub fn parse(line: &str) -> Option<Self> {
        serde_json::from_str::<Value>(line.trim()).ok().map(|inner| StreamLine { inner })
    }

    pub fn capture_session_id(&self) -> Option<String> {
        if self.inner["type"].as_str()? == "system"
            && self.inner["subtype"].as_str()? == "init"
        {
            self.inner["session_id"].as_str().map(|s| s.to_string())
        } else {
            None
        }
    }

    pub fn capture_mcp_servers(&self) -> Vec<String> {
        // 优先：从 mcp_servers 原生字段读取
        if let Some(servers) = self.inner["mcp_servers"].as_array() {
            return servers.iter()
                .filter_map(|s| s["name"].as_str().map(|n| n.to_string()))
                .collect();
        }
        // 兜底：从 mcp__<server>__<tool> 名称解析
        let tools = self.inner["tools"].as_array()?;
        let mut servers = BTreeSet::new();
        for tool in tools {
            if let Some(name) = tool["name"].as_str() {
                if let Some(rest) = name.strip_prefix("mcp__") {
                    if let Some(end) = rest.find("__") {
                        servers.insert(rest[..end].to_string());
                    }
                }
            }
        }
        servers.into_iter().collect()
    }
}
```

### StreamFrontendEvent 结构

```rust
// protocol.rs
pub struct StreamFrontendEvent {
    pub event_type: String,
    pub session_id: String,
    pub text: String,
    pub thinking: String,
    pub tool_use: Option<Vec<Value>>,
    pub control_request: Option<ControlRequest>,
    pub is_final: bool,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub cost_usd: Option<f64>,
}
```

## 依赖说明

### 内部依赖

本模块不依赖其他内部模块。

### 外部依赖（Cargo）

| 依赖 | 版本 | 用途 |
|------|------|------|
| `serde` | 1 | 序列化/反序列化（#[serde(tag = "type")] 枚举标签） |
| `serde_json` | 1 | JSON Value 解析 |

<!-- @generated v0.5.1 -->
<!-- @baseline commit=f67115370991f3521ab8aece00f990d651886eac generated=2026-06-26T12:00:00+08:00 -->
