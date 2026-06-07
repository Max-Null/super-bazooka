/// Integration test: spawn claude CLI and verify stream-json parsing.
///
/// This test requires `claude` CLI to be installed and configured
/// (API key set in ~/.claude/settings.json or env vars).
/// It sends a trivial prompt and verifies we get valid stream-json events.
///
/// Run with: cargo test --test integration_test -- --ignored
/// (marked #[ignore] by default since it requires network + API key)

use std::io::BufRead;
use std::process::{Command, Stdio};

fn find_claude() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let path = std::path::Path::new(&appdata).join("npm").join("claude.cmd");
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        for candidate in &[
            "/usr/local/bin/claude",
            "/opt/homebrew/bin/claude",
        ] {
            if std::path::Path::new(candidate).exists() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

#[test]
#[ignore = "requires claude CLI + API key + network"]
fn test_claude_says_hello() {
    let claude_path = find_claude().expect("claude CLI not found");

    let mut child = Command::new(&claude_path)
        .args([
            "--print",
            "--output-format",
            "stream-json",
            "--verbose",
            "--dangerously-skip-permissions",
            "--max-turns",
            "1",
            "Reply with exactly: OK",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn claude");

    let stdout = child.stdout.take().unwrap();
    let reader = std::io::BufReader::new(stdout);

    let mut events = Vec::new();
    let mut has_assistant = false;
    let mut has_result = false;

    for line in reader.lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }

        let parsed: serde_json::Value =
            serde_json::from_str(&line).expect(&format!("Failed to parse JSON: {}", line));

        let event_type = parsed["type"].as_str().unwrap_or("unknown");
        events.push(event_type.to_string());

        match event_type {
            "system" => {
                // system/init should have session_id, tools, model
                let subtype = parsed["subtype"].as_str().unwrap_or("");
                println!("[system/{}] session_id={:?}", subtype, parsed["session_id"]);
            }
            "assistant" => {
                has_assistant = true;
                let content = parsed["message"]["content"].as_array();
                if let Some(blocks) = content {
                    for block in blocks {
                        match block["type"].as_str() {
                            Some("text") => {
                                println!(
                                    "[assistant text] {}",
                                    block["text"].as_str().unwrap_or("")
                                );
                            }
                            Some("thinking") => {
                                println!(
                                    "[assistant thinking] {}",
                                    block["thinking"].as_str().unwrap_or("")
                                );
                            }
                            Some("tool_use") => {
                                println!(
                                    "[assistant tool_use] {}",
                                    block["name"].as_str().unwrap_or("?")
                                );
                            }
                            other => println!("[assistant {}] {:?}", other.unwrap_or("?"), block),
                        }
                    }
                }
            }
            "user" => {
                println!("[user] tool result event");
            }
            "result" => {
                has_result = true;
                let subtype = parsed["subtype"].as_str().unwrap_or("");
                let is_error = parsed["is_error"].as_bool().unwrap_or(false);
                println!("[result] subtype={} is_error={}", subtype, is_error);
                if is_error {
                    let errors = parsed["errors"].as_array();
                    println!("[result errors] {:?}", errors);
                }
            }
            other => println!("[{}] {:?}", other, parsed),
        }
    }

    let status = child.wait().expect("Failed to wait on claude");
    println!("Exit code: {:?}", status.code());
    println!("Events: {:?}", events);

    assert!(has_assistant, "Should have at least one assistant event");
    assert!(has_result, "Should have a result event");
    assert!(status.success(), "claude should exit successfully");
}

#[test]
fn test_protocol_parsing_unit() {
    // Fast unit test: verify we can parse all expected event types
    let samples = vec![
        r#"{"type":"system","subtype":"init","session_id":"test-1","tools":[],"mcp_servers":[],"model":"test","cwd":"/tmp"}"#,
        r#"{"type":"assistant","message":{"id":"m1","type":"message","role":"assistant","content":[{"type":"text","text":"hello"}],"model":"test","stop_reason":"end_turn"},"session_id":"test-1","parent_tool_use_id":null}"#,
        r#"{"type":"result","subtype":"success","result":"ok","num_turns":1,"total_cost_usd":0.001,"duration_ms":500,"session_id":"test-1"}"#,
    ];

    for sample in samples {
        let parsed: serde_json::Value = serde_json::from_str(sample)
            .unwrap_or_else(|_| panic!("Failed to parse: {}", sample));
        let event_type = parsed["type"].as_str().unwrap();
        println!("Parsed: type={}", event_type);
        assert!(!event_type.is_empty());
    }
}
