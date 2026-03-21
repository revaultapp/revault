mod commands;
mod core;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::compress::compress_images,
            commands::compress::compress_to_target,
            commands::convert::convert_images,
            commands::dedupe::find_duplicates,
            commands::organize::organize_by_date,
            commands::organize::rename_batch,
            commands::privacy::read_metadata,
            commands::privacy::strip_files,
            commands::privacy::strip_files_selective,
            commands::resize::resize_images,
            commands::scanner::scan_folder,
            commands::thumbnail::generate_thumbnail,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
