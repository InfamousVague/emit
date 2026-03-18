use crate::launcher::CommandEntry;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn search(query: String) -> Vec<CommandEntry> {
    crate::launcher::search_commands(&query)
}

#[tauri::command]
pub fn execute_command(id: String) -> Result<String, String> {
    crate::launcher::execute(&id)
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}
