use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

type RenderedImage = (usize, String, u32, u32, u64);

#[derive(Debug, Clone)]
pub struct SlideContent {
    pub index: usize,
    pub images: Vec<(String, u32, u32, u64)>,
    pub text: String,
}

static SOFFICE_CANDIDATES: &[&str] = &["soffice", "libreoffice", "openoffice"];

#[cfg(windows)]
static EMBEDDED_PDFIUM: &[u8] = include_bytes!("../../assets/pdfium/pdfium.dll.zst");
#[cfg(target_os = "linux")]
static EMBEDDED_PDFIUM: &[u8] = include_bytes!("../../assets/pdfium/libpdfium.so.zst");
#[cfg(target_os = "macos")]
static EMBEDDED_PDFIUM: &[u8] = include_bytes!("../../assets/pdfium/libpdfium.dylib.zst");
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
static EMBEDDED_PDFIUM: &[u8] = &[];

static PDFIUM_INSTANCE: OnceLock<Option<pdfium_render::prelude::Pdfium>> = OnceLock::new();

fn get_pdfium() -> Option<&'static pdfium_render::prelude::Pdfium> {
    PDFIUM_INSTANCE.get_or_init(|| {
        if EMBEDDED_PDFIUM.is_empty() {
            return None;
        }

        // Decompress embedded zstd data
        let decompressed = zstd::decode_all(EMBEDDED_PDFIUM).ok()?;

        let cache_dir = std::env::temp_dir().join("rust-mcp-server");
        let _ = std::fs::create_dir_all(&cache_dir);

        let lib_name = if cfg!(windows) {
            "pdfium.dll"
        } else if cfg!(target_os = "macos") {
            "libpdfium.dylib"
        } else {
            "libpdfium.so"
        };
        let lib_path = cache_dir.join(lib_name);

        // Write only if not already present or size differs
        let need_write = !lib_path.exists()
            || lib_path.metadata().map(|m| m.len() as usize).unwrap_or(0) != decompressed.len();
        if need_write {
            if std::fs::write(&lib_path, &decompressed).is_err() {
                return None;
            }
        }

        use pdfium_render::prelude::*;
        Pdfium::new(Pdfium::bind_to_library(&lib_path).ok()?).into()
    }).as_ref()
}

pub fn find_libreoffice() -> Option<String> {
    for name in SOFFICE_CANDIDATES {
        if Command::new(name)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
        {
            return Some(name.to_string());
        }
    }
    None
}

pub fn convert_old_format(
    soffice: &str,
    data: &[u8],
    extension: &str,
    target_ext: &str,
) -> Result<Vec<u8>, String> {
    let tmp_in = tempfile::Builder::new()
        .prefix("mcp_old_")
        .suffix(&format!(".{}", extension))
        .tempfile()
        .map_err(|e| format!("Failed to create temp input file: {}", e))?;
    tmp_in
        .as_file()
        .write_all(data)
        .map_err(|e| format!("Failed to write temp input: {}", e))?;

    let out_dir = tempfile::tempdir()
        .map_err(|e| format!("Failed to create temp output dir: {}", e))?;

    let output = Command::new(soffice)
        .args([
            "--headless",
            "--convert-to",
            target_ext,
            "--outdir",
        ])
        .arg(out_dir.path())
        .arg(tmp_in.path())
        .output()
        .map_err(|e| {
            format!(
                "Failed to run {}. Please install LibreOffice to read .{} files: {}",
                soffice, extension, e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{} conversion failed for .{} file: {}",
            soffice, extension, stderr
        ));
    }

    let stem = tmp_in
        .path()
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let converted = out_dir.path().join(format!("{}.{}", stem, target_ext));
    std::fs::read(&converted)
        .map_err(|e| format!("Failed to read converted file: {}", e))
}

