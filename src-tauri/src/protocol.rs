use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Events emitted by Claude Code CLI via stream-json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "system")]
    System {
        subtype: String,
        #[serde(flatten)]
        data: Value,
    },
    #[serde(rename = "assistant")]
    Assistant {
        message: AssistantMessage,
        session_id: String,
        parent_tool_use_id: Option<String>,
        #[serde(flatten)]
        extra: Value,
    },
    #[serde(rename = "user")]
    User {
        message: UserMessage,
        session_id: String,
        parent_tool_use_id: Option<String>,
        #[serde(flatten)]
        extra: Value,
    },
    #[serde(rename = "result")]
    Result {
        subtype: String,
        result: String,
        num_turns: u32,
        total_cost_usd: Option<f64>,
        duration_ms: Option<u64>,
        #[serde(flatten)]
        extra: Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
    pub model: Option<String>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: Vec<UserContentBlock>,
    pub parent_tool_use_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserContentBlock {
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: Option<bool>,
    },
}

/// Represents a single parsed line that might not be a complete event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamLine {
    #[serde(flatten)]
    pub inner: Value,
}

impl StreamLine {
    pub fn parse(line: &str) -> Option<Self> {
        serde_json::from_str::<Value>(line.trim()).ok().map(|inner| StreamLine { inner })
    }

    /// If this is a system/init event, extract the claude session ID
    pub fn capture_session_id(&self) -> Option<String> {
        if self.inner["type"].as_str()? == "system"
            && self.inner["subtype"].as_str()? == "init"
        {
            self.inner["session_id"].as_str().map(|s| s.to_string())
        } else {
            None
        }
    }

