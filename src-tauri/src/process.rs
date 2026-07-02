//! Process management — based on TOKENICODE's three-thread model.
//!
//! Architecture:
//!   Waiter Task       → owns child, tokio::select! { child.wait() | kill_rx => kill }
//!   Stdout Reader     → BufReader::lines, parse NDJSON, route events, EOF → process_exit
//!   Stdin Manager     → HashMap<id, ChildStdin>, routes frontend messages to CLI
//!
//! Reference: TOKENICODE src-tauri/src/commands/claude_process.rs

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;

use serde::Serialize;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{oneshot, Mutex, Notify};

use crate::protocol::StreamLine;
use crate::session::SessionManager;

// ── Data Structures ──

pub struct ManagedProcess {
    pub session_id: String,
    pub pid: u32,
    kill_tx: Option<oneshot::Sender<()>>,
    pub exit_notify: Arc<Notify>,
}

pub struct ProcessManager {
    processes: HashMap<String, Arc<Mutex<ManagedProcess>>>,
}

pub struct StdinManager {
    handles: HashMap<String, ChildStdin>,
}

// ── ProcessManager ──

impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            processes: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: String, process: Arc<Mutex<ManagedProcess>>) {
        self.processes.insert(id, process);
    }

    pub fn get(&self, id: &str) -> Option<Arc<Mutex<ManagedProcess>>> {
        self.processes.get(id).cloned()
    }

    /// Kill a session process gracefully (send kill_tx), then wait for exit_notify.
    /// 会自动从 HashMap 清除，防止 send_message 往已关闭管道写入。
    pub async fn kill(&mut self, id: &str) -> Result<(), String> {
        let proc = self
            .processes
            .get(id)
            .ok_or_else(|| format!("Session {} not found", id))?;
        let mut proc = proc.lock().await;

        if let Some(tx) = proc.kill_tx.take() {
            let _ = tx.send(());
        }

        // Wait for process exit (with timeout)
        tokio::time::timeout(std::time::Duration::from_secs(5), proc.exit_notify.notified())
            .await
            .map_err(|_| "Kill timeout".to_string())?;

        // 进程已退出，从管理器中移除（否则 send_message 会误判进程仍存活）
        drop(proc);
        self.processes.remove(id);

        Ok(())
    }
}

// ── StdinManager ──

impl StdinManager {
    pub fn new() -> Self {
        StdinManager {
            handles: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: String, stdin: ChildStdin) {
        self.handles.insert(id, stdin);
    }

    pub async fn send(&mut self, id: &str, data: &str) -> Result<(), String> {
        let stdin = self
            .handles
            .get_mut(id)
            .ok_or_else(|| format!("Stdin handle not found for session {}", id))?;

        stdin
            .write_all(data.as_bytes())
            .await
            .map_err(|e| format!("Stdin write error: {}", e))?;

        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| format!("Stdin write error: {}", e))?;

        Ok(())
    }

    pub fn remove(&mut self, id: &str) {
        self.handles.remove(id);
    }
}


// ── CLI spawner ──

/// Locate claude CLI executable on this system.
pub fn find_claude() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        // 1st: Native install (claude install <ver>), easy to downgrade
        if let Ok(home) = std::env::var("USERPROFILE") {
            let native = std::path::Path::new(&home).join(".local").join("bin").join("claude.exe");
            if native.exists() {
                return Some(native.to_string_lossy().to_string());
            }
        }
        // 2nd: npm global install (Roaming + Local — both are common prefixes)
        let npm_bases: Vec<std::path::PathBuf> = {
            let mut bases = Vec::new();
            // APPDATA = Roaming
            if let Ok(v) = std::env::var("APPDATA") {
                bases.push(std::path::PathBuf::from(&v));
            } else if let Ok(home) = std::env::var("USERPROFILE") {
                bases.push(std::path::Path::new(&home).join("AppData").join("Roaming"));
            }
            // LOCALAPPDATA = Local (default npm prefix on many Windows installs)
            if let Ok(v) = std::env::var("LOCALAPPDATA") {
                bases.push(std::path::PathBuf::from(&v));
            } else if let Ok(home) = std::env::var("USERPROFILE") {
                bases.push(std::path::Path::new(&home).join("AppData").join("Local"));
            }
            bases
        };
        for base_path in &npm_bases {
            // Try claude.exe first (bypasses cmd wrapper)
            let exe = base_path
                .join("npm")
                .join("node_modules")
                .join("@anthropic-ai")
                .join("claude-code")
                .join("bin")
                .join("claude.exe");
            if exe.exists() {
                return Some(exe.to_string_lossy().to_string());
            }
            // Fallback to claude.cmd
            let cmd = base_path.join("npm").join("claude.cmd");
            if cmd.exists() {
                return Some(cmd.to_string_lossy().to_string());
            }
        }
        // 3rd: npm root -g — 覆盖任意 npm prefix（D:\Program Files\nodejs 等非标准路径）
        if let Ok(output) = std::process::Command::new("npm").args(["root", "-g"]).output() {
            if output.status.success() {
                let npm_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let exe = std::path::Path::new(&npm_root)
                    .join("@anthropic-ai")
                    .join("claude-code")
                    .join("bin")
                    .join("claude.exe");
                if exe.exists() {
                    return Some(exe.to_string_lossy().to_string());
                }
                // fallback: claude.cmd in npm prefix
                let cmd = std::path::Path::new(&npm_root).parent()
                    .map(|p| p.join("claude.cmd"));
                if let Some(ref cmd_path) = cmd {
                    if cmd_path.exists() {
                        return Some(cmd_path.to_string_lossy().to_string());
                    }
                }
            }
        }
        // 4th: PATH fallback via where.exe (pip install, Chocolatey, manual PATH, etc.)
        if let Ok(output) = std::process::Command::new("where").arg("claude").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Take the first result line, prefer .exe over .cmd
                let mut best: Option<String> = None;
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if trimmed.ends_with(".exe") {
                        return Some(trimmed.to_string());
                    }
                    if best.is_none() {
                        best = Some(trimmed.to_string());
                    }
                }
                if let Some(found) = best {
                    return Some(found);
                }
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        for candidate in &[
            "/usr/local/bin/claude",
            "/opt/homebrew/bin/claude",
            "/home/linuxbrew/.linuxbrew/bin/claude",
        ] {
            if std::path::Path::new(candidate).exists() {
                return Some(candidate.to_string());
            }
        }
        // PATH fallback via which
        if let Ok(output) = std::process::Command::new("which").arg("claude").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// Parameters for spawning a claude session.