pub fn render_pptx_to_images(
    soffice: &str,
    data: &[u8],
    _dpi: u32,
    format: &str,
    out_dir: &PathBuf,
) -> Result<Vec<RenderedImage>, String> {
    let tmp_in = tempfile::Builder::new()
        .prefix("mcp_pptx_render_")
        .suffix(".pptx")
        .tempfile()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    tmp_in
        .as_file()
        .write_all(data)
        .map_err(|e| format!("Failed to write temp PPTX: {}", e))?;

    let output = Command::new(soffice)
        .args([
            "--headless",
            "--convert-to",
            format,
            "--outdir",
        ])
        .arg(out_dir)
        .arg(tmp_in.path())
        .output()
        .map_err(|e| format!("Failed to run {} for slide rendering: {}", soffice, e))?;

    if !output.status.success() {
        return Err(format!(
            "PPTX slide rendering failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stem = tmp_in
        .path()
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("slide");

    collect_rendered_files(out_dir, stem, format)
}

fn collect_rendered_files(
    out_dir: &PathBuf,
    stem: &str,
    format: &str,
) -> Result<Vec<RenderedImage>, String> {
    let ext = format.to_lowercase();
    let mut results = Vec::new();

    for entry in std::fs::read_dir(out_dir)
        .map_err(|e| format!("Failed to read output dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let fname = entry.file_name();
        let fname_str = fname.to_string_lossy();

        if fname_str
            .to_lowercase()
            .ends_with(&format!(".{}", ext))
            && fname_str.to_lowercase().starts_with(&stem.to_lowercase())
        {
            let path = entry.path();
            let meta = entry
                .metadata()
                .map_err(|e| format!("Metadata error: {}", e))?;
            let file_bytes = meta.len();

            let (w, h) = crate::utils::image_utils::get_image_dimensions(&path).unwrap_or((0, 0));

            let index = extract_slide_index(&fname_str, stem);

            results.push((index, path.to_string_lossy().to_string(), w, h, file_bytes));
        }
    }

    results.sort_by_key(|(i, _, _, _, _)| *i);
    Ok(results)
}

fn extract_slide_index(filename: &str, stem: &str) -> usize {
    let rest = filename.strip_prefix(stem).unwrap_or(filename);
    let rest = rest.strip_prefix("_").or_else(|| rest.strip_prefix("-")).unwrap_or(rest);
    rest.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<usize>()
        .map(|n| n.saturating_sub(1))
        .unwrap_or(0)
}

/// Extract embedded images from PDF pages using pure Rust (lopdf).
/// Returns a list of (page_index, image_file_path, width, height, file_size).
/// This works without Poppler or LibreOffice, but only extracts images that are
/// embedded as Image XObjects (common in scanned PDFs and PDFs with embedded images).
pub fn extract_pdf_page_images_native(
    pdf_data: &[u8],
    fmt: &str,
    out_dir: &std::path::Path,
) -> Result<Vec<(usize, String, u32, u32, u64)>, String> {
    use lopdf::Document;
    use std::io::Cursor;

    let cursor = Cursor::new(pdf_data.to_vec());
    let doc = Document::load_from(cursor)
        .map_err(|e| format!("PDF parse error: {}", e))?;

    let pages = doc.get_pages();
    let mut results = Vec::new();

    for (page_num, page_id) in pages.iter() {
        let images = doc.get_page_images(*page_id)
            .map_err(|e| format!("Failed to get page images: {}", e))?;

        for (img_idx, img) in images.iter().enumerate() {
            let raw_data: Vec<u8> = match decode_pdf_image_data(img) {
                Ok(data) => data,
                Err(_) => continue,
            };

            if raw_data.is_empty() {
                continue;
            }

            let ext = if fmt == "jpg" || fmt == "jpeg" { "jpg" } else { "png" };
            let filename = format!("pdf_page_{:04}_img_{}.{}", page_num, img_idx, ext);
            let filepath = out_dir.join(&filename);

            // If we need PNG but got JPEG raw data, we need to convert
            let write_data = if fmt == "png" && is_jpeg_data(&raw_data) {
                match convert_jpeg_to_png_bytes(&raw_data) {
                    Ok(png) => png,
                    Err(_) => raw_data,
                }
            } else {
                raw_data
            };

            std::fs::write(&filepath, &write_data)
                .map_err(|e| format!("Failed to write image: {}", e))?;

            let metadata = std::fs::metadata(&filepath)
                .map_err(|e| format!("Failed to get metadata: {}", e))?;

            results.push((
                *page_num as usize,
                filepath.to_string_lossy().to_string(),
                img.width as u32,
                img.height as u32,
                metadata.len(),
            ));
        }
    }

    if results.is_empty() {
        return Err(
            "PDF contains no embedded Image XObjects. \
             Use Poppler (pdftoppm) or LibreOffice for full page rendering."
                .to_string(),
        );
    }

    results.sort_by_key(|(i, _, _, _, _)| *i);
    Ok(results)
}

fn decode_pdf_image_data(img: &lopdf::xobject::PdfImage<'_>) -> Result<Vec<u8>, ()> {
    use std::io::Read;

    let mut data = img.content.to_vec();

    if let Some(ref filters) = img.filters {
        for filter in filters.iter().rev() {
            match filter.to_uppercase().as_str() {
                "FLATEDECODE" | "FL" => {
                    let mut decoder = flate2::read::ZlibDecoder::new(&data[..]);
                    let mut decoded = Vec::new();
                    decoder.read_to_end(&mut decoded).map_err(|_| ())?;
                    data = decoded;
                }
                "DCTDECODE" | "DCT" => {
                    // JPEG data - keep as-is
                    break;
                }
                "ASCII85DECODE" | "A85" => {
                    let decoded = decode_ascii85(&data).ok_or(())?;
                    data = decoded;
                }
                "ASCIIHEXDECODE" | "AHX" => {
                    let decoded = decode_ascii_hex(&data).ok_or(())?;
                    data = decoded;
                }
                "CCITTFAXDECODE" | "CCF" => {
                    // CCITT fax decode is complex; skip this image
                    return Err(());
                }
                _ => {}
            }
        }
    }

    Ok(data)
}

fn is_jpeg_data(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8
}

fn convert_jpeg_to_png_bytes(jpeg_data: &[u8]) -> Result<Vec<u8>, ()> {
    let img = image::load_from_memory(jpeg_data).map_err(|_| ())?;
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).map_err(|_| ())?;
    Ok(buf.into_inner())
}

fn decode_ascii85(data: &[u8]) -> Option<Vec<u8>> {
    let mut result = Vec::new();
    let mut group = [0u8; 5];
    let mut group_idx = 0;

    for &byte in data {
        if byte == b'~' {
            break; // End-of-data marker
        }
        if byte.is_ascii_whitespace() {
            continue;
        }
        if (b'!'..=b'u').contains(&byte) {
            group[group_idx] = byte;
            group_idx += 1;
            if group_idx == 5 {
                let mut value: u32 = 0;
                for &g in &group {
                    value = value.wrapping_mul(85).wrapping_add((g - b'!') as u32);
                }
                result.extend_from_slice(&value.to_be_bytes());
                group_idx = 0;
            }
        }
    }

    if group_idx > 0 {
        for i in group_idx..5 {
            group[i] = b'u';
        }
        let mut value: u32 = 0;
        for i in 0..5 {
            value = value.wrapping_mul(85).wrapping_add((group[i] - b'!') as u32);
        }
        result.extend_from_slice(&value.to_be_bytes()[..group_idx.saturating_sub(1)]);
    }

    Some(result)
}

fn decode_ascii_hex(data: &[u8]) -> Option<Vec<u8>> {
    let mut result = Vec::new();
    let mut hi_nibble: Option<u8> = None;

    for &byte in data {
        if byte == b'>' {
            break;
        }
        if byte.is_ascii_whitespace() {
            continue;
        }
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'A'..=b'F' => byte - b'A' + 10,
            b'a'..=b'f' => byte - b'a' + 10,
            _ => continue,
        };
        if let Some(hi) = hi_nibble {
            result.push((hi << 4) | nibble);
            hi_nibble = None;
        } else {
            hi_nibble = Some(nibble);
        }
    }

    Some(result)
}

/// Render PDF pages to images using PDFium (Chromium's PDF engine).
/// Returns list of (page_index, image_file_path, width, height, file_size).
/// Requires pdfium.dll (Windows) / libpdfium.so (Linux) in the executable directory or system path.
pub fn render_pdf_pages_native(
    data: &[u8],
    fmt: &str,
    out_dir: &std::path::Path,
) -> Result<Vec<(usize, String, u32, u32, u64)>, String> {
    use pdfium_render::prelude::*;

    let pdfium = get_pdfium()
        .ok_or_else(|| "PDFium library not available. Place pdfium.dll in assets/ directory.".to_string())?;
    let tmp_file = tempfile::Builder::new()
        .prefix("mcp_pdf_render_")
        .suffix(".pdf")
        .tempfile()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    let tmp_path = tmp_file.path().to_path_buf();
    std::fs::write(&tmp_path, data)
        .map_err(|e| format!("Failed to write temp PDF: {}", e))?;

    let document = pdfium
        .load_pdf_from_file(&tmp_path, None)
        .map_err(|e| format!("Failed to load PDF: {}", e))?;

    let render_config = PdfRenderConfig::new()
        .set_target_width(2000)
        .set_maximum_height(2000);

    let mut results = Vec::new();
    for (index, page) in document.pages().iter().enumerate() {
        let image = page
            .render_with_config(&render_config)
            .map_err(|e| format!("Failed to render page {}: {}", index + 1, e))?
            .as_image()
            .map_err(|e| format!("Failed to convert page {} to image: {}", index + 1, e))?
            .into_rgb8();

        let (w, h) = image.dimensions();
        let ext = if fmt == "jpg" || fmt == "jpeg" { "jpg" } else { "png" };
        let filename = format!("pdf_page_{:04}.{}", index, ext);
        let filepath = out_dir.join(&filename);

        if ext == "jpg" {
            image
                .save_with_format(&filepath, image::ImageFormat::Jpeg)
                .map_err(|e| format!("Failed to save page {}: {}", index + 1, e))?;
        } else {
            image
                .save_with_format(&filepath, image::ImageFormat::Png)
                .map_err(|e| format!("Failed to save page {}: {}", index + 1, e))?;
        }

        let metadata = std::fs::metadata(&filepath)
            .map_err(|e| format!("Failed to get metadata: {}", e))?;

        results.push((index, filepath.to_string_lossy().to_string(), w, h, metadata.len()));
    }

    Ok(results)
}

pub fn extract_pptx_images_text_native(
    data: &[u8],
    out_dir: &Path,
) -> Result<Vec<SlideContent>, String> {
    use std::io::Cursor;

    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to open PPTX as ZIP: {}", e))?;

    let mut slide_files: Vec<String> = Vec::new();

    if let Ok(mut file) = archive.by_name("ppt/_rels/presentation.xml.rels") {
        let mut xml = String::new();
        file.read_to_string(&mut xml).ok();
        let rels = parse_relationship_targets(&xml, "slide");
        for target in &rels {
            let full_target = if target.starts_with("slides/") {
                format!("ppt/{}", target)
            } else {
                target.clone()
            };
            slide_files.push(full_target);
        }
    }

    if slide_files.is_empty() {
        return Err("PPTX contains no slides".to_string());
    }

    let mut tmp_file = tempfile::Builder::new()
        .prefix("mcp_pptx_native_")
        .suffix(".pptx")
        .tempfile()
        .map_err(|e| format!("Failed to create temp file for PPTX: {}", e))?;
    tmp_file
        .write_all(data)
        .map_err(|e| format!("Failed to write temp PPTX file: {}", e))?;
    let tmp_path = tmp_file.path().to_path_buf();

    let mut slides_content: Vec<SlideContent> = Vec::new();

    for (slide_idx, _slide_file) in slide_files.iter().enumerate() {
        let slide_num = slide_idx + 1;
        let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_num);

        let mut image_refs: Vec<String> = Vec::new();
        if let Ok(mut file) = archive.by_name(&rels_path) {
            let mut xml = String::new();
            file.read_to_string(&mut xml).ok();
            image_refs = parse_relationship_targets(&xml, "image");
        }

        let mut images: Vec<(String, u32, u32, u64)> = Vec::new();
        for img_ref in &image_refs {
            let img_path_in_zip = if img_ref.starts_with("../media/") {
                format!("ppt/{}", &img_ref[3..])
            } else if img_ref.starts_with("media/") {
                format!("ppt/{}", img_ref)
            } else if !img_ref.starts_with("ppt/") {
                let fname = std::path::Path::new(img_ref)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(img_ref);
                format!("ppt/media/{}", fname)
            } else {
                img_ref.clone()
            };

            if let Ok(mut file) = archive.by_name(&img_path_in_zip) {
                let mut img_data = Vec::new();
                if file.read_to_end(&mut img_data).is_ok() {
                    let hash_bytes = {
                        use sha2::{Digest, Sha256};
                        let mut h = Sha256::new();
                        h.update(&img_data);
                        h.finalize()
                    };
                    let hash_str = format!("{:x}", hash_bytes);
                    let ext = std::path::Path::new(&img_path_in_zip)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("png");
                    let filename = format!("pptx_s{:04}_{}.{}", slide_idx, &hash_str[..8], ext);
                    let filepath = out_dir.join(&filename);
                    std::fs::write(&filepath, &img_data)
                        .map_err(|e| format!("Failed to write image: {}", e))?;
                    let (w, h) = crate::utils::image_utils::get_image_dimensions(&filepath).unwrap_or((0, 0));
                    let size = filepath.metadata().map(|m| m.len()).unwrap_or(0);
                    images.push((filepath.to_string_lossy().to_string(), w, h, size));
                }
            }
        }

        let text = extract_pptx_slide_text(&tmp_path, slide_idx)?;

        slides_content.push(SlideContent {
            index: slide_idx,
            images,
            text,
        });
    }

    Ok(slides_content)
}

fn parse_relationship_targets(xml: &str, target_type: &str) -> Vec<String> {
    let type_attr = format!("relationships/{}", target_type);
    let rel_re = regex::Regex::new(r#"<Relationship\s+([^>]*?)\s*/?>"#).unwrap();
    let attr_re = regex::Regex::new(r#"(\w+)="([^"]*)""#).unwrap();
    let mut targets = Vec::new();

    for cap in rel_re.captures_iter(xml) {
        let attrs_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let mut target_val = None;
        let mut type_val = None;
        for attr_cap in attr_re.captures_iter(attrs_str) {
            let name = attr_cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let value = attr_cap.get(2).map(|m| m.as_str()).unwrap_or("");
            match name {
                "Target" => target_val = Some(value),
                "Type" => type_val = Some(value),
                _ => {}
            }
        }
        if let (Some(target), Some(typ)) = (target_val, type_val) {
            if typ.contains(&type_attr) {
                targets.push(target.to_string());
            }
        }
    }
    targets
}

fn extract_pptx_slide_text(path: &Path, slide_index: usize) -> Result<String, String> {
    use ppt_rs::oxml::presentation::PresentationReader;
    let path_str = path.to_string_lossy();
    let reader = PresentationReader::open(path_str.as_ref())
        .map_err(|e| format!("PPTX open error: {}", e))?;
    let slides = reader
        .get_all_slides()
        .map_err(|e| format!("PPTX slide read error: {}", e))?;
    if slide_index >= slides.len() {
        return Ok(String::new());
    }
    let slide = &slides[slide_index];
        let all_texts = slide.all_text();
        let mut text = String::new();
        for t in &all_texts {
            let trimmed = t.trim();
            if !trimmed.is_empty() {
                text.push_str(trimmed);
                text.push('\n');
            }
        }
        if !slide.tables.is_empty() {
            for table in &slide.tables {
                text.push('\n');
                let rows = &table.rows;
                if rows.is_empty() {
                    continue;
                }
                let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
                for (ri, row) in rows.iter().enumerate() {
                    text.push('|');
                    for ci in 0..col_count {
                        text.push(' ');
                        text.push_str(if let Some(cell) = row.get(ci) {
                            &cell.text
                        } else {
                            ""
                        });
                        text.push_str(" |");
                    }
                    text.push('\n');
                    if ri == 0 {
                        text.push('|');
                        for _ in 0..col_count {
                            text.push_str(" --- |");
                        }
                        text.push('\n');
                    }
                }
                text.push('\n');
            }
        }
        Ok(text.trim().to_string())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OfficeDocStats {
    pub document_type: String,
    pub slide_count: Option<usize>,
    pub page_count: Option<usize>,
    pub image_count: Option<usize>,
    pub text_char_count: Option<usize>,
    pub sheet_count: Option<usize>,
}

pub fn get_office_document_stats(
    data: &[u8],
    extension: &str,
) -> OfficeDocStats {
    let fmt = crate::utils::office_utils::detect_office_format(extension);
    match fmt {
        crate::utils::office_utils::OfficeFormat::Docx | crate::utils::office_utils::OfficeFormat::Doc => {
            let image_count = count_docx_images(data);
            let text = crate::utils::office_utils::extract_text_from_bytes(data, fmt, None).unwrap_or_default();
            OfficeDocStats {
                document_type: "docx".to_string(),
                slide_count: None,
                page_count: None,
                image_count: Some(image_count),
                text_char_count: Some(text.chars().count()),
                sheet_count: None,
            }
        }
        crate::utils::office_utils::OfficeFormat::Pptx | crate::utils::office_utils::OfficeFormat::Ppt => {
            let (slide_count, image_count) = count_pptx_stats(data);
            let text = crate::utils::office_utils::extract_text_from_bytes(data, fmt, None).unwrap_or_default();
            OfficeDocStats {
                document_type: "pptx".to_string(),
                slide_count: Some(slide_count),
                page_count: None,
                image_count: Some(image_count),
                text_char_count: Some(text.chars().count()),
                sheet_count: None,
            }
        }
        crate::utils::office_utils::OfficeFormat::Pdf => {
            let page_count = count_pdf_pages(data);
            let text = crate::utils::office_utils::extract_text_from_bytes(data, fmt, None).unwrap_or_default();
            let image_count = count_pdf_images(data);
            OfficeDocStats {
                document_type: "pdf".to_string(),
                slide_count: None,
                page_count: Some(page_count),
                image_count: Some(image_count),
                text_char_count: Some(text.chars().count()),
                sheet_count: None,
            }
        }
        crate::utils::office_utils::OfficeFormat::Xlsx | crate::utils::office_utils::OfficeFormat::Xls => {
            let sheet_count = count_xlsx_sheets(data);
            let text = crate::utils::office_utils::extract_text_from_bytes(data, fmt, None).unwrap_or_default();
            OfficeDocStats {
                document_type: "xlsx".to_string(),
                slide_count: None,
                page_count: None,
                image_count: None,
                text_char_count: Some(text.chars().count()),
                sheet_count: Some(sheet_count),
            }
        }
        _ => OfficeDocStats {
            document_type: "unknown".to_string(),
            slide_count: None,
            page_count: None,
            image_count: None,
            text_char_count: None,
            sheet_count: None,
        },
    }
}

fn count_docx_images(data: &[u8]) -> usize {
    match docx_rs::read_docx(data) {
        Ok(docx) => docx.images.len(),
        Err(_) => 0,
    }
}

fn count_pptx_stats(data: &[u8]) -> (usize, usize) {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(_) => return (0, 0),
    };

    let mut slide_count = 0;
    if let Ok(mut file) = archive.by_name("ppt/_rels/presentation.xml.rels") {
        let mut xml = String::new();
        if file.read_to_string(&mut xml).is_ok() {
            slide_count = parse_relationship_targets(&xml, "slide").len();
        }
    }

    let mut image_count: usize = 0;
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            if entry.name().starts_with("ppt/media/") {
                image_count += 1;
            }
        }
    }

    (slide_count, image_count)
}

fn count_pdf_pages(data: &[u8]) -> usize {
    use std::io::Cursor;
    let cursor = Cursor::new(data.to_vec());
    match lopdf::Document::load_from(cursor) {
        Ok(doc) => doc.get_pages().len(),
        Err(_) => 0,
    }
}

fn count_pdf_images(data: &[u8]) -> usize {
    use std::io::Cursor;
    let cursor = Cursor::new(data.to_vec());
    let doc = match lopdf::Document::load_from(cursor) {
        Ok(d) => d,
        Err(_) => return 0,
    };
    let pages = doc.get_pages();
    let mut total = 0usize;
    for (_, page_id) in pages.iter() {
        if let Ok(images) = doc.get_page_images(*page_id) {
            total += images.len();
        }
    }
    total
}

fn count_xlsx_sheets(data: &[u8]) -> usize {
    use calamine::Reader;
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    match calamine::open_workbook_auto_from_rs(cursor) {
        Ok(wb) => wb.sheet_names().len(),
        Err(_) => 0,
    }
}
