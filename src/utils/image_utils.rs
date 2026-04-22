use std::path::Path;

/// Try to parse image dimensions from file header.
/// Supports PNG, JPEG, GIF, WebP.
pub fn get_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    let data = std::fs::read(path).ok()?;
    if data.len() < 16 {
        return None;
    }

    // PNG: IHDR chunk at offset 16, width at 16-19, height at 20-23
    if data.starts_with(b"\x89PNG\r\n\x1a\n") && data.len() >= 24 {
        let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return Some((width, height));
    }

    // GIF87a / GIF89a
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        if data.len() >= 10 {
            let width = u16::from_le_bytes([data[6], data[7]]) as u32;
            let height = u16::from_le_bytes([data[8], data[9]]) as u32;
            return Some((width, height));
        }
    }

    // WebP: VP8X at offset 12, width/height at 24-30 (little endian, 24-bit)
    if data.starts_with(b"RIFF") && data.len() >= 30 && &data[8..12] == b"WEBP" {
        if &data[12..16] == b"VP8X" {
            let width =
                u32::from_le_bytes([data[24], data[25], data[26], 0]) + 1;
            let height =
                u32::from_le_bytes([data[27], data[28], data[29], 0]) + 1;
            return Some((width, height));
        }
        // VP8 (lossy) at offset 26
        if &data[12..15] == b"VP8" && data.len() >= 30 {
            let width = u16::from_le_bytes([data[26], data[27]]) & 0x3FFF;
            let height = u16::from_le_bytes([data[28], data[29]]) & 0x3FFF;
            return Some((width as u32, height as u32));
        }
    }

    // JPEG: scan SOF markers
    if data.starts_with(b"\xff\xd8") {
        let mut i = 2;
        while i + 9 < data.len() {
            if data[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = data[i + 1];
            // SOF0, SOF1, SOF2, SOF3, SOF5..SOF15, SOF17 (progressive)
            if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
                let height = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
                let width = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
                return Some((width, height));
            }
            // Skip segment
            if marker == 0xD9 {
                break; // EOI
            }
            if marker == 0xD8 {
                i += 2;
                continue;
            }
            if i + 3 < data.len() {
                let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                i += 2 + len;
            } else {
                break;
            }
        }
    }

    None
}

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