    /// If this is a system/init event, extract connected MCP server names.
    /// Prefers the native `mcp_servers` field; falls back to parsing `mcp__<server>__<tool>` tool names.
    pub fn capture_mcp_servers(&self) -> Vec<String> {
        if self.inner["type"].as_str() != Some("system")
            || self.inner["subtype"].as_str() != Some("init")
        {
            return vec![];
        }

        // 优先：从 mcp_servers 原生字段读取（Claude Code 直接提供的服务器列表）
        if let Some(servers) = self.inner["mcp_servers"].as_array() {
            if !servers.is_empty() {
                return servers
                    .iter()
                    .filter_map(|s| s["name"].as_str().map(|n| n.to_string()))
                    .collect();
            }
        }

        // 兜底：从 tools 名称 mcp__<server>__<tool> 中提取
        let tools = match self.inner["tools"].as_array() {
            Some(t) => t,
            None => return vec![],
        };
        let mut servers = std::collections::BTreeSet::new();
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

    pub fn to_frontend_event(&self, session_id: &str) -> StreamFrontendEvent {
        let event_type = self.inner["type"].as_str().unwrap_or("unknown");

        match event_type {
            "stream_event" => {
                // With --include-partial-messages, incremental events arrive as stream_event
                let ev = &self.inner["event"];
                match ev["type"].as_str() {
                    Some("content_block_delta") => {
                        let delta = &ev["delta"];
                        match delta["type"].as_str() {
                            Some("text_delta") => {
                                let text = delta["text"].as_str().unwrap_or("");
                                StreamFrontendEvent {
                                    event_type: "assistant".to_string(),
                                    session_id: session_id.to_string(),
                                    text: text.to_string(),
                                    thinking: String::new(),
                                    tool_use: None,
                                    control_request: None,
                                    is_final: false,
                                    error: None,
                                    duration_ms: None, input_tokens: None, output_tokens: None, cost_usd: None,
                                }
                            }
                            Some("input_json_delta") => {
                                let _json = delta["partial_json"].as_str().unwrap_or("");
                                StreamFrontendEvent {
                                    event_type: "tool_input_delta".to_string(),
                                    session_id: session_id.to_string(),
                                    text: String::new(),
                                    thinking: String::new(),
                                    tool_use: None,
                                    control_request: None,
                                    is_final: false,
                                    error: None,
                                    duration_ms: None, input_tokens: None, output_tokens: None, cost_usd: None,
                                }
                            }
                            _ => StreamFrontendEvent::empty(event_type, session_id),
                        }
                    }
                    Some("content_block_start") => {
                        let block = &ev["content_block"];
                        if block["type"].as_str() == Some("tool_use") {
                            StreamFrontendEvent {
                                event_type: "tool_start".to_string(),
                                text: String::new(),
                                thinking: String::new(),
                                tool_use: Some(vec![serde_json::json!({
                                    "id": block["id"],
                                    "name": block["name"],
                                })]),
                                control_request: None,
                                session_id: session_id.to_string(),
                                is_final: false,
                                error: None,
                                duration_ms: None, input_tokens: None, output_tokens: None, cost_usd: None,
                            }
                        } else {
                            StreamFrontendEvent::empty(event_type, session_id)
                        }
                    }
                    // message_delta 携带该轮 assistant 的最终 token 使用量
                    Some("message_delta") => {
                        let usage = &ev["usage"];
                        StreamFrontendEvent {
                            event_type: "token_usage".to_string(),
                            session_id: session_id.to_string(),
                            text: String::new(),
                            thinking: String::new(),
                            tool_use: None,
                            control_request: None,
                            is_final: false,
                            error: None,
                            duration_ms: None,
                            input_tokens: usage["input_tokens"].as_u64().map(|v| v as u32),
                            output_tokens: usage["output_tokens"].as_u64().map(|v| v as u32),
                            cost_usd: None,
                        }
                    }
                    _ => StreamFrontendEvent::empty(event_type, session_id),
                }
            }
            "assistant" => {
                // 从完整的 assistant 消息中提取 text / thinking / tool_use 块。
                //
                // text 与 stream_event delta 的关系：
                //   - Anthropic API：开启 --include-partial-messages 后，文本
                //     既通过 stream_event.content_block_delta text_delta 增量发送，
                //     也会在这个完整 assistant 事件中再次出现。
                //     前端会追踪是否已通过增量方式收到文本，如果已收到则跳过
                //     完整事件中的文本，避免重复。
                //   - DeepSeek API：不发送 stream_event text_delta 事件——
                //     文本只能通过这个完整 assistant 事件获取。前端没有先前的
                //     增量文本，因此直接使用此处的 text。
                //   - thinking：目前不支持增量流式传输（未来会添加 thinking_delta），
                //     所以始终从这里提取。
                //   - tool_use：需要完整的 input 块，无法通过
                //     stream_event.content_block_start 获取，所以始终从这里提取。
                let mut texts = Vec::new();
                let mut thinkings = Vec::new();
                let mut tool_uses = Vec::new();

                if let Some(content) = self.inner["message"]["content"].as_array() {
                    for block in content {
                        match block["type"].as_str() {
                            Some("text") => {
                                if let Some(t) = block["text"].as_str() {
                                    texts.push(t.to_string());
                                }
                            }
                            Some("thinking") => {
                                if let Some(t) = block["thinking"].as_str() {
                                    thinkings.push(t.to_string());
                                }
                            }
                            Some("tool_use") => {
                                tool_uses.push(block.clone());
                            }
                            _ => {}
                        }
                    }
                }

                StreamFrontendEvent {
                    event_type: "assistant".to_string(),
                    session_id: session_id.to_string(),
                    text: texts.join(""),
                    thinking: thinkings.join(""),
                    tool_use: if tool_uses.is_empty() {
                        None
                    } else {
                        Some(tool_uses)
                    },
                    control_request: None,
                    is_final: false,
                    error: None,
                    duration_ms: None,
                    // assistant 事件携带 message.usage，DeepSeek 等后端 result 事件可能不含 usage
                    input_tokens: self.inner["message"]["usage"]["input_tokens"].as_u64().map(|v| v as u32),
                    output_tokens: self.inner["message"]["usage"]["output_tokens"].as_u64().map(|v| v as u32),
                    cost_usd: self.inner["total_cost_usd"].as_f64(),
                }
            }
            "result" => {
                let duration_ms = self.inner["duration_ms"].as_u64();
                let cost_usd = self.inner["total_cost_usd"].as_f64();
                let usage = &self.inner["usage"];
                StreamFrontendEvent {
                    event_type: "result".to_string(),
                    session_id: session_id.to_string(),
                    text: String::new(),
                    thinking: String::new(),
                    tool_use: None,
                    control_request: None,
                    is_final: true,
                    error: None,
                    duration_ms,
                    input_tokens: usage["input_tokens"].as_u64().map(|v| v as u32),
                    output_tokens: usage["output_tokens"].as_u64().map(|v| v as u32),
                    cost_usd,
                }
            },
            "control_request" => {
                // Handle both flat format (legacy) and nested format under "request" key.
                // Flat:    {"type":"control_request","subtype":"...","tool_name":"...","tool_input":{...},"request_id":"..."}
                // Nested:  {"type":"control_request","request_id":"...","request":{"subtype":"...","tool_name":"...","input":{...}}}
                let req = &self.inner["request"];
                let subtype = req["subtype"].as_str()
                    .or_else(|| self.inner["subtype"].as_str())
                    .unwrap_or("unknown");
                let tool_name = req["tool_name"].as_str()
                    .or_else(|| self.inner["tool_name"].as_str())
                    .map(|s| s.to_string());
                // 用于 updatedInput 回传的工具输入
                let tool_input = if req["input"].is_object() {
                    req["input"].clone()
                } else {
                    self.inner["tool_input"].clone()
                };
                // request_id: 响应时必须原样带回
                let request_id = self.inner["request_id"].as_str().map(|s| s.to_string());

                let control = ControlRequest {
                    subtype: subtype.to_string(),
                    tool_name,
                    tool_input,
                    request_id,
                };

                StreamFrontendEvent {
                    event_type: "control_request".to_string(),
                    session_id: session_id.to_string(),
                    text: String::new(),
                    thinking: String::new(),
                    tool_use: None,
                    control_request: Some(control),
                    is_final: false,
                    error: None,
                    duration_ms: None,
                    input_tokens: None,
                    output_tokens: None,
                    cost_usd: None,
                }
            }
            _ => StreamFrontendEvent {
                event_type: event_type.to_string(),
                session_id: session_id.to_string(),
                text: String::new(),
                thinking: String::new(),
                tool_use: None,
                control_request: None,
                is_final: false,
                error: None,
                duration_ms: None,
                input_tokens: None,
                output_tokens: None,
                cost_usd: None,
            },
        }
    }
}

/// SCP control_request from CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRequest {
    pub subtype: String,
    pub tool_name: Option<String>,
    pub tool_input: Value,
    /// 控制请求的唯一 ID，响应时必须原样带回
    pub request_id: Option<String>,
}

impl StreamFrontendEvent {
    fn empty(event_type: &str, session_id: &str) -> Self {
        StreamFrontendEvent {
            event_type: event_type.to_string(),
            session_id: session_id.to_string(),
            text: String::new(),
            thinking: String::new(),
            tool_use: None,
            control_request: None,
            is_final: false,
            error: None,
            duration_ms: None, input_tokens: None,
            output_tokens: None, cost_usd: None,
        }
    }
}

/// Simplified event sent to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamFrontendEvent {
    #[serde(rename = "type")]
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
