use crate::utils::file_utils::{
    get_file_extension, is_text_file, read_file_with_options, resolve_path,
    strip_unc_prefix,
};
use crate::utils::image_utils::{get_image_dimensions, get_image_mime_type};
use crate::utils::office_converter as converter;
use crate::utils::office_utils::{
    detect_office_format, extract_docx_images, extract_docx_markdown,
    extract_docx_with_images, extract_text_from_bytes, ImageMeta, OfficeFormat,
};
use base64::Engine;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico", "tiff", "tif",
];

static SOFFICE_PATH: OnceLock<Option<String>> = OnceLock::new();

fn get_soffice() -> Option<&'static str> {
    SOFFICE_PATH
        .get_or_init(converter::find_libreoffice)
        .as_deref()
}

fn temp_image_dir() -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("mcp_read_images");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadParams {
    #[schemars(description = "The file path to read. Supports text files, PDF, DOC/DOCX, PPT/PPTX, XLS/XLSX, IPYNB, and images.")]
    pub path: String,

    #[schemars(description = "Reading mode. Default: auto.\n- auto: auto-detect best mode for file type\n- text: force plain text with line numbers/offset\n- media: return base64 image content for vision models\n\nFor DOC/DOCX:\n- doc_text: markdown with headings, tables, bold/italic/strikethrough. Best for text-heavy docs.\n- doc_with_images: markdown with images embedded inline at positions. Best when doc has text+images.\n- doc_images: extracted images only (no text). Best for image-heavy docs.\n\nFor PPT/PPTX:\n- ppt_text: slide content as markdown (titles, bullets, tables). No LibreOffice required.\n- ppt_images: slides as images. Uses LibreOffice (best quality) if installed; otherwise native extraction (embedded images + text per slide, pure Rust).\n\nFor PDF:\n- pdf_text: extracted text content via lopdf.\n- pdf_images: each page rendered to image via PDFium (embedded in binary). Best for scanned PDFs.\n\nFor XLS/XLSX/IPYNB: text mode extracts tabular/cell data.\n\nStrategy: use FileStat first to check document stats (slide_count, image_count, text_char_count), then choose optimal mode.")]
    pub mode: Option<String>,

    #[schemars(description = "Starting line number (0-indexed, default: 0)")]
    pub start_line: Option<usize>,
    #[schemars(description = "Ending line number (exclusive, default: 500)")]
    pub end_line: Option<usize>,
    #[schemars(description = "Character offset to start reading from (alternative to start_line)")]
    pub offset_chars: Option<usize>,
    #[schemars(description = "Maximum characters to return (default: 15000)")]
    pub max_chars: Option<usize>,
    #[schemars(description = "Show line numbers in output (default: false)")]
    pub line_numbers: Option<bool>,
    #[schemars(description = "Line number to highlight (0-indexed)")]
    pub highlight_line: Option<usize>,

    #[schemars(description = "XLS/XLSX: Specific sheet name to read.")]
    pub sheet_name: Option<String>,

    #[schemars(description = "DPI for slide/page image rendering, default 150.")]
    pub image_dpi: Option<u32>,

    #[schemars(description = "Image output format: png (default) or jpg.")]
    pub image_format: Option<String>,

    #[schemars(description = "Batch mode: list of file paths to read concurrently.")]
    pub paths: Option<Vec<ReadItem>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadItem {
    #[schemars(description = "The file path to read")]
    pub path: String,
    #[schemars(description = "Reading mode (same options as ReadParams.mode)")]
    pub mode: Option<String>,
    #[schemars(description = "Starting line number (0-indexed, default: 0)")]
    pub start_line: Option<usize>,
    #[schemars(description = "Ending line number (exclusive, default: 500)")]
    pub end_line: Option<usize>,
    #[schemars(description = "Character offset to start reading from (alternative to start_line)")]
    pub offset_chars: Option<usize>,
    #[schemars(description = "Maximum characters to return (default: 15000)")]
    pub max_chars: Option<usize>,
    #[schemars(description = "Show line numbers in output (default: false)")]
    pub line_numbers: Option<bool>,
    #[schemars(description = "Line number to highlight (0-indexed)")]
    pub highlight_line: Option<usize>,
    #[schemars(description = "XLS/XLSX: Specific sheet name to read")]
    pub sheet_name: Option<String>,
    #[schemars(description = "DPI for slide/page image rendering, default 150")]
    pub image_dpi: Option<u32>,
    #[schemars(description = "Image output format: png (default) or jpg")]
    pub image_format: Option<String>,
}

#[derive(Debug, Serialize)]
struct ImageMetaResponse {
    index: usize,
    path: String,
    filename: String,
    mime_type: String,
    width: u32,
    height: u32,
    size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl From<&ImageMeta> for ImageMetaResponse {
    fn from(meta: &ImageMeta) -> Self {
        Self {
            index: meta.index,
            path: meta.path.clone(),
            filename: meta.filename.clone(),
            mime_type: meta.mime_type.clone(),
            width: meta.width,
            height: meta.height,
            size_bytes: meta.size_bytes,
            description: meta.description.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ReadResult {
    path: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<ImageMetaResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines_displayed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_count: Option<usize>,
}

pub async fn file_read(
    params: Parameters<ReadParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;

    if let Some(ref paths) = params.paths {
        if !paths.is_empty() {
            let mut futures = Vec::new();
            for item in paths {
                futures.push(read_single(
                    &item.path,
                    item.mode.as_deref(),
                    item.start_line,
                    item.end_line,
                    item.offset_chars,
                    item.max_chars,
                    item.line_numbers,
                    item.highlight_line,
                    item.sheet_name.as_deref(),
                    item.image_dpi,
                    item.image_format.as_deref(),
                    working_dir,
                ));
            }
            let results = futures::future::join_all(futures).await;
            let mut all_contents: Vec<rmcp::model::Content> = Vec::new();
            for result in results {
                match result {
                    Ok(contents) => all_contents.extend(contents),
                    Err(e) => {
                        all_contents.push(rmcp::model::Content::text(format!("{{\"error\": \"{}\"}}", e)));
                    }
                }
            }
            return Ok(CallToolResult::success(all_contents));
        }
    }

    let contents = read_single(
        &params.path,
        params.mode.as_deref(),
        params.start_line,
        params.end_line,
        params.offset_chars,
        params.max_chars,
        params.line_numbers,
        params.highlight_line,
        params.sheet_name.as_deref(),
        params.image_dpi,
        params.image_format.as_deref(),
        working_dir,
    )
    .await?;

    Ok(CallToolResult::success(contents))
}

#[allow(clippy::too_many_arguments)]
async fn read_single(
    path_str: &str,
    mode: Option<&str>,
    start_line: Option<usize>,
    end_line: Option<usize>,
    offset_chars: Option<usize>,
    max_chars: Option<usize>,
    line_numbers: Option<bool>,
    highlight_line: Option<usize>,
    sheet_name: Option<&str>,
    image_dpi: Option<u32>,
    image_format: Option<&str>,
    working_dir: &Path,
) -> Result<Vec<rmcp::model::Content>, String> {
    let path = Path::new(path_str);
    let canonical_path = match resolve_path(path, working_dir) {
        Ok(p) => p,
        Err(e) => {
            let r = err_result(path_str, e);
            return Ok(vec![rmcp::model::Content::text(serde_json::to_string_pretty(&r).unwrap_or_default())]);
        }
    };

    if !canonical_path.exists() {
        let r = err_result(path_str, format!("File '{}' does not exist", path_str));
        return Ok(vec![rmcp::model::Content::text(serde_json::to_string_pretty(&r).unwrap_or_default())]);
    }
    if !canonical_path.is_file() {
        let r = err_result(path_str, format!("Path '{}' is not a file", path_str));
        return Ok(vec![rmcp::model::Content::text(serde_json::to_string_pretty(&r).unwrap_or_default())]);
    }

    let path_ref: &Path = &canonical_path;
    let ext = get_file_extension(path_ref).unwrap_or_default();
    let mode = mode.unwrap_or("auto");

    let is_img = IMAGE_EXTENSIONS.contains(&ext.as_str());

    // --- mode: media ---
    if mode == "media" || (mode == "auto" && is_img) {
        return read_media_mode(path_ref, path_str).await;
    }

    // --- mode: text ---
    if mode == "text" {
        return read_text_mode(path_ref, path_str, start_line, end_line, offset_chars, max_chars, line_numbers, highlight_line).await;
    }

    // --- PDF modes ---
    if ext == "pdf" {
        return match mode {
            "auto" | "pdf_text" => read_pdf_text(path_ref, path_str).await,
            "pdf_images" => read_pdf_images(path_ref, path_str, image_dpi, image_format).await,
            _ => {
                let r = err_result(path_str, format!("Invalid mode '{}' for PDF files", mode));
                Ok(vec![rmcp::model::Content::text(serde_json::to_string_pretty(&r).unwrap_or_default())])
            }
        };
    }

    // --- PPTX/PPT modes ---
    if ext == "pptx" || ext == "ppt" {
        return match mode {
            "auto" | "ppt_text" => read_pptx_text(path_ref, path_str).await,
            "ppt_images" => read_pptx_images(path_ref, path_str, image_dpi, image_format).await,
            _ => {
                let r = err_result(path_str, format!("Invalid mode '{}' for PPT/PPTX files", mode));
                Ok(vec![rmcp::model::Content::text(serde_json::to_string_pretty(&r).unwrap_or_default())])
            }
        };
    }

    // --- DOCX/DOC modes ---
    if ext == "docx" || ext == "doc" {
        return match mode {
            "auto" | "doc_text" => read_docx_text_mode(path_ref, path_str).await,
            "doc_with_images" => read_docx_with_images_mode(path_ref, path_str).await,
            "doc_images" => read_docx_images_mode(path_ref, path_str).await,
            _ => {
                let r = err_result(path_str, format!("Invalid mode '{}' for DOC/DOCX files", mode));
                Ok(vec![rmcp::model::Content::text(serde_json::to_string_pretty(&r).unwrap_or_default())])
            }
        };
    }

    // --- XLSX/XLS ---
    let office_fmt = detect_office_format(&ext);
    if matches!(office_fmt, OfficeFormat::Xlsx | OfficeFormat::Xls) {
        return read_office_format(path_ref, path_str, office_fmt, sheet_name).await;
    }

    // --- IPYNB ---
    if matches!(office_fmt, OfficeFormat::Ipynb) {
        return read_office_format(path_ref, path_str, office_fmt, sheet_name).await;
    }

    // --- Fallback: text ---
    if is_text_file(path_ref) {
        return read_text_mode(path_ref, path_str, start_line, end_line, offset_chars, max_chars, line_numbers, highlight_line).await;
    }

    let r = err_result(
        path_str,
        format!(
            "Cannot read binary file '{}'. Use mode=\"media\" for supported formats.",
            path_str
        ),
    );
    Ok(vec![rmcp::model::Content::text(serde_json::to_string_pretty(&r).unwrap_or_default())])
}

// ---- Mode handlers ----

async fn read_media_mode(path_ref: &Path, original_path: &str) -> Result<Vec<rmcp::model::Content>, String> {
    let mime = get_image_mime_type(path_ref).to_string();
    let path_buf = path_ref.to_path_buf();
    let dims = tokio::task::spawn_blocking(move || get_image_dimensions(&path_buf))
        .await
        .unwrap_or(None);
    let meta = tokio::fs::metadata(path_ref).await;
    let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let filename = path_ref
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let data = tokio::fs::read(path_ref)
        .await
        .map_err(|e| format!("Failed to read file '{}': {}", original_path, e))?;

    let base64_data = base64::engine::general_purpose::STANDARD.encode(&data);
    let canonical_display = strip_unc_prefix(&path_ref.to_string_lossy());

    let meta_text = format!(
        "{{\"path\": \"{}\", \"filename\": \"{}\", \"mime_type\": \"{}\", \"size_bytes\": {}, \"width\": {}, \"height\": {}}}",
        canonical_display,
        filename,
        mime,
        size_bytes,
        dims.map(|d| d.0.to_string()).unwrap_or_else(|| "null".to_string()),
        dims.map(|d| d.1.to_string()).unwrap_or_else(|| "null".to_string()),
    );

    Ok(vec![
        rmcp::model::Content::image(base64_data, mime),
        rmcp::model::Content::text(meta_text),
    ])
}

async fn read_pdf_text(path_ref: &Path, _original_path: &str) -> Result<Vec<rmcp::model::Content>, String> {
    let canonical_display = strip_unc_prefix(&path_ref.to_string_lossy());
    let data = tokio::fs::read(path_ref).await
        .map_err(|e| format!("Failed to read PDF file '{}': {}", _original_path, e))?;
    let page_count = estimate_pdf_page_count(&data);
    match extract_text_from_bytes(&data, OfficeFormat::Pdf, None) {
        Ok(text) => {
            let result = ReadResult {
                path: canonical_display,
                success: true,
                error: None,
                content: Some(text),
                images: None,
                lines_displayed: None,
                total_lines: None,
                truncated: None,
                format: Some("pdf_text".to_string()),
                page_count,
            };
            let json = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
            Ok(vec![rmcp::model::Content::text(json)])
        }
        Err(e) => {
            let result = ReadResult {
                path: canonical_display,
                success: false,
                error: Some(format!("PDF text extraction failed: {}", e)),
                content: None,
                images: None,
                lines_displayed: None,
                total_lines: None,
                truncated: None,
                format: Some("pdf_text".to_string()),
                page_count: None,
            };
            let json = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
            Ok(vec![rmcp::model::Content::text(json)])
        }
    }
}

async fn read_pdf_images(
    path_ref: &Path,
    _original_path: &str,
    image_dpi: Option<u32>,
    image_format: Option<&str>,
) -> Result<Vec<rmcp::model::Content>, String> {
    let canonical_display = strip_unc_prefix(&path_ref.to_string_lossy());
    let _dpi = image_dpi.unwrap_or(150);
    let fmt = image_format.unwrap_or("png");

    let data = tokio::fs::read(path_ref).await
        .map_err(|e| {
            let r = err_result(_original_path, format!("Failed to read PDF file: {}", e));
            serde_json::to_string_pretty(&r).unwrap_or_default()
        })?;

    let out_dir = temp_image_dir();
    let hash = format!("{:x}", {
        let mut h = Sha256::new();
        h.update(&data);
        h.finalize()
    });
    let out_dir = out_dir.join(format!("pdf_{}", &hash[..12]));
    let _ = std::fs::create_dir_all(&out_dir);

    let rendered = converter::render_pdf_pages_native(&data, fmt, &out_dir)
        .or_else(|_| converter::extract_pdf_page_images_native(&data, fmt, &out_dir));

    match rendered {
        Ok(pages) => {
            let image_count = pages.len();
            let mut contents: Vec<rmcp::model::Content> = Vec::new();
            let mime = if fmt == "jpg" || fmt == "jpeg" { "image/jpeg" } else { "image/png" };
            for (idx, (_, img_path, _w, _h, _size)) in pages.iter().enumerate() {
                match tokio::fs::read(img_path).await {
                    Ok(img_data) => {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&img_data);
                        contents.push(rmcp::model::Content::image(b64, mime.to_string()));
                    }
                    Err(e) => {
                        contents.push(rmcp::model::Content::text(format!("[Image {} read error: {}]", idx + 1, e)));
                    }
                }
            }
            let meta = serde_json::json!({
                "path": canonical_display,
                "success": true,
                "format": "pdf_images",
                "image_count": image_count,
                "image_format": fmt,
                "note": "PDF pages rendered to images using PDFium"
            });
            contents.push(rmcp::model::Content::text(meta.to_string()));
            Ok(contents)
        }
        Err(e) => {
            let r = err_result(_original_path, e);
            let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
            Ok(vec![rmcp::model::Content::text(json)])
        }
    }
}

async fn read_pptx_text(path_ref: &Path, _original_path: &str) -> Result<Vec<rmcp::model::Content>, String> {
    let canonical_display = strip_unc_prefix(&path_ref.to_string_lossy());
    let ext = get_file_extension(path_ref).unwrap_or_default();

    let data = match tokio::fs::read(path_ref).await {
        Ok(d) => d,
        Err(e) => {
            let r = err_result(_original_path, format!("Failed to read PPTX file: {}", e));
            let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
            return Ok(vec![rmcp::model::Content::text(json)]);
        }
    };

    if ext == "ppt" {
        let soffice = match get_soffice() {
            Some(s) => s,
            None => {
                let r = err_result(_original_path,
                    "LibreOffice is required to read old .ppt files. Please install LibreOffice.".to_string());
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                return Ok(vec![rmcp::model::Content::text(json)]);
            }
        };
        match converter::convert_old_format(soffice, &data, "ppt", "pptx") {
            Ok(pptx_data) => {
                match extract_text_from_bytes(&pptx_data, OfficeFormat::Pptx, None) {
                    Ok(text) => {
                        let r = text_result(canonical_display, text, "ppt_text");
                        let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                        Ok(vec![rmcp::model::Content::text(json)])
                    }
                    Err(e) => {
                        let r = err_result(_original_path, format!("PPT text extraction failed: {}", e));
                        let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                        Ok(vec![rmcp::model::Content::text(json)])
                    }
                }
            }
            Err(e) => {
                let r = err_result(_original_path, e);
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                Ok(vec![rmcp::model::Content::text(json)])
            }
        }
    } else {
        match extract_text_from_bytes(&data, OfficeFormat::Pptx, None) {
            Ok(text) => {
                let r = text_result(canonical_display, text, "ppt_text");
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                Ok(vec![rmcp::model::Content::text(json)])
            }
            Err(e) => {
                let r = err_result(_original_path, format!("PPTX text extraction failed: {}", e));
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                Ok(vec![rmcp::model::Content::text(json)])
            }
        }
    }
}

async fn read_pptx_images(
    path_ref: &Path,
    _original_path: &str,
    image_dpi: Option<u32>,
    image_format: Option<&str>,
) -> Result<Vec<rmcp::model::Content>, String> {
    let canonical_display = strip_unc_prefix(&path_ref.to_string_lossy());
    let _dpi = image_dpi.unwrap_or(150);
    let fmt = image_format.unwrap_or("png");
    let ext = get_file_extension(path_ref).unwrap_or_default();

    let data = match tokio::fs::read(path_ref).await {
        Ok(d) => d,
        Err(e) => {
            let r = err_result(_original_path, format!("Failed to read file: {}", e));
            let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
            return Ok(vec![rmcp::model::Content::text(json)]);
        }
    };

    let pptx_data = if ext == "ppt" {
        let soffice = match get_soffice() {
            Some(s) => s,
            None => {
                let r = err_result(_original_path,
                    "LibreOffice is required to read old .ppt files. Please install LibreOffice.".to_string());
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                return Ok(vec![rmcp::model::Content::text(json)]);
            }
        };
        match converter::convert_old_format(soffice, &data, "ppt", "pptx") {
            Ok(d) => d,
            Err(e) => {
                let r = err_result(_original_path, e);
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                return Ok(vec![rmcp::model::Content::text(json)]);
            }
        }
    } else {
        data
    };

    let out_dir = temp_image_dir();
    let hash = format!("{:x}", sha2::Sha256::digest(&pptx_data));
    let out_dir = out_dir.join(format!("pptx_{}", &hash[..12]));
    let _ = std::fs::create_dir_all(&out_dir);

    let soffice_available = get_soffice().is_some();

    if soffice_available {
        // Best quality: full slide rendering via LibreOffice
        let soffice = get_soffice().unwrap();
        match converter::render_pptx_to_images(soffice, &pptx_data, _dpi, fmt, &out_dir) {
            Ok(slides) => {
                let slide_count = slides.len();
                let mut contents: Vec<rmcp::model::Content> = Vec::new();
                let mime = if fmt == "jpg" || fmt == "jpeg" { "image/jpeg" } else { "image/png" };
                for (idx, (_, img_path, _w, _h, _size)) in slides.iter().enumerate() {
                    match tokio::fs::read(img_path).await {
                        Ok(img_data) => {
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&img_data);
                            contents.push(rmcp::model::Content::image(b64, mime.to_string()));
                        }
                        Err(e) => {
                            contents.push(rmcp::model::Content::text(format!("[Slide {} read error: {}]", idx + 1, e)));
                        }
                    }
                }
                let meta = serde_json::json!({
                    "path": canonical_display,
                    "success": true,
                    "format": "ppt_images",
                    "slide_count": slide_count,
                    "image_format": fmt,
                    "dpi": _dpi,
                    "method": "libreoffice"
                });
                contents.push(rmcp::model::Content::text(meta.to_string()));
                return Ok(contents);
            }
            Err(e) => {
                // LibreOffice render failed, fall through to native method
                tracing::warn!("LibreOffice render failed ({}), falling back to native method", e);
            }
        }
    }

    // Native method: extract images + text per slide (pure Rust, no external deps)
    match converter::extract_pptx_images_text_native(&pptx_data, &out_dir) {
        Ok(slides_content) => {
            let slide_count = slides_content.len();
            let mut contents: Vec<rmcp::model::Content> = Vec::new();

            for slide in &slides_content {
                let slide_header = format!("## Slide {}", slide.index + 1);
                contents.push(rmcp::model::Content::text(slide_header));

                for (img_path, _w, _h, _) in &slide.images {
                    match tokio::fs::read(img_path).await {
                        Ok(img_data) => {
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&img_data);
                            let mime = if img_path.ends_with(".jpg") || img_path.ends_with(".jpeg") {
                                "image/jpeg"
                            } else {
                                "image/png"
                            };
                            contents.push(rmcp::model::Content::image(b64, mime.to_string()));
                        }
                        Err(e) => {
                            contents.push(rmcp::model::Content::text(
                                format!("[Image read error: {}]", e)
                            ));
                        }
                    }
                }

                if !slide.text.is_empty() {
                    contents.push(rmcp::model::Content::text(slide.text.clone()));
                }

                contents.push(rmcp::model::Content::text("\n---\n".to_string()));
            }

            let total_images: usize = slides_content.iter().map(|s| s.images.len()).sum();
            let soffice_note = if !soffice_available {
                ", note: install LibreOffice for full slide rendering (text+shapes as images)"
            } else {
                ""
            };
            let meta = serde_json::json!({
                "path": canonical_display,
                "success": true,
                "format": "ppt_images",
                "slide_count": slide_count,
                "image_count": total_images,
                "method": "native",
                "note": format!("Extracted embedded images + text from each slide.{}", soffice_note)
            });
            contents.push(rmcp::model::Content::text(meta.to_string()));
            Ok(contents)
        }
        Err(e) => {
            let r = err_result(_original_path, e);
            let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
            Ok(vec![rmcp::model::Content::text(json)])
        }
    }
}

async fn read_docx_text_mode(path_ref: &Path, _original_path: &str) -> Result<Vec<rmcp::model::Content>, String> {
    let canonical_display = strip_unc_prefix(&path_ref.to_string_lossy());
    let ext = get_file_extension(path_ref).unwrap_or_default();

    let data = match tokio::fs::read(path_ref).await {
        Ok(d) => d,
        Err(e) => {
            let r = err_result(_original_path, format!("Failed to read DOCX file: {}", e));
            let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
            return Ok(vec![rmcp::model::Content::text(json)]);
        }
    };

    if ext == "doc" {
        let soffice = match get_soffice() {
            Some(s) => s,
            None => {
                let r = err_result(_original_path,
                    "LibreOffice is required to read old .doc files. Please install LibreOffice.".to_string());
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                return Ok(vec![rmcp::model::Content::text(json)]);
            }
        };
        match converter::convert_old_format(soffice, &data, "doc", "docx") {
            Ok(docx_data) => {
                match extract_docx_markdown(&docx_data) {
                    Ok(text) => {
                        let r = text_result(canonical_display, text, "doc_text");
                        let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                        Ok(vec![rmcp::model::Content::text(json)])
                    }
                    Err(e) => {
                        let r = err_result(_original_path, e);
                        let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                        Ok(vec![rmcp::model::Content::text(json)])
                    }
                }
            }
            Err(e) => {
                let r = err_result(_original_path, e);
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                Ok(vec![rmcp::model::Content::text(json)])
            }
        }
    } else {
        match extract_docx_markdown(&data) {
            Ok(text) => {
                let r = text_result(canonical_display, text, "doc_text");
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                Ok(vec![rmcp::model::Content::text(json)])
            }
            Err(e) => {
                let r = err_result(_original_path, e);
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                Ok(vec![rmcp::model::Content::text(json)])
            }
        }
    }
}

async fn read_docx_with_images_mode(path_ref: &Path, _original_path: &str) -> Result<Vec<rmcp::model::Content>, String> {
    let canonical_display = strip_unc_prefix(&path_ref.to_string_lossy());
    let ext = get_file_extension(path_ref).unwrap_or_default();

    let data = match tokio::fs::read(path_ref).await {
        Ok(d) => d,
        Err(e) => {
            let r = err_result(_original_path, format!("Failed to read file: {}", e));
            let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
            return Ok(vec![rmcp::model::Content::text(json)]);
        }
    };

    let docx_data = if ext == "doc" {
        let soffice = match get_soffice() {
            Some(s) => s,
            None => {
                let r = err_result(_original_path,
                    "LibreOffice is required to read old .doc files. Please install LibreOffice.".to_string());
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                return Ok(vec![rmcp::model::Content::text(json)]);
            }
        };
        match converter::convert_old_format(soffice, &data, "doc", "docx") {
            Ok(d) => d,
            Err(e) => {
                let r = err_result(_original_path, e);
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                return Ok(vec![rmcp::model::Content::text(json)]);
            }
        }
    } else {
        data
    };

    let temp_dir = temp_image_dir();
    let hash = format!("{:x}", sha2::Sha256::digest(&docx_data));
    let temp_dir = temp_dir.join(format!("docx_{}", &hash[..12]));
    let _ = std::fs::create_dir_all(&temp_dir);

    match extract_docx_with_images(&docx_data, &temp_dir) {
        Ok((text, images)) => {
            let mut contents: Vec<rmcp::model::Content> = Vec::new();
            
            if images.is_empty() {
                let r = text_result(canonical_display, text, "doc_with_images");
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                contents.push(rmcp::model::Content::text(json));
                return Ok(contents);
            }

            // Split text at {{IMAGE:N}} markers
            let re = regex::Regex::new(r"\{\{IMAGE:(\d+)\}\}").map_err(|e| e.to_string())?;
            let mut last_end = 0;
            let mut image_count = 0;
            for captures in re.captures_iter(&text) {
                let full_match = captures.get(0).unwrap();
                let img_idx: usize = captures.get(1).unwrap().as_str().parse().unwrap_or(0);

                // Push text before this image as plain text (no JSON wrapper)
                let text_segment = &text[last_end..full_match.start()];
                if !text_segment.trim().is_empty() {
                    contents.push(rmcp::model::Content::text(text_segment.trim().to_string()));
                }

                // Push the image
                if img_idx < images.len() {
                    let ref img = images[img_idx];
                    match tokio::fs::read(&img.path).await {
                        Ok(img_data) => {
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&img_data);
                            let img_content = rmcp::model::Content::image(b64, img.mime_type.clone());
                            contents.push(img_content);
                            image_count += 1;
                        }
                        Err(e) => {
                            contents.push(rmcp::model::Content::text(
                                format!("[Image {} read error: {}]", img_idx + 1, e)
                            ));
                        }
                    }
                }

                last_end = full_match.end();
            }

            // Push remaining text after last image
            if last_end < text.len() {
                let text_segment = &text[last_end..];
                if !text_segment.trim().is_empty() {
                    contents.push(rmcp::model::Content::text(text_segment.trim().to_string()));
                }
            }

            // Append a single metadata note at the end
            let meta = serde_json::json!({
                "path": canonical_display,
                "success": true,
                "format": "doc_with_images",
                "image_count": image_count,
                "note": "Document text with images embedded inline above"
            });
            contents.push(rmcp::model::Content::text(meta.to_string()));

            Ok(contents)
        }
        Err(e) => {
            let r = err_result(_original_path, e);
            let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
            Ok(vec![rmcp::model::Content::text(json)])
        }
    }
}

async fn read_docx_images_mode(path_ref: &Path, _original_path: &str) -> Result<Vec<rmcp::model::Content>, String> {
    let canonical_display = strip_unc_prefix(&path_ref.to_string_lossy());
    let ext = get_file_extension(path_ref).unwrap_or_default();

    let data = match tokio::fs::read(path_ref).await {
        Ok(d) => d,
        Err(e) => {
            let r = err_result(_original_path, format!("Failed to read file: {}", e));
            let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
            return Ok(vec![rmcp::model::Content::text(json)]);
        }
    };

    let docx_data = if ext == "doc" {
        let soffice = match get_soffice() {
            Some(s) => s,
            None => {
                let r = err_result(_original_path,
                    "LibreOffice is required to read old .doc files. Please install LibreOffice.".to_string());
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                return Ok(vec![rmcp::model::Content::text(json)]);
            }
        };
        match converter::convert_old_format(soffice, &data, "doc", "docx") {
            Ok(d) => d,
            Err(e) => {
                let r = err_result(_original_path, e);
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                return Ok(vec![rmcp::model::Content::text(json)]);
            }
        }
    } else {
        data
    };

    let temp_dir = temp_image_dir();
    let hash = format!("{:x}", sha2::Sha256::digest(&docx_data));
    let temp_dir = temp_dir.join(format!("docx_{}", &hash[..12]));
    let _ = std::fs::create_dir_all(&temp_dir);

    match extract_docx_images(&docx_data, &temp_dir) {
        Ok((images, img_count)) => {
            let mut contents: Vec<rmcp::model::Content> = Vec::new();
            for img in &images {
                match tokio::fs::read(&img.path).await {
                    Ok(img_data) => {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&img_data);
                        contents.push(rmcp::model::Content::image(b64, img.mime_type.clone()));
                    }
                    Err(e) => {
                        contents.push(rmcp::model::Content::text(
                            format!("[Image {} read error: {}]", img.index + 1, e)
                        ));
                    }
                }
            }
            let meta = serde_json::json!({
                "path": canonical_display,
                "success": true,
                "format": "doc_images",
                "image_count": img_count
            });
            contents.push(rmcp::model::Content::text(meta.to_string()));
            Ok(contents)
        }
        Err(e) => {
            let r = err_result(_original_path, e);
            let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
            Ok(vec![rmcp::model::Content::text(json)])
        }
    }
}

