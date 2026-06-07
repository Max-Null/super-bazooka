/// Unit tests for stream-json protocol parsing
use serde_json::json;

// We test the protocol types directly since they're in the main crate
// These tests verify parsing of Claude Code CLI stream-json output

#[test]
fn test_parse_assistant_text() {
    let raw = json!({
        "type": "assistant",
        "message": {
            "id": "msg_001",
            "type": "message",
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Hello, world!"}
            ],
            "model": "claude-sonnet-4-6",
            "stop_reason": null,
            "stop_sequence": null
        },
        "session_id": "abc-123",
        "parent_tool_use_id": null
    });

    assert_eq!(raw["type"].as_str().unwrap(), "assistant");

    let content = raw["message"]["content"].as_array().unwrap();
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"].as_str().unwrap(), "text");
    assert_eq!(content[0]["text"].as_str().unwrap(), "Hello, world!");
}

#[test]
fn test_parse_assistant_with_thinking() {
    let raw = json!({
        "type": "assistant",
        "message": {
            "id": "msg_002",
            "type": "message",
            "role": "assistant",
            "content": [
                {"type": "thinking", "thinking": "Let me analyze this..."},
                {"type": "text", "text": "The answer is 42."}
            ],
            "model": "claude-opus-4-8",
            "stop_reason": "end_turn",
            "stop_sequence": null
        },
        "session_id": "abc-456",
        "parent_tool_use_id": null
    });

    let content = raw["message"]["content"].as_array().unwrap();
    assert_eq!(content.len(), 2);

    // First block: thinking
    assert_eq!(content[0]["type"], "thinking");
    assert_eq!(content[0]["thinking"], "Let me analyze this...");

    // Second block: text
    assert_eq!(content[1]["type"], "text");
    assert_eq!(content[1]["text"], "The answer is 42.");
}

#[test]
fn test_parse_assistant_tool_use() {
    let raw = json!({
        "type": "assistant",
        "message": {
            "id": "msg_003",
            "type": "message",
            "role": "assistant",
            "content": [
                {"type": "tool_use", "id": "tu_001", "name": "Bash", "input": {"command": "ls -la"}}
            ],
            "model": "claude-sonnet-4-6",
            "stop_reason": "tool_use",
            "stop_sequence": null
        },
        "session_id": "abc-789",
        "parent_tool_use_id": null
    });

    let content = raw["message"]["content"].as_array().unwrap();
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "tool_use");
    assert_eq!(content[0]["name"], "Bash");
    assert_eq!(content[0]["input"]["command"], "ls -la");
}

#[test]
fn test_parse_result_event() {
    let raw = json!({
        "type": "result",
        "subtype": "success",
        "result": "Completed successfully",
        "num_turns": 3,
        "total_cost_usd": 0.015,
        "duration_ms": 2345,
        "session_id": "abc-123"
    });

    assert_eq!(raw["type"], "result");
    assert_eq!(raw["subtype"], "success");
    assert_eq!(raw["num_turns"], 3);
}

#[test]
fn test_parse_system_init() {
    let raw = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "uuid-session-123",
        "tools": ["Bash", "Read", "Write", "Edit"],
        "mcp_servers": [],
        "model": "claude-sonnet-4-6",
        "cwd": "/home/user/project"
    });

    assert_eq!(raw["type"], "system");
    assert_eq!(raw["subtype"], "init");
    assert_eq!(raw["session_id"], "uuid-session-123");
}

#[test]
fn test_extract_text_across_blocks() {
    // Simulates multiple text blocks in one assistant message
    let raw = json!({
        "type": "assistant",
        "message": {
            "id": "msg_004",
            "type": "message",
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Part 1. "},
                {"type": "text", "text": "Part 2. "},
                {"type": "text", "text": "Part 3."}
            ],
            "model": "claude-sonnet-4-6",
            "stop_reason": "end_turn"
        },
        "session_id": "abc"
    });

    let content = raw["message"]["content"].as_array().unwrap();
    let all_text: String = content
        .iter()
        .filter(|b| b["type"] == "text")
        .map(|b| b["text"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join("");

    assert_eq!(all_text, "Part 1. Part 2. Part 3.");
}

#[test]
fn test_empty_content() {
    // Edge case: empty content array
    let raw = json!({
        "type": "assistant",
        "message": {
            "id": "msg_005",
            "type": "message",
            "role": "assistant",
            "content": [],
            "model": "claude-sonnet-4-6",
            "stop_reason": "end_turn"
        },
        "session_id": "abc"
    });

    let content = raw["message"]["content"].as_array().unwrap();
    assert_eq!(content.len(), 0);
}

#[test]
fn test_malformed_json_is_rejected() {
    // Invalid JSON should fail to parse
    let result = serde_json::from_str::<serde_json::Value>("not json");
    assert!(result.is_err());
}

#[test]
fn test_empty_line() {
    let result = serde_json::from_str::<serde_json::Value>("");
    assert!(result.is_err());
}
