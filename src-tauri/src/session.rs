//! Session manager — SQLite-backed session CRUD + API connection testing.
//!
//! Schema (from db.rs):
//!   sessions: id, title, cli_session_id, cwd, model, status, created_at, updated_at
//!   messages: id, session_id, role, content (JSON blob), token_usage, created_at

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::Db;

// ── Data types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub cli_session_id: Option<String>,
    pub cwd: String,
    pub model: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: u32,
    pub total_tokens: Option<f64>,
    pub total_cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String, // JSON blob of the full stream-json message
    pub token_usage: String,
    pub created_at: String,
}

// ── SessionManager ──

pub struct SessionManager {
    db: Db,
}

impl SessionManager {
    pub fn new(db: Db) -> Self {
        SessionManager { db }
    }

    // ── Session CRUD ──

    /// Create a new session row in SQLite. Returns the session.
    pub fn create_session(
        &self,
        cwd: &str,
        model: &str,
    ) -> Result<Session, String> {
        let id = uuid_v4();
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;

        conn.execute(
            "INSERT INTO sessions (id, title, cwd, model, status) VALUES (?1, 'New Chat', ?2, ?3, 'idle')",
            params![id, cwd, model],
        )
        .map_err(|e| format!("DB insert session: {}", e))?;

        drop(conn);
        self.get_session(&id)
    }

    /// Get a single session by id.
    pub fn get_session(&self, id: &str) -> Result<Session, String> {
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;

        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title, s.cli_session_id, s.cwd, s.model, s.status,
                        s.created_at, s.updated_at,
                        (SELECT COUNT(*) FROM messages WHERE session_id = s.id) AS msg_count
                 FROM sessions s WHERE s.id = ?1",
            )
            .map_err(|e| format!("DB prepare: {}", e))?;

