use crate::core::resize;

#[tauri::command]
pub async fn resize_images(
    paths: Vec<String>,
    width: u32,
    height: u32,
    mode: resize::ResizeMode,
    quality: Option<f32>,
    output_dir: Option<String>,
    strip_gps: Option<bool>,
) -> Result<Vec<resize::ResizeResult>, String> {
    let strip_gps = strip_gps.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || {
        Ok(resize::resize_batch(
            &paths,
            width,
            height,
            mode,
            quality,
            output_dir.as_deref(),
            strip_gps,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
