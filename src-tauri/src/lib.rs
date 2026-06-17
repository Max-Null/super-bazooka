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
    file_paths: Option<Vec<String>>,
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
        file_paths: file_paths.unwrap_or_default(),
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
async fn read_file_base64(path: String) -> Result<String, String> {
    use base64::Engine;
    let bytes = std::fs::read(&path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

#[tauri::command]
async fn get_auto_mode_status() -> Result<bool, String> {
    let settings_path = dirs::home_dir()
        .ok_or("No home dir")?
        .join(".claude")
        .join("settings.json");

    match std::fs::read_to_string(&settings_path) {
        Ok(content) => {
            let settings: serde_json::Value =
                serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {}", e))?;
            let mode = settings["permissions"]["defaultMode"]
                .as_str()
                .unwrap_or("default");
            Ok(mode == "auto")
        }
        Err(_) => Ok(false), // No settings.json → not auto
    }
}

#[tauri::command]
async fn write_file(path: String, content: String) -> Result<(), String> {
    // 只允许写入 ~/.claude/ 目录内的文件
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;
    let claude_dir = home.join(".claude");
    // normalize：统一用 backslash，便于 Windows 下前缀比较
    let resolved = std::fs::canonicalize(&path).unwrap_or_else(|_| {
        let p = std::path::Path::new(&path);
        if p.is_absolute() { p.to_path_buf() } else { home.join(p) }
    });
    let resolved_s = resolved.to_string_lossy().replace("\\\\?\\", "");
    let claude_s = claude_dir.to_string_lossy().replace("\\\\?\\", "");
    let claude_json = home.join(".claude.json").to_string_lossy().replace("\\\\?\\", "");
    if !resolved_s.starts_with(&claude_s) && resolved_s != claude_json {
        return Err(format!("路径超出允许范围: {}", path));
    }
    std::fs::write(&resolved, &content)
        .map_err(|e| format!("写入文件失败 '{}': {}", path, e))
}

/// 返回 ~/.claude 目录路径
#[tauri::command]
async fn get_claude_dir() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;
    Ok(home.join(".claude").to_string_lossy().to_string())
}

// ── 项目描述缓存（翻译 + 持久化）──

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct DescriptionItem {
    item_type: String,
    name: String,
    #[serde(default)]
    desc_en: Option<String>,
    #[serde(default)]
    desc_zh: Option<String>,
}

/// 确保每个 item 都有中英双语描述。已有翻译的直接从 DB 返回；
/// 缺失中文的调用 DeepSeek API 翻译后存入 DB。
#[tauri::command]
async fn ensure_item_descriptions(
    state: State<'_, AppState>,
    items: Vec<DescriptionItem>,
    api_key: String,
    base_url: String,
) -> Result<Vec<DescriptionItem>, String> {
    // 如果 cc-gui 未配置 API Key，尝试从 ~/.claude/settings.json 读取 ANTHROPIC_AUTH_TOKEN
    let api_key = if api_key.is_empty() {
        read_claude_api_key().unwrap_or_default()
    } else {
        api_key
    };

    // 第一步：在 DB 锁内查询缓存，收集需要翻译的项
    let (mut results, to_translate): (Vec<DescriptionItem>, Vec<(usize, String)>) = {
        let db = state.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        let mut results = Vec::new();
        let mut to_translate = Vec::new();

        for item in &items {
            let mut out = item.clone();
            let cached: Option<(Option<String>, Option<String>)> = db
                .query_row(
                    "SELECT desc_en, desc_zh FROM item_descriptions WHERE item_type = ?1 AND name = ?2",
                    rusqlite::params![item.item_type, item.name],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            if let Some((cached_en, cached_zh)) = cached {
                // 英文变了，或中文缺失 → 重新翻译
                let needs_retranslate = cached_zh.is_none()
                    || match (&cached_en, &item.desc_en) {
                        (Some(old), Some(new)) if old != new => true,
                        (None, Some(_)) => true,
                        _ => false,
                    };
                if needs_retranslate {
                    if let Some(ref desc_en) = item.desc_en {
                        to_translate.push((results.len(), desc_en.clone()));
                    }
                    out.desc_en = item.desc_en.clone().or(cached_en);
                    out.desc_zh = cached_zh; // 暂时保旧
                } else {
                    out.desc_en = cached_en.or(item.desc_en.clone());
                    out.desc_zh = cached_zh;
                }
            } else if let Some(ref desc_en) = item.desc_en {
                to_translate.push((results.len(), desc_en.clone()));
            }

            results.push(out);
        }
        (results, to_translate)
    }; // DB 锁在此释放

    // 第二步：无锁状态下调用 API 翻译
    let mut translations: Vec<(usize, String)> = Vec::new();
    for (idx, desc_en) in &to_translate {
        match translate_to_chinese(&api_key, &base_url, desc_en).await {
            Ok(zh) => translations.push((*idx, zh)),
            Err(e) => eprintln!("translate error for {}[{}]: {}", items[*idx].name, idx, e),
        }
    }

    // 第三步：重新获取 DB 锁，保存翻译结果
    {
        let db = state.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        for (idx, zh) in &translations {
            let item = &items[*idx];
            results[*idx].desc_zh = Some(zh.clone());
            if let Some(ref desc_en) = item.desc_en {
                let _ = db.execute(
                    "INSERT OR REPLACE INTO item_descriptions (item_type, name, desc_en, desc_zh, translated_at) VALUES (?1, ?2, ?3, ?4, datetime('now'))",
                    rusqlite::params![item.item_type, item.name, desc_en, zh],
                );
            }
        }
    }

    Ok(results)
}

/// 调用 DeepSeek API 将英文描述翻译为中文
/// 从 ~/.claude/settings.json 读取 cc-gui 相关配置
#[derive(serde::Serialize)]
struct ClaudeSettings {
    api_key: String,
    base_url: String,
    model: String,
    effort: String,
    /// permissions.defaultMode: auto | plan | default | acceptEdits | bypassPermissions | dontAsk
    permission_mode: String,
}

#[tauri::command]
fn get_claude_settings() -> Result<ClaudeSettings, String> {
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;
    let path = home.join(".claude").join("settings.json");
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 settings.json 失败: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("解析 settings.json 失败: {}", e))?;

    let env = &json["env"];
    Ok(ClaudeSettings {
        api_key: env["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap_or("").to_string(),
        base_url: env["ANTHROPIC_BASE_URL"].as_str().unwrap_or("https://api.deepseek.com").to_string(),
        model: env["ANTHROPIC_MODEL"]
            .as_str()
            .or(env["ANTHROPIC_DEFAULT_OPUS_MODEL"].as_str())
            .unwrap_or("deepseek-v4-pro[1M]")
            .to_string(),
        effort: json["effortLevel"].as_str().unwrap_or("high").to_string(),
        permission_mode: json["permissions"]["defaultMode"]
            .as_str()
            .unwrap_or("default")
            .to_string(),
    })
}

/// 将 cc-gui 配置写入 ~/.claude/settings.json
#[tauri::command]
fn set_claude_settings(
    api_key: String,
    base_url: String,
    model: String,
    effort: String,
    permission_mode: String,
) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;
    let path = home.join(".claude").join("settings.json");

    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 settings.json 失败: {}", e))?;
    let mut json: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("解析 settings.json 失败: {}", e))?;

    if json["env"].is_null() || !json["env"].is_object() {
        json["env"] = serde_json::Value::Object(serde_json::Map::new());
    }
    if json["permissions"].is_null() || !json["permissions"].is_object() {
        json["permissions"] = serde_json::Value::Object(serde_json::Map::new());
    }

    json["env"]["ANTHROPIC_AUTH_TOKEN"] = serde_json::Value::String(api_key);
    json["env"]["ANTHROPIC_BASE_URL"] = serde_json::Value::String(base_url);
    json["env"]["ANTHROPIC_MODEL"] = serde_json::Value::String(model);
    json["effortLevel"] = serde_json::Value::String(effort);
    json["permissions"]["defaultMode"] = serde_json::Value::String(permission_mode);

    let out = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("序列化 settings.json 失败: {}", e))?;
    std::fs::write(&path, out)
        .map_err(|e| format!("写入 settings.json 失败: {}", e))?;

    Ok(())
}