pub struct SpawnParams {
    pub session_id: String,
    pub message: String,
    pub resume_id: Option<String>,
    pub plan_mode: bool,
    pub auto_mode: bool,
    pub permission_mode: String,
    pub effort: String,
    pub ultracode: bool,
    pub cwd: String,
    pub model: String,
    pub file_paths: Vec<String>,
    pub claude_path: Option<String>,
}

/// Result of spawning a claude session.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessExitedEvent {
    pub session_id: String,
    pub exit_code: Option<i32>,
    pub success: bool,
}

/// Spawn a claude CLI process using the three-thread model.
///
/// Returns the ManagedProcess handle (so the caller can kill it later).
/// Events are emitted to the app_handle via Tauri events during the session.
/// Registers the stdin handle with the provided StdinManager.
/// Keep settings.json in sync with the current permission mode.
/// "auto" is NOT a CLI flag — it's `permissions.defaultMode` in settings.json.
/// Other modes use CLI `--permission-mode` flag directly.
/// Sync `permissions.defaultMode` in settings.json to match the current permission mode.
///
/// When `auto_mode` is true, write "auto" (classifier-based, not a CLI flag).
/// When `plan_mode` is true, write "plan".
/// Otherwise, write the explicit `permission_mode` string directly,
/// so bypassPermissions / dontAsk / acceptEdits are preserved
/// for future CLI runs outside the GUI.
fn sync_permission_settings(auto_mode: bool, plan_mode: bool, permission_mode: &str) -> Result<(), String> {
    let settings_path = dirs::home_dir()
        .ok_or("No home dir")?
        .join(".claude")
        .join("settings.json");

    let content = std::fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings.json: {}", e))?;

    let mut settings: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid settings.json: {}", e))?;

    // 白名单校验 permission_mode，拒绝无效值
    let target = if auto_mode {
        "auto"
    } else if plan_mode {
        "plan"
    } else {
        match permission_mode {
            "default" | "acceptEdits" | "bypassPermissions" | "dontAsk" => permission_mode,
            other => return Err(format!("无效的权限模式: {}", other)),
        }
    };
    let current = settings["permissions"]["defaultMode"]
        .as_str()
        .unwrap_or("default");

    // Only write if the value actually changed (avoid unnecessary I/O)
    if current != target {
        settings["permissions"]["defaultMode"] = serde_json::Value::String(target.to_string());
        let json = serde_json::to_string_pretty(&settings)
            .map_err(|e| format!("Failed to serialize settings.json: {}", e))?;
        std::fs::write(&settings_path, json)
            .map_err(|e| format!("Failed to write settings.json: {}", e))?;
    }

    Ok(())
}

