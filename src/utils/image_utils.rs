use std::path::Path;

/// Get MIME type for an image file based on its extension
pub fn get_image_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => match ext.to_lowercase().as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "ico" => "image/x-icon",
            "tiff" | "tif" => "image/tiff",
            "avif" => "image/avif",
            _ => "application/octet-stream",
        },
        None => "application/octet-stream",
    }
}

/// Check if a file is an image based on its extension
pub fn is_image_file(path: &Path) -> bool {
    const IMAGE_EXTENSIONS: &[&str] = &[
        "png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "ico", "tiff", "tif", "avif",
    ];

    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()),
        None => false,
    }
}

/// Read image file and encode to base64
pub fn read_image_base64(path: &Path) -> Result<(String, String), String> {
    use base64::Engine;

    let data = std::fs::read(path).map_err(|e| format!("Failed to read image: {}", e))?;
    let mime_type = get_image_mime_type(path);
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&data);

    Ok((base64_data, mime_type.to_string()))
}

/// Get image dimensions if available (for common formats)
pub fn get_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    // This is a simplified version - in production you might want to use an image crate
    // For now, we'll return None as we don't want to add heavy dependencies
    let _ = path;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_image_mime_type() {
        assert_eq!(
            get_image_mime_type(Path::new("test.png")),
            "image/png"
        );
        assert_eq!(
            get_image_mime_type(Path::new("test.jpg")),
            "image/jpeg"
        );
        assert_eq!(
            get_image_mime_type(Path::new("test.jpeg")),
            "image/jpeg"
        );
        assert_eq!(
            get_image_mime_type(Path::new("test.gif")),
            "image/gif"
        );
        assert_eq!(
            get_image_mime_type(Path::new("test.svg")),
            "image/svg+xml"
        );
        assert_eq!(
            get_image_mime_type(Path::new("test.unknown")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file(Path::new("test.png")));
        assert!(is_image_file(Path::new("test.jpg")));
        assert!(is_image_file(Path::new("test.JPG")));
        assert!(is_image_file(Path::new("test.webp")));
        assert!(!is_image_file(Path::new("test.txt")));
        assert!(!is_image_file(Path::new("test.exe")));
        assert!(!is_image_file(Path::new("test")));
    }
}
