use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CompressionResult {
    pub input_path: String,
    pub output_path: String,
    pub original_size: u64,
    pub compressed_size: u64,
}

pub fn compress_batch(
    _paths: &[String],
    _quality: u8,
) -> Result<Vec<CompressionResult>, Box<dyn std::error::Error>> {
    // TODO: Implement with mozjpeg, oxipng, webp
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_batch_empty_input() {
        let result = compress_batch(&[], 80).unwrap();
        assert!(result.is_empty());
    }
}
