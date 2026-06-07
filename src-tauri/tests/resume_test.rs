/// Multi-turn context test: verifies --resume preserves conversation context.
///
/// Test flow:
///   1. Spawn claude without --resume → tell it "My name is Max"
///   2. Capture session_id from system/init
///   3. Spawn claude WITH --resume → ask "What's my name?"
///   4. Verify response contains "Max"
///
/// Run with:
///   cargo test --test resume_test test_resume_context -- --ignored --nocapture
///
/// Requires: claude CLI installed + API configured in ~/.claude/settings.json

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
    None
}

/// Spawn claude and return (exit_code, captured_session_id, all_text_content)
fn run_claude(args: &[&str]) -> (Option<i32>, Option<String>, String) {
    let claude_path = find_claude().expect("claude not found");

    let mut child = Command::new(&claude_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn claude");

    let stdout = child.stdout.take().unwrap();
    let reader = std::io::BufReader::new(stdout);

    let mut session_id: Option<String> = None;
    let mut all_text = String::new();

    for line in reader.lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }

        let parsed: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let event_type = parsed["type"].as_str().unwrap_or("");

        match event_type {
            "system" if parsed["subtype"] == "init" => {
                session_id = parsed["session_id"].as_str().map(|s| s.to_string());
            }
            "assistant" => {
                if let Some(content) = parsed["message"]["content"].as_array() {
                    for block in content {
                        if block["type"] == "text" {
                            if let Some(t) = block["text"].as_str() {
                                all_text.push_str(t);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let status = child.wait().expect("Failed to wait");
    (status.code(), session_id, all_text)
}

#[test]
#[ignore = "requires claude CLI + API key + network"]
fn test_resume_context() {
    // ── Turn 1: Establish context ──
    println!("═══ Turn 1: Establish context ═══");

    let (code1, sid, text1) = run_claude(&[
        "--print",
        "--output-format",
        "stream-json",
        "--verbose",
        "--dangerously-skip-permissions",
        "--max-turns",
        "2",
        "我的名字是Max，记住它。只需要回复：知道了。",
    ]);

    println!("Turn 1 exit code: {:?}", code1);
    println!("Turn 1 session_id: {:?}", sid);
    println!("Turn 1 text: {}", text1);

    assert!(code1 == Some(0), "Turn 1 should succeed");
    assert!(sid.is_some(), "Turn 1 should produce a session_id");
    let session_id = sid.unwrap();

    // ── Turn 2: Resume and ask ──
    println!("\n═══ Turn 2: Resume + ask ═══");

    let (code2, _, text2) = run_claude(&[
        "--print",
        "--output-format",
        "stream-json",
        "--verbose",
        "--dangerously-skip-permissions",
        "--max-turns",
        "2",
        "--resume",
        &session_id,
        "我刚才说我叫什么名字？只需要回复名字。",
    ]);

    println!("Turn 2 exit code: {:?}", code2);
    println!("Turn 2 text: {}", text2);

    assert!(code2 == Some(0), "Turn 2 should succeed");

    let lower = text2.to_lowercase();
    assert!(
        lower.contains("max"),
        "Turn 2 should mention 'Max'. Got: {}",
        text2
    );

    println!("\n✅ Multi-turn context works! Turn 2 remembered 'Max'.");
}
