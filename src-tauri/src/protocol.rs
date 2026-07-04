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
        // 多态：可以是 string、[{type:"text", text:"..."}]、或 null
        content: Value,
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
                                    content_blocks: None,
                                    tool_results: None,
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
                                    content_blocks: None,
                                    tool_results: None,
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
                                    content_blocks: None,
                                    tool_results: None,
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
                            content_blocks: None,
                            tool_results: None,
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
                let mut ordered_blocks: Vec<Value> = Vec::new();

                if let Some(content) = self.inner["message"]["content"].as_array() {
                    for block in content {
                        match block["type"].as_str() {
                            Some("text") => {
                                if let Some(t) = block["text"].as_str() {
                                    texts.push(t.to_string());
                                }
                            }
                            // thinking 块字段名不稳定：某些版本用 "text" 代替 "thinking"
                            Some("thinking") => {
                                let t = block["thinking"].as_str()
                                    .or_else(|| block["text"].as_str())
                                    .unwrap_or("");
                                if !t.is_empty() {
                                    thinkings.push(t.to_string());
                                }
                            }
                            Some("tool_use") => {
                                tool_uses.push(block.clone());
                            }
                            // tool_result 也可能出现在 assistant 事件中（较少见但合法）
                            Some("tool_result") => {
                                // 在 assistant 分支中暂不独立处理，仅记录到 ordered_blocks
                            }
                            _ => {}
                        }
                        // 按原始顺序记录块（直接 clone CC 事件中的 block，统一序列化路径）
                        ordered_blocks.push(block.clone());
                    }
                }
                let has_blocks = !ordered_blocks.is_empty();

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
                    content_blocks: if has_blocks { Some(ordered_blocks) } else { None },
                    tool_results: None,
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
                    content_blocks: None,
                    tool_results: None,
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
                    content_blocks: None,
                    tool_results: None,
                }
            }
            // user 事件携带 tool_result 块——工具执行结果。
            // content 字段是多态的：string | [{type:"text", text:"..."}] | null，
            // 通过 extract_tool_result_content() 归一化为纯文本。
            "user" => {
                let mut tool_results = Vec::new();
                if let Some(content) = self.inner["message"]["content"].as_array() {
                    for block in content {
                        if block["type"].as_str() == Some("tool_result") {
                            let raw_content = &block["content"];
                            tool_results.push(ToolResultData {
                                tool_use_id: block["tool_use_id"].as_str().unwrap_or("").to_string(),
                                content: extract_tool_result_content(raw_content),
                                is_error: block["is_error"].as_bool(),
                            });
                        }
                    }
                }
                StreamFrontendEvent {
                    event_type: "user".to_string(),
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
                    content_blocks: None,
                    tool_results: if tool_results.is_empty() { None } else { Some(tool_results) },
                }
            }
            // system/result 是旧版 CC 的轮次结束事件（legacy），转换为等价 result 事件。
            "system" => {
                if self.inner["subtype"].as_str() == Some("result") {
                    StreamFrontendEvent {
                        event_type: "result".to_string(),
                        session_id: session_id.to_string(),
                        text: String::new(),
                        thinking: String::new(),
                        tool_use: None,
                        control_request: None,
                        is_final: true,
                        error: None,
                        duration_ms: self.inner["duration_ms"].as_u64(),
                        input_tokens: self.inner["usage"]["input_tokens"].as_u64().map(|v| v as u32),
                        output_tokens: self.inner["usage"]["output_tokens"].as_u64().map(|v| v as u32),
                        cost_usd: self.inner["total_cost_usd"].as_f64(),
                        content_blocks: None,
                        tool_results: None,
                    }
                } else {
                    StreamFrontendEvent {
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
                        content_blocks: None,
                        tool_results: None,
                    }
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
                content_blocks: None,
                tool_results: None,
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
            content_blocks: None,
            tool_results: None,
        }
    }
}

/// 归一化 tool_result.content 的三种形态 → 纯文本字符串。
/// content 可以是：纯文本 string、块数组 [{type:"text", text:"..."}]、或 null。
fn extract_tool_result_content(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr.iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

/// 工具执行结果数据（从 user 事件的 tool_result 块中提取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultData {
    pub tool_use_id: String,
    pub content: String,
    pub is_error: Option<bool>,
}

