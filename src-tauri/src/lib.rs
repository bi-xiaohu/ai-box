mod commands;
mod db;
mod doc_processor;
mod embedding;
mod llm;

use db::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            let database =
                Database::new(&app_dir).expect("Failed to initialize database");
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Chat
            commands::chat::create_conversation,
            commands::chat::list_conversations,
            commands::chat::delete_conversation,
            commands::chat::rename_conversation,
            commands::chat::get_messages,
            commands::chat::send_message,
            // Settings
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::delete_setting,
            commands::settings::get_available_models,
            // Knowledge base
            commands::knowledge::list_documents,
            commands::knowledge::upload_document,
            commands::knowledge::delete_document,
            commands::knowledge::search_knowledge_base,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
