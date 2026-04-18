mod commands;
mod core;

use rayon::ThreadPoolBuilder;
use std::thread;

fn init_rayon_pool() {
    let cores = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);
    let cores = cores.max(2);
    let rayon_threads = if cores >= 4 { cores - 1 } else { cores };
    ThreadPoolBuilder::new()
        .num_threads(rayon_threads)
        .build_global()
        .expect("failed to configure rayon thread pool");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_rayon_pool();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            {
                use tauri::Manager;
                if let Some(window) = _app.get_webview_window("main") {
                    window.set_decorations(false)?;
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::compress::compress_images,
            commands::compress::preview_compress,
            commands::convert::convert_images,
            commands::dedupe::find_duplicates,
            commands::delete::delete_files,
            commands::gif::export_gif,
            commands::gif::check_gifski,
            commands::gif::download_gifski,
            commands::privacy::read_metadata,
            commands::privacy::strip_files,
            commands::privacy::strip_files_selective,
            commands::resize::resize_images,
            commands::scanner::scan_folder,
            commands::thumbnail::generate_thumbnail,
            commands::thumbnail::get_file_sizes,
            commands::video::compress_video,
            commands::video::preview_video_compression,
            commands::video::cancel_video_compress,
            commands::video::check_ffmpeg,
            commands::video::download_ffmpeg,
            commands::video::reveal_video_output,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
