mod commands;
pub mod core;

use commands::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState)
        .setup(|app| {
            // 标题栏主题最终由前端按用户选择设置；这里只保证窗口存在时可被控制。
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("KC-Switch");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_paths,
            commands::providers_list,
            commands::providers_set_default_model,
            commands::providers_upsert,
            commands::providers_remove,
            commands::providers_upsert_model,
            commands::providers_diagnose,
            commands::providers_meta,
            commands::providers_discover_models,
            commands::providers_import_models,
            commands::providers_remove_model,
            commands::mcp_list,
            commands::mcp_upsert,
            commands::mcp_remove,
            commands::mcp_set_deferred,
            commands::skills_list,
            commands::skills_toggle,
            commands::skills_delete,
            commands::plugins_list,
            commands::plugins_toggle,
            commands::plugins_remove,
            commands::app_version,
            commands::set_window_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running KC-Switch");
}
