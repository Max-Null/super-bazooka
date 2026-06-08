pub mod db;
pub mod process;
pub mod protocol;
pub mod session;

use std::sync::Arc;

use db::Db;
use process::{spawn_claude_session, ProcessManager, SpawnParams, StdinManager};
use session::{SessionManager};
use tauri::State;
use tokio::sync::Mutex;

pub struct AppState {
    pub db: Db,
    pub process_manager: Arc<Mutex<ProcessManager>>,
    pub stdin_manager: Arc<Mutex<StdinManager>>,
    pub session_manager: Arc<Mutex<SessionManager>>,
}

/// Detect the project root directory from the Tauri app's CWD.
///
/// In dev mode (`cargo tauri dev`), CWD is `src-tauri/` — we use the parent.
/// In production, the binary is installed elsewhere; we walk up to find
/// `src-tauri/tauri.conf.json` or fall back to the CWD.
fn detect_project_root() -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // Dev mode: CWD is usually <project>/src-tauri → use parent
    let path = std::path::Path::new(&cwd);
    if path.file_name().map(|n| n == "src-tauri").unwrap_or(false) {
        if let Some(parent) = path.parent() {
            let parent_str = parent.to_string_lossy().to_string();
            if parent.join("src-tauri").join("tauri.conf.json").exists() {
                return parent_str;
            }
        }
    }

    // Walk up to find the project root (has src-tauri/tauri.conf.json)
    for ancestor in path.ancestors() {
        if ancestor.join("src-tauri").join("tauri.conf.json").exists() {
            return ancestor.to_string_lossy().to_string();
        }
    }

    // Fallback to CWD for production installations
    cwd
}

// ── Tauri Commands ──

#[tauri::command]
async fn send_message(
    state: State<'_, AppState>,
    session_id: String,
    message: String,
    plan_mode: bool,
    auto_mode: bool,
    permission_mode: String,
    effort: String,
    ultracode: bool,
    model: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Look up claude session UUID for resume + CWD + model
    let (resume_id, cwd, session_model) = {
        let session = state.session_manager.lock().await;
        let s = session.get_session(&session_id);
        let resume = session.get_claude_session(&session_id);
        let (cwd, model) = match s {
            Ok(s) => (s.cwd, s.model),
            Err(_) => (
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                String::new(),
            ),
        };
        (resume, cwd, model)
    };

    // Save user message to DB + auto-title on first message
    {
        let session = state.session_manager.lock().await;
        let msg_id = format!("{}-u", chrono_now());
        let _ = session.save_message(&msg_id, &session_id, "user", &message, "{}");
        let _ = session.auto_title_from_first_message(&session_id, &message);
    }

    // Spawn claude using the three-thread model
    // Use frontend-provided model if given, otherwise fall back to session's stored model
    let spawn_model = model.filter(|m| !m.is_empty()).unwrap_or(session_model);
    let params = SpawnParams {
        session_id: session_id.clone(),
        message,
        resume_id,
        plan_mode,
        auto_mode,
        permission_mode,
        effort,
        ultracode,
        cwd,
        model: spawn_model,
    };

    let stdin_mgr = state.stdin_manager.clone();
    let managed = spawn_claude_session(params, app_handle, stdin_mgr)
        .await
        .map_err(|e| format!("Failed: {}", e))?;

    // Register in process manager for future kill/lifecycle
    {
        let mut pm = state.process_manager.lock().await;
        pm.register(session_id.clone(), managed);
    }

    Ok(session_id)
}

#[tauri::command]
async fn store_claude_session(
    state: State<'_, AppState>,
    our_session_id: String,
    claude_session_id: String,
) -> Result<(), String> {
    let session = state.session_manager.lock().await;
    session.set_claude_session(&our_session_id, &claude_session_id)
}

#[tauri::command]
async fn connect_llm(
    state: State<'_, AppState>,
    api_key: String,
    base_url: String,
    model: String,
) -> Result<String, String> {
    let session = state.session_manager.lock().await;
    session
        .test_connection(&api_key, &base_url, &model)
        .await
        .map_err(|e| format!("Connection failed: {}", e))
}

#[tauri::command]
async fn stop_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let pm = state.process_manager.lock().await;
    pm.kill(&session_id).await?;
    // Update status in DB
    let session = state.session_manager.lock().await;
    let _ = session.set_session_status(&session_id, "completed");
    Ok(())
}

// ── New: Session management commands ──

#[tauri::command]
async fn create_session(
    state: State<'_, AppState>,
    model: Option<String>,
) -> Result<session::Session, String> {
    let session = state.session_manager.lock().await;
    let cwd = detect_project_root();
    session.create_session(&cwd, &model.unwrap_or_default())
}

