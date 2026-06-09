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

    pub fn to_frontend_event(&self) -> StreamFrontendEvent {
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
                                    text: String::new(),
                                    thinking: String::new(),
                                    tool_use: None,
                                    control_request: None,
                                    is_final: false,
                                    error: None,
                                    duration_ms: None, input_tokens: None, output_tokens: None, cost_usd: None,
                                }
                            }
                            _ => StreamFrontendEvent::empty(event_type),
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
                                is_final: false,
                                error: None,
                                duration_ms: None, input_tokens: None, output_tokens: None, cost_usd: None,
                            }
                        } else {
                            StreamFrontendEvent::empty(event_type)
                        }
                    }
                    _ => StreamFrontendEvent::empty(event_type),
                }
            }
            "assistant" => {
                // Extract text and thinking content
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
                    input_tokens: None,
                    output_tokens: None,
                    cost_usd: None,
                }
            }
            "result" => {
                let duration_ms = self.inner["duration_ms"].as_u64();
                let cost_usd = self.inner["total_cost_usd"].as_f64();
                let usage = &self.inner["usage"];
                StreamFrontendEvent {
                    event_type: "result".to_string(),
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
                let subtype = self.inner["subtype"].as_str().unwrap_or("unknown");
                let tool_name = self.inner["tool_name"].as_str().map(|s| s.to_string());
                let tool_input = self.inner["tool_input"].clone();

                let control = ControlRequest {
                    subtype: subtype.to_string(),
                    tool_name,
                    tool_input,
                };

                StreamFrontendEvent {
                    event_type: "control_request".to_string(),
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
}

impl StreamFrontendEvent {
    fn empty(event_type: &str) -> Self {
        StreamFrontendEvent {
            event_type: event_type.to_string(),
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
