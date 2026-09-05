/// Current model-download percentage, or `None` when no download is in flight.
///
/// The three `model_download_*` events are the live update path, but Tauri's
/// `emit` only reaches webviews that already hold a listener, and the startup
/// download begins in `setup()` — before the frontend has mounted, let alone
/// subscribed. Everything emitted in that window is lost, so the frontend seeds
/// itself from this query on mount and takes events from there.
#[tauri::command]
pub fn model_download_state() -> Option<u32> {
    crate::model::download_progress()
}