/// Simplified event sent to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamFrontendEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub session_id: String,
    /// Deprecated: 合并后的纯文本（保留向后兼容），新代码请用 content_blocks
    pub text: String,
    /// Deprecated: 合并后的思考文本，新代码请用 content_blocks
    pub thinking: String,
    pub tool_use: Option<Vec<Value>>,
    pub control_request: Option<ControlRequest>,
    pub is_final: bool,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub cost_usd: Option<f64>,
    /// 保持 CC 原始 content 块顺序的数组，解决"文字全堆在工具调用后面"的问题
    #[serde(default)]
    pub content_blocks: Option<Vec<Value>>,
    /// 工具执行结果（从 user 事件的 tool_result 块中提取）
    #[serde(default)]
    pub tool_results: Option<Vec<ToolResultData>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_tool_result_content_string() {
        let content = json!("plain text result");
        assert_eq!(extract_tool_result_content(&content), "plain text result");
    }

    #[test]
    fn test_extract_tool_result_content_array() {
        let content = json!([
            {"type": "text", "text": "line one\n"},
            {"type": "text", "text": "line two"}
        ]);
        assert_eq!(extract_tool_result_content(&content), "line one\nline two");
    }

    #[test]
    fn test_extract_tool_result_content_null() {
        let content = json!(null);
        assert_eq!(extract_tool_result_content(&content), "");
    }

    #[test]
    fn test_user_event_with_tool_result() {
        let raw = json!({
            "type": "user",
            "session_id": "test-sid",
            "parent_tool_use_id": null,
            "message": {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "call_abc123",
                        "content": "file contents here",
                        "is_error": false
                    }
                ]
            }
        });
        let line = StreamLine { inner: raw };
        let evt = line.to_frontend_event("test-sid");
        assert_eq!(evt.event_type, "user");
        assert_eq!(evt.text, "");
        let trs = evt.tool_results.expect("should have tool_results");
        assert_eq!(trs.len(), 1);
        assert_eq!(trs[0].tool_use_id, "call_abc123");
        assert_eq!(trs[0].content, "file contents here");
        assert_eq!(trs[0].is_error, Some(false));
    }

    #[test]
    fn test_user_event_with_tool_result_error() {
        let raw = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "call_err",
                        "content": "command not found",
                        "is_error": true
                    }
                ]
            }
        });
        let line = StreamLine { inner: raw };
        let evt = line.to_frontend_event("sid");
        assert_eq!(evt.event_type, "user");
        let trs = evt.tool_results.expect("should have tool_results");
        assert_eq!(trs[0].is_error, Some(true));
    }

    #[test]
    fn test_user_event_without_tool_result_is_empty() {
        let raw = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {"type": "text", "text": "some text"}
                ]
            }
        });
        let line = StreamLine { inner: raw };
        let evt = line.to_frontend_event("sid");
        assert_eq!(evt.event_type, "user");
        assert!(evt.tool_results.is_none(), "should have no tool_results when no tool_result blocks");
    }

    #[test]
    fn test_thinking_block_fallback_to_text_field() {
        // 某些版本的 thinking 块使用 "text" 字段名而非 "thinking"
        let raw = json!({
            "type": "assistant",
            "session_id": "test-sid",
            "message": {
                "role": "assistant",
                "content": [
                    {"type": "thinking", "text": "this uses text field"},
                    {"type": "thinking", "thinking": "this uses thinking field"}
                ]
            }
        });
        let line = StreamLine { inner: raw };
        let evt = line.to_frontend_event("test-sid");
        // 两种形式都应被提取
        assert_eq!(evt.thinking, "this uses text fieldthis uses thinking field");
    }

    #[test]
    fn test_system_result_legacy_event() {
        let raw = json!({
            "type": "system",
            "subtype": "result",
            "session_id": "legacy-sid",
            "duration_ms": 5000,
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50
            },
            "total_cost_usd": 0.01
        });
        let line = StreamLine { inner: raw };
        let evt = line.to_frontend_event("legacy-sid");
        assert_eq!(evt.event_type, "result");
        assert!(evt.is_final);
        assert_eq!(evt.duration_ms, Some(5000));
        assert_eq!(evt.input_tokens, Some(100));
        assert_eq!(evt.output_tokens, Some(50));
    }

    #[test]
    fn test_stream_frontend_event_serialization() {
        let evt = StreamFrontendEvent {
            event_type: "user".to_string(),
            session_id: "s1".to_string(),
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
            content_blocks: None,
            tool_results: Some(vec![ToolResultData {
                tool_use_id: "call_1".to_string(),
                content: "result content".to_string(),
                is_error: Some(false),
            }]),
        };
        let json_str = serde_json::to_string(&evt).expect("serialization should succeed");
        let parsed: Value = serde_json::from_str(&json_str).expect("deserialization should succeed");
        assert_eq!(parsed["type"], "user");
        let trs = parsed["tool_results"].as_array().expect("tool_results should exist");
        assert_eq!(trs.len(), 1);
        assert_eq!(trs[0]["tool_use_id"], "call_1");
        assert_eq!(trs[0]["content"], "result content");
    }
}
