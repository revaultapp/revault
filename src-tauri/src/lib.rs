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
        .invoke_handler(tauri::generate_handler![
            commands::compress::compress_images,
            commands::compress::preview_compress,
            commands::convert::convert_images,
            commands::dedupe::find_duplicates,
            commands::delete::delete_files,
            commands::privacy::read_metadata,
            commands::privacy::strip_files,
            commands::privacy::strip_files_selective,
            commands::resize::resize_images,
            commands::scanner::scan_folder,
            commands::thumbnail::generate_thumbnail,
            commands::thumbnail::get_file_sizes,
            commands::video::compress_video,
            commands::video::cancel_video_compress,
            commands::video::check_ffmpeg,
            commands::video::download_ffmpeg,
            commands::video::reveal_video_output,
            commands::watermark::apply_text_watermark,
            commands::watermark::apply_image_watermark,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
