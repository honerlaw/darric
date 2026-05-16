use crate::{
    ai::{claude::ClaudeProvider, gemini::GeminiProvider, harness::AgentHarness, AiProvider},
    db,
    error::Result,
    state::AppState,
};

#[tauri::command]
pub async fn save_setting(
    state: tauri::State<'_, AppState>,
    key: String,
    value: String,
) -> Result<()> {
    {
        let db = state.db.0.lock().unwrap();
        db.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
    }

    if key.starts_with("ai.") {
        let (provider_name, claude_key, gemini_key) = {
            let conn = state.db.0.lock().unwrap();
            (
                db::get_setting(&conn, "ai.provider"),
                db::get_setting(&conn, "ai.claude.api_key"),
                db::get_setting(&conn, "ai.gemini.api_key"),
            )
        };
        let provider: Option<AiProvider> = match provider_name.as_deref() {
            Some("gemini") => gemini_key.map(|k| AiProvider::Gemini(GeminiProvider::new(k))),
            _ => claude_key.map(|k| AiProvider::Claude(ClaudeProvider::new(k))),
        };
        let new_harness = provider.map(|p| AgentHarness::new(p, state.mcp.clone()));
        *state.ai_harness.lock().await = new_harness;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_setting(state: tauri::State<'_, AppState>, key: String) -> Result<Option<String>> {
    let db = state.db.0.lock().unwrap();
    let mut stmt = db.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query(rusqlite::params![key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn list_mcp_servers(state: tauri::State<'_, AppState>) -> Result<Vec<String>> {
    Ok(state.mcp.server_names())
}