/// 清空翻译缓存，下次加载时全部重新翻译
#[tauri::command]
fn clear_item_descriptions(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
    db.execute("DELETE FROM item_descriptions", [])
        .map_err(|e| format!("清空翻译缓存失败: {}", e))?;
    Ok(())
}

/// 只清空 MCP 描述缓存
#[tauri::command]
fn clear_mcp_descriptions(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
    db.execute("DELETE FROM item_descriptions WHERE item_type = 'mcp'", [])
        .map_err(|e| format!("清空 MCP 描述缓存失败: {}", e))?;
    Ok(())
}

/// 从 ~/.claude/settings.json 读取 ANTHROPIC_AUTH_TOKEN（内部翻译用）
fn read_claude_api_key() -> Option<String> {
    let home = dirs::home_dir()?;
    let path = home.join(".claude").join("settings.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    json["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().map(|s| s.to_string())
}

/// 读取 CLAUDE_CODE_SUBAGENT_MODEL，fallback 到 deepseek-chat
fn subagent_model() -> String {
    let home = dirs::home_dir().unwrap_or_default();
    let path = home.join(".claude").join("settings.json");
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(m) = json["env"]["CLAUDE_CODE_SUBAGENT_MODEL"].as_str() {
                if !m.is_empty() { return m.to_string(); }
            }
        }
    }
    "deepseek-chat".into()
}

fn openai_base(base_url: &str) -> String {
    base_url
        .trim_end_matches('/')
        .trim_end_matches("/anthropic")
        .trim_end_matches("/v1")
        .to_string()
}