async fn read_office_format(
    path_ref: &Path,
    _original_path: &str,
    fmt: OfficeFormat,
    sheet_name: Option<&str>,
) -> Result<Vec<rmcp::model::Content>, String> {
    let canonical_display = strip_unc_prefix(&path_ref.to_string_lossy());

    let data = match tokio::fs::read(path_ref).await {
        Ok(d) => d,
        Err(e) => {
            let r = err_result(_original_path, format!("Failed to read file: {}", e));
            let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
            return Ok(vec![rmcp::model::Content::text(json)]);
        }
    };

    let format_str = format!("{:?}", fmt).to_lowercase();
    if fmt == OfficeFormat::Xls {
        match extract_text_from_bytes(&data, fmt, sheet_name) {
            Ok(text) => {
                let r = text_result(canonical_display, text, &format_str);
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                Ok(vec![rmcp::model::Content::text(json)])
            }
            Err(e) => {
                let r = err_result(_original_path, format!("XLS extraction failed: {}", e));
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                Ok(vec![rmcp::model::Content::text(json)])
            }
        }
    } else {
        match extract_text_from_bytes(&data, fmt, sheet_name) {
            Ok(text) => {
                let r = text_result(canonical_display, text, &format_str);
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                Ok(vec![rmcp::model::Content::text(json)])
            }
            Err(e) => {
                let r = err_result(_original_path, format!("Format extraction failed: {}", e));
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                Ok(vec![rmcp::model::Content::text(json)])
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn read_text_mode(
    path_ref: &Path,
    original_path: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
    offset_chars: Option<usize>,
    max_chars: Option<usize>,
    line_numbers: Option<bool>,
    highlight_line: Option<usize>,
) -> Result<Vec<rmcp::model::Content>, String> {
    let canonical_display = strip_unc_prefix(&path_ref.to_string_lossy());
    let start_line = start_line.unwrap_or(0);
    let end_line = end_line.unwrap_or(500);
    let max_chars = max_chars.unwrap_or(15000);
    let line_numbers = line_numbers.unwrap_or(false);
    let is_text = is_text_file(path_ref);

    let result = if let Some(offset) = offset_chars {
        const MAX_READ_SIZE: u64 = 100 * 1024 * 1024;
        if let Ok(meta) = tokio::fs::metadata(path_ref).await {
            if meta.len() > MAX_READ_SIZE {
                let r = ReadResult {
                    path: canonical_display,
                    success: false,
                    error: Some(format!(
                        "File too large ({} bytes). Use start_line/end_line instead of offset_chars for large files.",
                        meta.len()
                    )),
                    content: None,
                    images: None,
                    lines_displayed: None,
                    total_lines: None,
                    truncated: None,
                    format: Some("text".to_string()),
                    page_count: None,
                };
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                return Ok(vec![rmcp::model::Content::text(json)]);
            }
        }
        let file_content = match tokio::fs::read_to_string(path_ref).await {
            Ok(c) => c,
            Err(e) => {
                let msg = if !is_text {
                    format!(
                        "File '{}' appears to be binary and cannot be read as text: {}",
                        original_path, e
                    )
                } else {
                    format!("Failed to read file '{}': {}", path_ref.display(), e)
                };
                let r = ReadResult {
                    path: canonical_display,
                    success: false,
                    error: Some(msg),
                    content: None,
                    images: None,
                    lines_displayed: None,
                    total_lines: None,
                    truncated: None,
                    format: Some("text".to_string()),
                    page_count: None,
                };
                let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                return Ok(vec![rmcp::model::Content::text(json)]);
            }
        };

        let total_chars = file_content.chars().count();
        let offset = offset.min(total_chars);
        let slice: String = file_content.chars().skip(offset).collect();

        let total_lines = file_content.lines().count();

        let prefix: String = file_content.chars().take(offset).collect();
        let computed_start_line = prefix.lines().count().saturating_sub(1);

        let mut content = String::new();
        let mut chars_count = 0;
        let mut lines_included = 0;
        let mut truncated_val = false;
        let mut slice_byte_pos = 0;

        for (idx, line) in slice.lines().enumerate() {
            if idx > 0 {
                if slice.as_bytes().get(slice_byte_pos) == Some(&b'\r') {
                    slice_byte_pos += 1;
                }
                if slice.as_bytes().get(slice_byte_pos) == Some(&b'\n') {
                    slice_byte_pos += 1;
                }
            }

            let line_num = computed_start_line + idx;
            let is_highlight = highlight_line
                .map(|hl| hl == line_num + 1)
                .unwrap_or(false);
            let formatted = if line_numbers {
                if is_highlight {
                    format!(">>>{:4} | {}\n", line_num, line)
                } else {
                    format!("{:4} | {}\n", line_num, line)
                }
            } else if is_highlight {
                format!(">>> {}\n", line)
            } else {
                format!("{}\n", line)
            };

            if chars_count + formatted.len() > max_chars {
                truncated_val = true;
                break;
            }
            chars_count += formatted.len();
            content.push_str(&formatted);
            lines_included += 1;
            slice_byte_pos += line.len();
        }
        if content.ends_with('\n') {
            content.pop();
        }

        let mut response = content;
        if truncated_val {
            let next_start = if offset + slice_byte_pos < total_chars {
                offset + slice_byte_pos
            } else {
                offset + chars_count
            };
            let next_start_lines = computed_start_line + lines_included;
            response.push_str(&format!(
                "\n\n[... Content truncated at {} characters. To continue, use start_line={} or offset_chars={} ...]",
                max_chars, next_start_lines, next_start
            ));
        }

        let has_more = offset + slice_byte_pos < total_chars;
        let is_partial = lines_included < total_lines;

        if is_partial || has_more {
            response.push_str(&format!(
                "\n\n[File info: total_lines={}, lines_displayed={}",
                total_lines, lines_included
            ));
            if has_more {
                let hint_start = offset + slice_byte_pos;
                response.push_str(&format!(
                    ", hint: start_line={} end_line={} or offset_chars={}]",
                    hint_start, hint_start + 500, hint_start
                ));
            } else {
                response.push(']');
            }
        }

        ReadResult {
            path: canonical_display,
            success: true,
            error: None,
            content: Some(response),
            images: None,
            lines_displayed: Some(lines_included),
            total_lines: Some(total_lines),
            truncated: Some(truncated_val),
            format: Some("text".to_string()),
            page_count: None,
        }
    } else {
        let (mut content, lines_read, truncated_val, total_lines) =
            match read_file_with_options(
                path_ref,
                start_line,
                end_line,
                max_chars,
                line_numbers,
            )
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    let msg = if !is_text {
                        format!(
                            "File '{}' appears to be binary and cannot be read as text: {}",
                            original_path, e
                        )
                    } else {
                        e
                    };
                    let r = ReadResult {
                        path: canonical_display,
                        success: false,
                        error: Some(msg),
                        content: None,
                        images: None,
                        lines_displayed: None,
                        total_lines: None,
                        truncated: None,
                        format: Some("text".to_string()),
                        page_count: None,
                    };
                    let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
                    return Ok(vec![rmcp::model::Content::text(json)]);
                }
            };

        if let Some(hl) = highlight_line {
            if hl > start_line && hl <= end_line.min(total_lines) {
                let hl_0based = hl - start_line - 1;
                let lines_in_content: Vec<&str> = content.lines().collect();
                let mut new_lines = Vec::new();
                for (idx, line) in lines_in_content.iter().enumerate() {
                    if idx == hl_0based {
                        if line_numbers {
                            let prefix_match = line.chars().take(4).collect::<String>() == "   0"
                                || line
                                    .chars()
                                    .take(4)
                                    .collect::<String>()
                                    .trim_start_matches(' ')
                                    .len()
                                    <= 4;
                            if prefix_match && line.get(4..7) == Some(" | ") {
                                new_lines.push(format!(">>>{}", line));
                            } else if line.starts_with(">>>") {
                                new_lines.push(line.to_string());
                            } else {
                                new_lines.push(format!(">>> {}", line));
                            }
                        } else {
                            new_lines.push(format!(">>> {}", line));
                        }
                    } else {
                        new_lines.push(line.to_string());
                    }
                }
                content = new_lines.join("\n");
            }
        }

        let mut response = content;
        if truncated_val {
            let next_start = start_line + lines_read;
            response.push_str(&format!(
                "\n\n[... Content truncated at {} characters. To continue, use start_line={} ...]",
                max_chars, next_start
            ));
        }

        let has_more = end_line < total_lines || truncated_val;
        let is_partial = lines_read < total_lines;

        if is_partial || has_more {
            response.push_str(&format!(
                "\n\n[File info: total_lines={}, lines_displayed={}",
                total_lines, lines_read
            ));
            if has_more {
                let hint_start = start_line + lines_read;
                let hint_end = hint_start + 500;
                response.push_str(&format!(
                    ", hint: start_line={} end_line={}]",
                    hint_start, hint_end
                ));
            } else {
                response.push(']');
            }
        }

        ReadResult {
            path: canonical_display,
            success: true,
            error: None,
            content: Some(response),
            images: None,
            lines_displayed: Some(lines_read),
            total_lines: Some(total_lines),
            truncated: Some(truncated_val),
            format: Some("text".to_string()),
            page_count: None,
        }
    };

    let json = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
    Ok(vec![rmcp::model::Content::text(json)])
}

// ---- Helpers ----

fn err_result(path_str: &str, error: String) -> ReadResult {
    ReadResult {
        path: path_str.to_string(),
        success: false,
        error: Some(error),
        content: None,
        images: None,
        lines_displayed: None,
        total_lines: None,
        truncated: None,
        format: None,
        page_count: None,
    }
}

fn text_result(path: String, text: String, format: &str) -> ReadResult {
    ReadResult {
        path,
        success: true,
        error: None,
        content: Some(text),
        images: None,
        lines_displayed: None,
        total_lines: None,
        truncated: None,
        format: Some(format.to_string()),
        page_count: None,
    }
}

fn estimate_pdf_page_count(data: &[u8]) -> Option<usize> {
    use lopdf::Document;
    use std::io::Cursor;

    let cursor = Cursor::new(data.to_vec());
    let doc = Document::load_from(cursor).ok()?;
    Some(doc.get_pages().len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_read() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Line 1\nLine 2\nLine 3").unwrap();

        let params = ReadParams {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("text".to_string()),
            start_line: Some(0),
            end_line: Some(2),
            offset_chars: None,
            max_chars: None,
            line_numbers: Some(true),
            highlight_line: None,
            sheet_name: None,
            image_dpi: None,
            image_format: None,
            paths: None,
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("Line 1"));
                assert!(text.text.contains("Line 2"));
                assert!(!text.text.contains("Line 3"));
                assert!(text.text.contains("total_lines=3"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_read_highlight() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Line 1\nLine 2\nLine 3").unwrap();

        let params = ReadParams {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("text".to_string()),
            start_line: Some(0),
            end_line: Some(10),
            offset_chars: None,
            max_chars: None,
            line_numbers: Some(true),
            highlight_line: Some(2),
            sheet_name: None,
            image_dpi: None,
            image_format: None,
            paths: None,
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains(">>>"));
                assert!(text.text.contains("Line 2"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_read_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let params = ReadParams {
            path: "/nonexistent/file.txt".to_string(),
            mode: None,
            start_line: None,
            end_line: None,
            offset_chars: None,
            max_chars: None,
            line_numbers: None,
            highlight_line: None,
            sheet_name: None,
            image_dpi: None,
            image_format: None,
            paths: None,
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"success\": false"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_read_offset_chars() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello\nWorld\nFoo").unwrap();

        let params = ReadParams {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("text".to_string()),
            start_line: None,
            end_line: None,
            offset_chars: Some(6),
            max_chars: None,
            line_numbers: Some(true),
            highlight_line: None,
            sheet_name: None,
            image_dpi: None,
            image_format: None,
            paths: None,
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("World"));
                assert!(!text.text.contains("Hello"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_read_multiple() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("a.txt");
        let file2 = temp_dir.path().join("b.txt");
        fs::write(&file1, "File A").unwrap();
        fs::write(&file2, "File B\nLine 2").unwrap();

        let params = ReadParams {
            path: String::new(),
            mode: None,
            start_line: None,
            end_line: None,
            offset_chars: None,
            max_chars: None,
            line_numbers: None,
            highlight_line: None,
            sheet_name: None,
            image_dpi: None,
            image_format: None,
            paths: Some(vec![
                ReadItem {
                    path: file1.to_string_lossy().to_string(),
                    start_line: Some(0),
                    end_line: Some(10),
                    offset_chars: None,
                    max_chars: None,
                    line_numbers: Some(true),
                    highlight_line: None,
                    mode: Some("text".to_string()),
                    sheet_name: None,
                    image_dpi: None,
                    image_format: None,
                },
                ReadItem {
                    path: file2.to_string_lossy().to_string(),
                    start_line: Some(0),
                    end_line: Some(10),
                    offset_chars: None,
                    max_chars: None,
                    line_numbers: Some(true),
                    highlight_line: None,
                    mode: Some("text".to_string()),
                    sheet_name: None,
                    image_dpi: None,
                    image_format: None,
                },
            ]),
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            assert!(call_result.content.len() >= 2);
            let mut found_a = false;
            let mut found_b = false;
            for c in &call_result.content {
                if let Some(text) = c.as_text() {
                    if text.text.contains("File A") { found_a = true; }
                    if text.text.contains("File B") { found_b = true; }
                }
            }
            assert!(found_a, "Should contain File A");
            assert!(found_b, "Should contain File B");
        }
    }

    #[tokio::test]
    async fn test_file_read_media_mode() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.png");
        let png_data = [
            0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
            b'I', b'H', b'D', b'R', 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x10,
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE,
        ];
        fs::write(&file_path, png_data).unwrap();

        let params = ReadParams {
            path: file_path.to_string_lossy().to_string(),
            mode: None,
            start_line: None,
            end_line: None,
            offset_chars: None,
            max_chars: None,
            line_numbers: None,
            highlight_line: None,
            sheet_name: None,
            image_dpi: None,
            image_format: None,
            paths: None,
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            // First content should be the image
            assert!(call_result.content.first().and_then(|c| c.as_image()).is_some(), "First item should be image content");
            // Second content should be text metadata
            if let Some(text) = call_result.content.get(1).and_then(|c| c.as_text()) {
                assert!(text.text.contains("image/png"));
                assert!(text.text.contains("16"));
            }
        }
    }

    #[test]
    fn test_pdf_page_count_estimate() {
        let result = estimate_pdf_page_count(b"not a pdf");
        assert!(result.is_none());
    }
}
