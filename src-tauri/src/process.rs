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
    pub async fn kill(&self, id: &str) -> Result<(), String> {
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
        if let Ok(appdata) = std::env::var("APPDATA") {
            let path = std::path::Path::new(&appdata).join("npm").join("claude.cmd");
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let path = std::path::Path::new(&userprofile)
                .join("AppData")
                .join("Roaming")
                .join("npm")
                .join("claude.cmd");
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
            "/home/linuxbrew/.linuxbrew/bin/claude",
        ] {
            if std::path::Path::new(candidate).exists() {
                return Some(candidate.to_string());
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
fn sync_permission_settings(auto_mode: bool) -> Result<(), String> {
    let settings_path = dirs::home_dir()
        .ok_or("No home dir")?
        .join(".claude")
        .join("settings.json");

    let content = std::fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings.json: {}", e))?;

    let mut settings: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid settings.json: {}", e))?;

    let target = if auto_mode { "auto" } else { "default" };
    let current = settings["permissions"]["defaultMode"]
        .as_str()
        .unwrap_or("default");

    // Only write if the value actually changed (avoid unnecessary I/O)
    if current != target {
        settings["permissions"]["defaultMode"] = serde_json::Value::String(target.to_string());
        std::fs::write(&settings_path, serde_json::to_string_pretty(&settings).unwrap())
            .map_err(|e| format!("Failed to write settings.json: {}", e))?;
    }

    Ok(())
}

pub async fn spawn_claude_session(
    params: SpawnParams,
    app_handle: tauri::AppHandle,
    stdin_manager: Arc<Mutex<StdinManager>>,
) -> Result<Arc<Mutex<ManagedProcess>>, String> {
    let claude_path =
        find_claude().unwrap_or_else(|| "claude".to_string());

    // Sync settings.json: auto mode writes "auto", other modes revert to "default"
    sync_permission_settings(params.auto_mode)?;

    // Build command args
    let mut args = vec![
        "--print".to_string(),
        "--output-format".to_string(), "stream-json".to_string(),
        "--verbose".to_string(),
    ];

    // NOTE: --model is intentionally NOT passed to the CLI.
    // The Claude CLI uses its own configuration (~/.claude/settings.json)
    // to determine the model. The GUI's model setting (e.g. deepseek-v4-pro[1M])
    // is stored in the session record for reference but is a different provider.
    // Passing a non-Anthropic model name to the Claude CLI would cause it to fail.

    // Let the CLI access the user's global config directory
    if let Some(home) = dirs::home_dir() {
        let claude_config = home.join(".claude");
        if claude_config.exists() {
            args.push("--add-dir".to_string());
            args.push(claude_config.to_string_lossy().to_string());
        }
    }

    // Permission mode → CLI flag mapping
    // CLI accepts: default | acceptEdits | bypassPermissions | plan
    // "auto" is NOT a CLI flag — it's permissions.defaultMode in settings.json
    // For auto mode: CLI runs with "default" → emits control_requests → frontend decides
    let cli_perm = if params.plan_mode {
        "plan".to_string()
    } else if params.auto_mode {
        "default".to_string()  // auto = default + frontend handles control_requests
    } else {
        // Pass through the exact permission mode from UI
        // acceptEdits / bypassPermissions / default
        params.permission_mode.clone()
    };

    args.push("--permission-mode".to_string());
    args.push(cli_perm);

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

    args.push(params.message.clone());

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
    let _stdin = child.stdin.take().ok_or("Failed to capture stdin")?;

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

    tokio::spawn(async move {
        let mut child = child; // take ownership

        tokio::select! {
            status = child.wait() => {
                let ev = ProcessExitedEvent {
                    session_id: sid_waiter.clone(),
                    exit_code: status.as_ref().ok().and_then(|s| s.code()),
                    success: status.map(|s| s.success()).unwrap_or(false),
                };
                let _ = app_waiter.emit("process-exited", &ev);
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
                // Capture session_id from system/init
                if let Some(claude_sid) = parsed.capture_session_id() {
                    let _ = app_reader.emit(
                        "session-created",
                        &serde_json::json!({
                            "ourId": sid_reader,
                            "claudeSessionId": claude_sid,
                        }),
                    );
                }

                let frontend_event = parsed.to_frontend_event();
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