        stmt.query_row(params![id], |row| {
            Ok(Session {
                id: row.get(0)?,
                title: row.get(1)?,
                cli_session_id: row.get(2)?,
                cwd: row.get(3)?,
                model: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                message_count: row.get(8)?,
                total_tokens: None,
                total_cost: None,
            })
        })
        .map_err(|e| format!("Session not found: {} ({})", id, e))
    }

    /// List all sessions, newest first.
    pub fn list_sessions(&self) -> Result<Vec<Session>, String> {
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;

        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title, s.cli_session_id, s.cwd, s.model, s.status,
                        s.created_at, s.updated_at,
                        (SELECT COUNT(*) FROM messages WHERE session_id = s.id) AS msg_count
                 FROM sessions s ORDER BY s.updated_at DESC",
            )
            .map_err(|e| format!("DB prepare: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    cli_session_id: row.get(2)?,
                    cwd: row.get(3)?,
                    model: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    message_count: row.get(8)?,
                    total_tokens: None,
                    total_cost: None,
                })
            })
            .map_err(|e| format!("DB query: {}", e))?;

        let mut sessions = Vec::new();
        for row in rows {
            let mut s = row.map_err(|e| format!("DB row: {}", e))?;
            // Compute token totals from messages
            if s.message_count > 0 {
                let mut tok_stmt = conn
                    .prepare("SELECT content FROM messages WHERE session_id = ?1 AND role = 'assistant'")
                    .map_err(|e| format!("DB prepare: {}", e))?;
                let mut tokens: f64 = 0.0;
                let mut cost: f64 = 0.0;
                let content_rows = tok_stmt
                    .query_map(params![s.id], |row| row.get::<_, String>(0))
                    .map_err(|e| format!("DB query: {}", e))?;
                for cr in content_rows {
                    if let Ok(content) = cr {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                            tokens += parsed["inputTokens"].as_f64().unwrap_or(0.0);
                            tokens += parsed["outputTokens"].as_f64().unwrap_or(0.0);
                            cost += parsed["costUSD"].as_f64().unwrap_or(0.0);
                        }
                    }
                }
                if tokens > 0.0 { s.total_tokens = Some(tokens); }
                if cost > 0.0 { s.total_cost = Some(cost); }
            }
            sessions.push(s);
        }
        Ok(sessions)
    }

    /// Delete a session and its messages (CASCADE).
    pub fn delete_session(&self, id: &str) -> Result<(), String> {
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])
            .map_err(|e| format!("DB delete: {}", e))?;
        Ok(())
    }

    /// Rename a session (update title).
    pub fn rename_session(&self, id: &str, title: &str) -> Result<(), String> {
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        conn.execute(
            "UPDATE sessions SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![title, id],
        )
        .map_err(|e| format!("DB rename: {}", e))?;
        Ok(())
    }

    /// Set the claude CLI session UUID for resume support.
    pub fn set_claude_session(&self, our_id: &str, claude_id: &str) -> Result<(), String> {
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        conn.execute(
            "UPDATE sessions SET cli_session_id = ?1, status = 'running', updated_at = datetime('now') WHERE id = ?2",
            params![claude_id, our_id],
        )
        .map_err(|e| format!("DB set claude session: {}", e))?;
        Ok(())
    }

    /// Get the stored claude session UUID for --resume.
    pub fn get_claude_session(&self, our_id: &str) -> Option<String> {
        let conn = self.db.conn.lock().ok()?;
        conn.query_row(
            "SELECT cli_session_id FROM sessions WHERE id = ?1",
            params![our_id],
            |row| row.get(0),
        )
        .ok()
        .flatten()
    }

    /// Auto-title: take first user message (first 50 chars) if title is still default.
    pub fn auto_title_from_first_message(&self, id: &str, message: &str) -> Result<(), String> {
        let title: String = message.chars().take(50).collect();
        let title = if message.chars().count() > 50 {
            format!("{}…", title)
        } else {
            title
        };
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        conn.execute(
            "UPDATE sessions SET title = ?1 WHERE id = ?2 AND title = 'New Chat'",
            params![title, id],
        )
        .map_err(|e| format!("DB auto title: {}", e))?;
        Ok(())
    }

    /// Update session status (idle / running / completed / error).
    pub fn set_session_status(&self, id: &str, status: &str) -> Result<(), String> {
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![status, id],
        )
        .map_err(|e| format!("DB set status: {}", e))?;
        Ok(())
    }

    // ── Message persistence ──

    /// Save a message to the messages table.
    pub fn save_message(
        &self,
        id: &str,
        session_id: &str,
        role: &str,
        content: &str,
        token_usage: &str,
    ) -> Result<(), String> {
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        conn.execute(
            "INSERT OR REPLACE INTO messages (id, session_id, role, content, token_usage) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, session_id, role, content, token_usage],
        )
        .map_err(|e| format!("DB save message: {}", e))?;

        // Touch session updated_at
        conn.execute(
            "UPDATE sessions SET updated_at = datetime('now'), message_count = (SELECT COUNT(*) FROM messages WHERE session_id = ?1) WHERE id = ?1",
            params![session_id],
        )
        .map_err(|e| format!("DB touch session: {}", e))?;

        Ok(())
    }

    /// List messages for a session, ordered by creation time.
    pub fn list_messages(&self, session_id: &str) -> Result<Vec<Message>, String> {
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, role, content, token_usage, created_at
                 FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| format!("DB prepare: {}", e))?;

        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok(Message {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    token_usage: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| format!("DB query: {}", e))?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row.map_err(|e| format!("DB row: {}", e))?);
        }
        Ok(messages)
    }

    // ── API connection test ──

    /// Test connection to a DeepSeek-compatible API.
    pub async fn test_connection(
        &self,
        api_key: &str,
        base_url: &str,
        model: &str,
    ) -> Result<String, String> {
        let client = reqwest::Client::new();
        let base = base_url
            .trim_end_matches('/')
            .trim_end_matches("/anthropic")
            .trim_end_matches("/v1");
        let url = format!("{}/v1/chat/completions", base);

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "user", "content": "Hi, respond with just 'ok'."}
            ],
            "max_tokens": 5
        });

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();

        if status.is_success() {
            Ok(format!("Connected! Status: {}", status))
        } else {
            Err(format!("API error {}: {}", status, body_text))
        }
    }

    // ── Approved Scenarios ──

    pub fn add_approved_scenario(&self, tool_name: &str, pattern: &str) -> Result<(), String> {
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        conn.execute(
            "INSERT OR REPLACE INTO approved_scenarios (tool_name, pattern) VALUES (?1, ?2)",
            params![tool_name, pattern],
        )
        .map_err(|e| format!("DB save approved: {}", e))?;
        Ok(())
    }

    pub fn remove_approved_scenario(&self, tool_name: &str, pattern: &str) -> Result<(), String> {
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        conn.execute(
            "DELETE FROM approved_scenarios WHERE tool_name = ?1 AND pattern = ?2",
            params![tool_name, pattern],
        )
        .map_err(|e| format!("DB remove approved: {}", e))?;
        Ok(())
    }

    pub fn is_approved(&self, tool_name: &str, _input_summary: &str) -> bool {
        let conn = self.db.conn.lock().ok();
        match conn {
            Some(c) => {
                let count: i64 = c
                    .query_row(
                        "SELECT COUNT(*) FROM approved_scenarios WHERE tool_name = ?1",
                        params![tool_name],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                count > 0
            }
            None => false,
        }
    }

    pub fn list_approved_scenarios(&self) -> Result<Vec<(String, String)>, String> {
        let conn = self.db.conn.lock().map_err(|e| format!("DB lock: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT tool_name, pattern FROM approved_scenarios ORDER BY tool_name")
            .map_err(|e| format!("DB prepare: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("DB query: {}", e))?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row.map_err(|e| format!("DB row: {}", e))?);
        }
        Ok(list)
    }
}

// ── Helpers ──

fn uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}
