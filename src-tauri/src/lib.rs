pub mod content;
pub mod storage;
pub mod publisher;
pub mod network;
pub mod node;
pub mod ipc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ipc::commands::publish_site,
            ipc::commands::get_published_sites,
            ipc::commands::unpublish_site,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
