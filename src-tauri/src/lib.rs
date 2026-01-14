use std::fs;
use std::path::Path;
pub mod network;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_file_list(folder_path: &str) -> Result<Vec<String>, String> {

    let path = Path::new(folder_path);
    if !path.is_dir() {
        return Err(format!("The path '{}' is not a valid directory.", folder_path));
    }

    match fs::read_dir(path) {
        Ok(entries) => {
            let file_names: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            Ok(file_names)
        }
        Err(e) => Err(format!("Failed to read directory '{}': {}", folder_path, e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![greet, get_file_list])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
