use crate::error::{AppError, Result};
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessageRow {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[tauri::command]
pub async fn send_chat_message(
    state: State<'_, AppState>,
    handle: AppHandle,
    prompt: String,
) -> Result<()> {
    let harness = {
        let guard = state.ai_harness.lock().await;
        guard.clone().ok_or_else(|| {
            AppError::Ai(
                "No AI provider configured. Add an API key in Settings.".to_string(),
            )
        })?
    };

    let mut history = state.chat_history.lock().await;

    match harness.run_turn(&prompt, &mut history, &handle).await {
        Ok(response) => {
            persist_messages(&state, &prompt, &response)?;
            handle.emit("chat_done", ()).ok();
            Ok(())
        }
        Err(e) => {
            handle.emit("chat_error", e.to_string()).ok();
            Err(e)
        }
    }
}

fn persist_messages(state: &AppState, user: &str, assistant: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let db = &state.db;
    let conn = db.0.lock().unwrap();

    conn.execute(
        "INSERT INTO chat_messages (id, role, content, created_at) VALUES (?1, 'user', ?2, ?3)",
        rusqlite::params![uuid::Uuid::new_v4().to_string(), user, now],
    )?;

    conn.execute(
        "INSERT INTO chat_messages (id, role, content, created_at) VALUES (?1, 'assistant', ?2, ?3)",
        rusqlite::params![uuid::Uuid::new_v4().to_string(), assistant, now],
    )?;

    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn get_chat_history(state: State<'_, AppState>) -> Result<Vec<ChatMessageRow>> {
    let db = &state.db;
    let conn = db.0.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT id, role, content, created_at FROM chat_messages ORDER BY created_at ASC",
    )?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ChatMessageRow {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn clear_chat_history(state: State<'_, AppState>) -> Result<()> {
    let db = &state.db;
    let conn = db.0.lock().unwrap();
    conn.execute("DELETE FROM chat_messages", [])?;
    Ok(())
}