pub async fn spawn_claude_session(
    params: SpawnParams,
    app_handle: tauri::AppHandle,
    stdin_manager: Arc<Mutex<StdinManager>>,
    session_manager: Arc<Mutex<SessionManager>>,
) -> Result<Arc<Mutex<ManagedProcess>>, String> {
    let claude_path = params.claude_path
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| find_claude().unwrap_or_else(|| "claude".to_string()));

    // Sync settings.json so future CLI runs (outside GUI) use the same mode
    sync_permission_settings(params.auto_mode, params.plan_mode, &params.permission_mode)?;

    // Build command args (per docs: stream-json + verbose + include-partial-messages
    // for real-time token streaming)
    // --input-format stream-json 是关键 flag：让 CC 进入双工 NDJSON 模式，通过 stdout 发
    // control_request(can_use_tool) 审批请求，通过 stdin 收 control_response 审批决策。
    // 不加此 flag 时 --print 模式直接 auto-approve/auto-deny，不会弹审批。
    // 协议规范：不要同时使用 --print 和 --input-format stream-json。
    // --print 是单轮非交互模式，--input-format stream-json 是双工流模式，两者互斥。
    // 消息已通过 stdin NDJSON 发送，不需要 --print。
    let mut args = vec![
        "--output-format".to_string(), "stream-json".to_string(),
        "--input-format".to_string(), "stream-json".to_string(),
        "--verbose".to_string(),
        "--include-partial-messages".to_string(),
        // 关键 flag: 让 CC 通过 stdout 发送 can_use_tool 审批请求，通过 stdin 接收审批决策。
        // 此 flag 不在 claude --help 中显示（SDK 内部专用），但 SDK 和第三方实现均使用。
        // 不加则 --print 非交互模式下所有 tool 直接 auto-deny
        "--permission-prompt-tool".to_string(), "stdio".to_string(),
    ]; // --permission-mode 在下文根据用户设置追加，控制审批行为

    // NOTE: --model is intentionally NOT passed to the CLI.
    // The Claude CLI uses its own configuration (~/.claude/settings.json)
    // to determine the model. The GUI's model setting (e.g. deepseek-v4-pro[1M])
    // is stored in the session record for reference but is a different provider.
    // Passing a non-Anthropic model name to the Claude CLI would cause it to fail.

    // Note: CLI reads ~/.claude/settings.json automatically — no --add-dir needed.
    // --add-dir grants file EDIT permissions, which should NOT include CLI config.

    // Add parent directories of attached files so Claude can read them
    for file_path in &params.file_paths {
        if let Some(parent) = std::path::Path::new(file_path).parent() {
            if parent.exists() {
                args.push("--add-dir".to_string());
                args.push(parent.to_string_lossy().to_string());
            }
        }
    }

    // Permission mode → CLI flag mapping
    // CLI accepts (per docs): default | acceptEdits | bypassPermissions | plan | auto | dontAsk
    let cli_perm = if params.plan_mode {
        "plan".to_string()
    } else if params.auto_mode {
        "auto".to_string()
    } else {
        params.permission_mode.clone()
    };

    args.push("--permission-mode".to_string());
    args.push(cli_perm.clone());

    // Effort → CLI flag mapping
    //
    // API effort levels (model-native): low | medium | high | xhigh | max
    // ultracode is NOT an effort level — it's a Claude Code session setting
    // that combines xhigh reasoning + auto Workflow orchestration.
    //
    // For ultracode we:
    //   1. Send --effort xhigh (the actual model parameter)
    //   2. Pass --settings '{"ultracode":true}' (enables auto-workflow harness behavior)
    //
    // max is a valid API effort level, passed directly.
    let effort = match params.effort.as_str() {
        "ultracode" => "xhigh".to_string(),
        e => e.to_string(), // low | medium | high | xhigh | max all pass through
    };
    args.push("--effort".to_string());
    args.push(effort);

    // Ultracode mode: inject --settings to enable auto Workflow orchestration
    // Docs: --settings accepts a file path or inline JSON string
    if params.ultracode {
        args.push("--settings".to_string());
        args.push(r#"{"ultracode":true}"#.to_string());
    }

    // Max turns
    args.push("--max-turns".to_string());
    args.push("10".to_string());

    if let Some(ref resume_id) = params.resume_id {
        args.push("--resume".to_string());
        args.push(resume_id.clone());
    }

    // --input-format stream-json 模式下，消息通过 stdin NDJSON 发送，不能作为命令行参数
    // args.push(params.message.clone());  // 已移除：消息在下文通过 stdin 写入

    #[cfg(target_os = "windows")]
    let mut cmd = {
        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        let mut c = Command::new(&claude_path);
        c.args(&args);
        c.stdin(Stdio::piped());
        c.stdout(Stdio::piped());
        c.stderr(Stdio::piped());
        // Set CWD to project root so Claude CLI can access user files
        c.current_dir(&params.cwd);
        // CREATE_NO_WINDOW = 0x08000000 — prevent console flash on Windows
        c.creation_flags(0x08000000);
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = Command::new(&claude_path);
        c.args(&args);
        c.stdin(Stdio::piped());
        c.stdout(Stdio::piped());
        c.stderr(Stdio::piped());
        c.current_dir(&params.cwd);
        c
    };

    let mut child = cmd.spawn().map_err(|e| {
        format!(
            "Failed to spawn claude (tried: '{}'): {}. Is Claude Code CLI installed?",
            claude_path, e
        )
    })?;

    let pid = child.id().unwrap_or(0);
    let session_id = params.session_id.clone();

    // Take stdout, stderr, stdin
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let mut _stdin = child.stdin.take().ok_or("Failed to capture stdin")?;

    // 构建含附件路径的用户消息文本（复用 lib.rs 公共函数，避免重复）
    let full_message = crate::build_user_message(&params.message, &params.file_paths);

    // --input-format stream-json 模式：先通过 stdin 发送用户消息（NDJSON），再注册供后续控制响应使用
    let user_msg = serde_json::json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": full_message
        }
    });
    let mut msg_json = serde_json::to_string(&user_msg)
        .map_err(|e| format!("序列化用户消息失败: {}", e))?;
    msg_json.push('\n');
    _stdin.write_all(msg_json.as_bytes()).await
        .map_err(|e| format!("stdin 写入用户消息失败: {}", e))?;

    // 发送 set_permission_mode 控制请求，告知 CC 当前权限模式
    let perm_req = serde_json::json!({
        "type": "control_request",
        "request_id": format!("perm_{}", params.session_id),
        "request": {
            "subtype": "set_permission_mode",
            "mode": cli_perm  // 已在上文根据 plan_mode/auto_mode/permission_mode 解析
        }
    });
    let mut perm_json = serde_json::to_string(&perm_req)
        .map_err(|e| format!("序列化 set_permission_mode 失败: {}", e))?;
    perm_json.push('\n');
    _stdin.write_all(perm_json.as_bytes()).await
        .map_err(|e| format!("stdin 写入 set_permission_mode 失败: {}", e))?;

    // Register stdin handle for later writes (e.g., permission responses)
    {
        let mut sm = stdin_manager.lock().await;
        sm.register(session_id.clone(), _stdin);
    }

    // ── Thread 1: Waiter (owns child, handles kill signal) ──
    let (kill_tx, kill_rx) = oneshot::channel::<()>();
    let exit_notify = Arc::new(Notify::new());
    let exit_notify_waiter = exit_notify.clone();
    let sid_waiter = session_id.clone();
    let app_waiter = app_handle.clone();
    let session_mgr = session_manager.clone();

    tokio::spawn(async move {
        let mut child = child; // take ownership

        tokio::select! {
            status = child.wait() => {
                let exit_code = status.as_ref().ok().and_then(|s| s.code());
                let success = status.map(|s| s.success()).unwrap_or(false);
                let ev = ProcessExitedEvent {
                    session_id: sid_waiter.clone(),
                    exit_code,
                    success,
                };
                let _ = app_waiter.emit("process-exited", &ev);

                let new_status = if success { "completed" } else { "error" };
                let mgr = session_mgr.lock().await;
                if let Err(e) = mgr.set_session_status(&sid_waiter, new_status) {
                    eprintln!(
                        "[SB] Failed to update session status for {} -> {}: {}",
                        sid_waiter, new_status, e
                    );
                }
            }
            _ = kill_rx => {
                // Graceful kill attempt
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
        }

        exit_notify_waiter.notify_waiters();
    });

    // ── Thread 2: Stdout Reader (parse NDJSON, emit events) ──
    let sid_reader = session_id.clone();
    let app_reader = app_handle.clone();

    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }

            // Debug: emit raw line
            let _ = app_reader.emit("stream-debug", &line);

            if let Some(parsed) = StreamLine::parse(&line) {
                // Capture session_id + MCP servers from system/init
                if let Some(claude_sid) = parsed.capture_session_id() {
                    let mcp_servers = parsed.capture_mcp_servers();
                    let _ = app_reader.emit(
                        "session-created",
                        &serde_json::json!({
                            "ourId": sid_reader,
                            "claudeSessionId": claude_sid,
                            "mcpServers": mcp_servers,
                        }),
                    );
                }

                let frontend_event = parsed.to_frontend_event(&sid_reader);
                let _ = app_reader.emit("stream-event", &frontend_event);
            } else {
                let _ = app_reader.emit(
                    "stream-error",
                    &format!("Failed to parse: {}", line),
                );
            }
        }
    });

    // ── Thread 3: Stderr Reader ──
    let app_stderr = app_handle.clone();

    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if !line.trim().is_empty() {
                let _ = app_stderr.emit("stream-error", &line);
            }
        }
    });

    // ── Build ManagedProcess handle ──
    let managed = Arc::new(Mutex::new(ManagedProcess {
        session_id: session_id.clone(),
        pid,
        kill_tx: Some(kill_tx),
        exit_notify,
    }));

    Ok(managed)
}