#[tauri::command]
async fn list_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<session::Session>, String> {
    let session = state.session_manager.lock().await;
    session.list_sessions()
}

#[tauri::command]
async fn delete_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let session = state.session_manager.lock().await;
    session.delete_session(&session_id)
}

#[tauri::command]
async fn rename_session(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<(), String> {
    let session = state.session_manager.lock().await;
    session.rename_session(&session_id, &title)
}

#[tauri::command]
async fn get_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<session::Session, String> {
    let session = state.session_manager.lock().await;
    session.get_session(&session_id)
}

#[tauri::command]
async fn send_stdin(
    state: State<'_, AppState>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let mut stdin_mgr = state.stdin_manager.lock().await;
    stdin_mgr.send(&session_id, &data).await
}

#[tauri::command]
async fn add_approved_scenario(
    state: State<'_, AppState>,
    tool_name: String,
    pattern: String,
) -> Result<(), String> {
    let session = state.session_manager.lock().await;
    session.add_approved_scenario(&tool_name, &pattern)
}

#[tauri::command]
async fn remove_approved_scenario(
    state: State<'_, AppState>,
    tool_name: String,
    pattern: String,
) -> Result<(), String> {
    let session = state.session_manager.lock().await;
    session.remove_approved_scenario(&tool_name, &pattern)
}

#[tauri::command]
async fn list_approved_scenarios(
    state: State<'_, AppState>,
) -> Result<Vec<(String, String)>, String> {
    let session = state.session_manager.lock().await;
    session.list_approved_scenarios()
}

#[tauri::command]
async fn save_message(
    state: State<'_, AppState>,
    id: String,
    session_id: String,
    role: String,
    content: String,
    token_usage: Option<String>,
) -> Result<(), String> {
    let session = state.session_manager.lock().await;
    session.save_message(&id, &session_id, &role, &content, &token_usage.unwrap_or_default())
}

#[tauri::command]
async fn list_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<session::Message>, String> {
    let session = state.session_manager.lock().await;
    session.list_messages(&session_id)
}

// ── File operations ──

#[derive(serde::Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
}

#[tauri::command]
async fn list_dir(path: String) -> Result<Vec<FileEntry>, String> {
    let entries = std::fs::read_dir(&path)
        .map_err(|e| format!("Failed to read dir '{}': {}", path, e))?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let p = entry.path();
        result.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: p.to_string_lossy().to_string(),
            is_dir: p.is_dir(),
            size: if p.is_file() { p.metadata().ok().map(|m| m.len()).unwrap_or(0) } else { 0 },
        });
    }
    result.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(result)
}

#[tauri::command]
async fn read_file_content(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read '{}': {}", path, e))
}

#[tauri::command]
async fn get_workspace_root() -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("{}", e))?;
    let root = if cwd.ends_with("src-tauri") { cwd.parent().unwrap_or(&cwd).to_path_buf() } else { cwd };
    Ok(root.to_string_lossy().to_string())
}

#[tauri::command]
async fn reveal_in_explorer(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let p = std::path::Path::new(&path);
        if p.is_dir() {
            std::process::Command::new("explorer")
                .arg(p)
                .spawn()
                .map_err(|e| format!("{}", e))?;
        } else {
            std::process::Command::new("explorer")
                .arg("/select,")
                .arg(p)
                .spawn()
                .map_err(|e| format!("{}", e))?;
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let p = std::path::Path::new(&path);
        let dir = if p.is_dir() { p } else { p.parent().unwrap_or(p) };
        std::process::Command::new("open")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("{}", e))?;
    }
    Ok(())
}

// ── Entry Point ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Db::new().expect("Failed to initialize SQLite database");
    let session_mgr = Arc::new(Mutex::new(SessionManager::new(db.clone())));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            db: db.clone(),
            process_manager: Arc::new(Mutex::new(ProcessManager::new())),
            stdin_manager: Arc::new(Mutex::new(StdinManager::new())),
            session_manager: session_mgr,
        })
        .invoke_handler(tauri::generate_handler![
            send_message,
            store_claude_session,
            connect_llm,
            stop_session,
            create_session,
            list_sessions,
            delete_session,
            rename_session,
            get_session,
            send_stdin,
            add_approved_scenario,
            remove_approved_scenario,
            list_approved_scenarios,
            save_message,
            list_messages,
            list_dir,
            read_file_content,
            get_workspace_root,
            reveal_in_explorer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default()
}
