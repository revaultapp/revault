use crate::core::resize;

#[tauri::command]
pub async fn resize_images(
    paths: Vec<String>,
    width: u32,
    height: u32,
    mode: resize::ResizeMode,
    quality: Option<f32>,
    output_dir: Option<String>,
) -> Result<Vec<resize::ResizeResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(resize::resize_batch(
            &paths,
            width,
            height,
            mode,
            quality,
            output_dir.as_deref(),
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