async fn translate_to_chinese(
    api_key: &str,
    base_url: &str,
    text: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/v1/chat/completions", openai_base(base_url));

    let body = serde_json::json!({
        "model": subagent_model(),
        "messages": [
            {"role": "system", "content": "你是一个专业翻译。将英文描述翻译为简洁的中文。只输出中文译文，不要任何解释。"},
            {"role": "user", "content": format!("翻译: {}", text)}
        ],
        "max_tokens": 400,
        "temperature": 0.3
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("API {} ({})", status, body_text));
    }

    // 解析 DeepSeek chat completions 响应
    let json: serde_json::Value =
        serde_json::from_str(&body_text).map_err(|e| format!("JSON parse: {}", e))?;

    let translated = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if translated.is_empty() {
        Err("Empty translation result".into())
    } else {
        Ok(translated)
    }
}

/// 用 DeepSeek API 为 MCP 服务器名称批量生成中文描述，优先读 DB 缓存
#[tauri::command]
async fn generate_mcp_descriptions(
    state: State<'_, AppState>,
    names: Vec<String>,
    api_key: String,
    base_url: String,
) -> Result<Vec<DescriptionItem>, String> {
    if names.is_empty() {
        return Ok(vec![]);
    }

    // 第一步：查 DB 缓存，只对缺失的调 API
    let (mut cached, missing): (Vec<DescriptionItem>, Vec<String>) = {
        let db = state.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        let mut cached = Vec::new();
        let mut missing = Vec::new();
        for name in &names {
            let zh: Option<String> = db
                .query_row(
                    "SELECT desc_zh FROM item_descriptions WHERE item_type = 'mcp' AND name = ?1",
                    rusqlite::params![name],
                    |row| row.get(0),
                )
                .ok();
            if let Some(zh) = zh {
                cached.push(DescriptionItem {
                    item_type: "mcp".into(),
                    name: name.clone(),
                    desc_en: None,
                    desc_zh: Some(zh),
                });
            } else {
                missing.push(name.clone());
            }
        }
        (cached, missing)
    };

    if missing.is_empty() {
        return Ok(cached);
    }

    let api_key = if api_key.is_empty() {
        read_claude_api_key().unwrap_or_default()
    } else {
        api_key
    };
    if api_key.is_empty() {
        return Err("未配置 API Key".into());
    }

    // 第二步：调 API 为缺失的名称生成描述
    let client = reqwest::Client::new();
    let url = format!("{}/v1/chat/completions", openai_base(&base_url));

    let name_list = missing
        .iter()
        .enumerate()
        .map(|(i, n)| format!("{}. {}", i + 1, n))
        .collect::<Vec<_>>()
        .join("\n");

    let body = serde_json::json!({
        "model": subagent_model(),
        "messages": [
            {"role": "system", "content": "你是一个技术文档助手。为每个 MCP 服务器写一句 25 字以内的中文简介。仅输出 JSON 对象，格式：{\"名称\":\"简介\",...}。不要任何其他文字。"},
            {"role": "user", "content": format!("以下 MCP 服务器各写一句简介：\n{}", name_list)}
        ],
        "max_tokens": 800,
        "temperature": 0.3
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("API {} ({})", status, body_text));
    }

    let json: serde_json::Value =
        serde_json::from_str(&body_text).map_err(|e| format!("JSON parse: {}", e))?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    let descs: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析描述 JSON 失败: {} — {}", e, content))?;

    let mut fresh: Vec<DescriptionItem> = Vec::new();
    if let Some(obj) = descs.as_object() {
        let db = state.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        for (name, desc_val) in obj {
            if let Some(zh) = desc_val.as_str() {
                let _ = db.execute(
                    "INSERT OR REPLACE INTO item_descriptions (item_type, name, desc_zh, translated_at) VALUES ('mcp', ?1, ?2, datetime('now'))",
                    rusqlite::params![name, zh],
                );
                fresh.push(DescriptionItem {
                    item_type: "mcp".into(),
                    name: name.clone(),
                    desc_en: None,
                    desc_zh: Some(zh.to_string()),
                });
            }
        }
    }

    // 合并缓存 + 新生成的
    cached.extend(fresh);
    Ok(cached)
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
    let db = Db::new().unwrap_or_else(|e| {
        eprintln!("Fatal: Failed to initialize SQLite database: {}", e);
        eprintln!("Data directory: {:?}", db::db_dir());
        std::process::exit(1);
    });
    let session_mgr = Arc::new(Mutex::new(SessionManager::new(db.clone())));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
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
            get_auto_mode_status,
            read_file_base64,
            write_file,
            get_claude_dir,
            get_claude_settings,
            set_claude_settings,
            ensure_item_descriptions,
            generate_mcp_descriptions,
            clear_item_descriptions,
            clear_mcp_descriptions,
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
