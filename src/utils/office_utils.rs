use std::collections::HashMap;
use std::io::Cursor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OfficeFormat {
    Doc,
    Docx,
    Ppt,
    Pptx,
    Xls,
    Xlsx,
    Pdf,
    Ipynb,
    Unknown,
}

pub fn detect_office_format(extension: &str) -> OfficeFormat {
    match extension.to_lowercase().as_str() {
        "doc" => OfficeFormat::Doc,
        "docx" => OfficeFormat::Docx,
        "ppt" => OfficeFormat::Ppt,
        "pptx" => OfficeFormat::Pptx,
        "xls" => OfficeFormat::Xls,
        "xlsx" => OfficeFormat::Xlsx,
        "pdf" => OfficeFormat::Pdf,
        "ipynb" => OfficeFormat::Ipynb,
        _ => OfficeFormat::Unknown,
    }
}

#[allow(dead_code)]
pub fn is_old_format(format: OfficeFormat) -> bool {
    matches!(format, OfficeFormat::Doc | OfficeFormat::Ppt)
}

#[allow(dead_code)]
pub fn old_format_new_extension(format: OfficeFormat) -> Option<&'static str> {
    match format {
        OfficeFormat::Doc => Some("docx"),
        OfficeFormat::Ppt => Some("pptx"),
        OfficeFormat::Xls => Some("xlsx"),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn old_format_extension(format: OfficeFormat) -> Option<&'static str> {
    match format {
        OfficeFormat::Doc => Some("doc"),
        OfficeFormat::Ppt => Some("ppt"),
        OfficeFormat::Xls => Some("xls"),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct ImageMeta {
    pub index: usize,
    pub path: String,
    pub filename: String,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub description: Option<String>,
}

pub fn extract_text_from_bytes(
    data: &[u8],
    format: OfficeFormat,
    sheet_name: Option<&str>,
) -> Result<String, String> {
    if is_old_format(format) {
        return Err(format!(
            "{:?} is a legacy Office binary format and cannot be parsed directly. \
            LibreOffice is required to convert it to the modern OpenXML format first.",
            format
        ));
    }
    match format {
        OfficeFormat::Docx | OfficeFormat::Doc => extract_docx_text(data),
        OfficeFormat::Pptx | OfficeFormat::Ppt => extract_pptx_text(data),
        OfficeFormat::Xlsx | OfficeFormat::Xls => extract_xlsx_text(data, sheet_name),
        OfficeFormat::Pdf => extract_pdf_text(data),
        OfficeFormat::Ipynb => extract_ipynb_text(data),
        OfficeFormat::Unknown => Err("Unsupported office format".to_string()),
    }
}

pub fn extract_docx_markdown(data: &[u8]) -> Result<String, String> {
    use docx_rs::*;

    let docx = read_docx(data).map_err(|e| format!("DOCX read error: {}", e))?;
    let mut md = String::new();
    let heading_map = docx.styles.create_heading_style_map();
    let image_map: HashMap<String, usize> = docx.images.iter().enumerate().map(|(i, (id, _, _, _))| (id.clone(), i)).collect();

    render_document_children_markdown(&docx.document.children, &mut md, &heading_map, &image_map);

    Ok(md.trim().to_string())
}

fn render_document_children_markdown(
    children: &[docx_rs::DocumentChild],
    out: &mut String,
    heading_map: &HashMap<String, usize>,
    image_map: &HashMap<String, usize>,
) {
    for child in children {
        match child {
            docx_rs::DocumentChild::Paragraph(para) => {
                render_paragraph_markdown(para, out, heading_map, image_map);
            }
            docx_rs::DocumentChild::Table(table) => {
                render_table_markdown(table, out, image_map);
            }
            docx_rs::DocumentChild::StructuredDataTag(_sdt) => {}
            docx_rs::DocumentChild::BookmarkStart(_)
            | docx_rs::DocumentChild::BookmarkEnd(_)
            | docx_rs::DocumentChild::CommentStart(_)
            | docx_rs::DocumentChild::CommentEnd(_)
            | docx_rs::DocumentChild::TableOfContents(_) => {}
            docx_rs::DocumentChild::Section(_section) => {}
        }
    }
}

fn render_paragraph_markdown(
    para: &docx_rs::Paragraph,
    out: &mut String,
    heading_map: &HashMap<String, usize>,
    image_map: &HashMap<String, usize>,
) {
    let style_id = para
        .property
        .style
        .as_ref()
        .map(|s| s.val.clone())
        .unwrap_or_default();

    let heading_level = heading_map.get(&style_id).copied();
    let is_numbered = para.property.numbering_property.is_some();

    if let Some(level) = heading_level {
        let text = paragraph_raw_text(para);
        if text.trim().is_empty() {
            return;
        }
        let level_actual = level.min(6);
        let hashes = "#".repeat(level_actual);
        let formatted = paragraph_formatted_text(para, image_map);
        out.push_str(&format!("{} {}\n\n", hashes, formatted.trim()));
    } else if is_numbered {
        let formatted = paragraph_formatted_text(para, image_map);
        if formatted.trim().is_empty() {
            return;
        }
        out.push_str(&format!("- {}\n", formatted.trim()));
    } else {
        let formatted = paragraph_formatted_text(para, image_map);
        if formatted.trim().is_empty() {
            out.push('\n');
            return;
        }
        out.push_str(&formatted);
        out.push_str("\n\n");
    }
}

fn paragraph_raw_text(para: &docx_rs::Paragraph) -> String {
    let mut text = String::new();
    for child in &para.children {
        match child {
            docx_rs::ParagraphChild::Run(run) => {
                for rc in &run.children {
                    if let docx_rs::RunChild::Text(t) = rc {
                        text.push_str(&t.text);
                    }
                }
            }
            docx_rs::ParagraphChild::Insert(ins) => {
                for ic in &ins.children {
                    if let docx_rs::InsertChild::Run(run) = ic {
                        for rc in &run.children {
                            if let docx_rs::RunChild::Text(t) = rc {
                                text.push_str(&t.text);
                            }
                        }
                    }
                }
            }
            docx_rs::ParagraphChild::Hyperlink(hl) => {
                for hc in &hl.children {
                    if let docx_rs::ParagraphChild::Run(run) = hc {
                        for rc in &run.children {
                            if let docx_rs::RunChild::Text(t) = rc {
                                text.push_str(&t.text);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    text
}

fn paragraph_formatted_text(para: &docx_rs::Paragraph, image_map: &HashMap<String, usize>) -> String {
    let mut result = String::new();
    for child in &para.children {
        match child {
            docx_rs::ParagraphChild::Run(run) => {
                result.push_str(&run_formatted_text(run, image_map));
            }
            docx_rs::ParagraphChild::Insert(ins) => {
                for ic in &ins.children {
                    if let docx_rs::InsertChild::Run(run) = ic {
                        result.push_str(&run_formatted_text(run, image_map));
                    }
                }
            }
            docx_rs::ParagraphChild::Hyperlink(hl) => {
                let mut link_text = String::new();
                let mut url = String::new();
                if let docx_rs::HyperlinkData::External { ref path, .. } = hl.link {
                    url = path.clone();
                }
                for hc in &hl.children {
                    if let docx_rs::ParagraphChild::Run(run) = hc {
                        link_text.push_str(&run_formatted_text(run, image_map));
                    }
                }
                if !url.is_empty() {
                    result.push_str(&format!("[{}]({})", link_text, url));
                } else {
                    result.push_str(&link_text);
                }
            }
            _ => {}
        }
    }
    result
}

fn run_formatted_text(run: &docx_rs::Run, image_map: &HashMap<String, usize>) -> String {
    let mut text = String::new();
    for child in &run.children {
        match child {
            docx_rs::RunChild::Text(t) => text.push_str(&t.text),
            docx_rs::RunChild::Drawing(d) => {
                if let Some(ref data) = d.data {
                    match data {
                        docx_rs::DrawingData::Pic(pic) => {
                            if let Some(img_idx) = image_map.get(&pic.id) {
                                text.push_str(&format!("{{{{IMAGE:{}}}}}", img_idx));
                            }
                        }
                        docx_rs::DrawingData::TextBox(_) => {
                            text.push_str("{{IMAGE:textbox}}");
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if text.is_empty() {
        return text;
    }

    let rp = &run.run_property;

    let is_bold = rp.bold.is_some();
    let is_italic = rp.italic.is_some();
    let is_strike = rp.strike.is_some() || rp.dstrike.is_some();

    if is_strike {
        text = format!("~~{}~~", text);
    }
    if is_bold && is_italic {
        text = format!("***{}***", text);
    } else if is_bold {
        text = format!("**{}**", text);
    } else if is_italic {
        text = format!("*{}*", text);
    }
    text
}

fn render_table_markdown(table: &docx_rs::Table, out: &mut String, image_map: &HashMap<String, usize>) {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for row_child in &table.rows {
        let docx_rs::TableChild::TableRow(row) = row_child;
        let mut cells = Vec::new();
        for cell_child in &row.cells {
            let docx_rs::TableRowChild::TableCell(cell) = cell_child;
            let mut cell_text = String::new();
            for content in &cell.children {
                if let docx_rs::TableCellContent::Paragraph(p) = content {
                    cell_text.push_str(&paragraph_formatted_text(p, image_map));
                    cell_text.push(' ');
                }
            }
            cells.push(cell_text.trim().to_string());
        }
        if !cells.iter().all(|c| c.is_empty()) {
            rows.push(cells);
        }
    }

    if rows.is_empty() {
        return;
    }

    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if col_count == 0 {
        return;
    }

    for (ri, row) in rows.iter().enumerate() {
        out.push('|');
        for ci in 0..col_count {
            out.push(' ');
            out.push_str(row.get(ci).map(|s| s.as_str()).unwrap_or(""));
            out.push_str(" |");
        }
        out.push('\n');

        if ri == 0 {
            out.push('|');
            for _ in 0..col_count {
                out.push_str(" --- |");
            }
            out.push('\n');
        }
    }
    out.push('\n');
}

pub fn extract_docx_text(data: &[u8]) -> Result<String, String> {
    extract_docx_markdown(data)
}

pub fn extract_docx_images(data: &[u8], temp_dir: &std::path::Path) -> Result<(Vec<ImageMeta>, usize), String> {
    use docx_rs::*;

    let docx = read_docx(data).map_err(|e| format!("DOCX read error: {}", e))?;
    let mut images = Vec::new();

    let total_images = docx.images.len();
    for (idx, (_id, _path, _image_data, png_data)) in docx.images.iter().enumerate() {
        let ext = std::path::Path::new(&_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let mime = match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "webp" => "image/webp",
            _ => "image/png",
        };

        let hash_bytes = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&png_data.0);
            h.finalize()
        };
        let hash_str = format!("{:x}", hash_bytes);
        let filename = format!("docx_img_{:04}_{}.{}", idx, &hash_str[..8], ext);
        let filepath = temp_dir.join(&filename);

        std::fs::write(&filepath, &png_data.0)
            .map_err(|e| format!("Failed to write docx image: {}", e))?;

        let metadata = std::fs::metadata(&filepath)
            .map_err(|e| format!("Failed to read image metadata: {}", e))?;

        let dims = crate::utils::image_utils::get_image_dimensions(&filepath);

        images.push(ImageMeta {
            index: idx,
            path: filepath.to_string_lossy().to_string(),
            filename,
            mime_type: mime.to_string(),
            width: dims.map(|d| d.0).unwrap_or(0),
            height: dims.map(|d| d.1).unwrap_or(0),
            size_bytes: metadata.len(),
            description: Some(format!("Image {}", idx + 1)),
        });
    }

    Ok((images, total_images))
}

pub fn extract_docx_with_images(
    data: &[u8],
    temp_dir: &std::path::Path,
) -> Result<(String, Vec<ImageMeta>), String> {
    let markdown = extract_docx_markdown(data)?;
    let (images, _) = extract_docx_images(data, temp_dir)?;

    Ok((markdown, images))
}

fn extract_pptx_text(data: &[u8]) -> Result<String, String> {
    use std::io::Write;

    let mut tmp_file = tempfile::Builder::new()
        .prefix("mcp_pptx_")
        .suffix(".pptx")
        .tempfile()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    tmp_file
        .write_all(data)
        .map_err(|e| format!("Failed to write temp PPTX file: {}", e))?;

    let result = (|| -> Result<String, String> {
        use ppt_rs::oxml::presentation::PresentationReader;

        let path_str = tmp_file.path().to_string_lossy().to_string();
        let reader = PresentationReader::open(&path_str)
            .map_err(|e| format!("PPTX open error: {}", e))?;

        let mut text = String::new();

        let slides = reader
            .get_all_slides()
            .map_err(|e| format!("PPTX slide read error: {}", e))?;

        for (i, slide) in slides.iter().enumerate() {
            text.push_str(&format!("## Slide {}\n\n", i + 1));

            let all_texts = slide.all_text();
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
                    for (ri, row) in rows.iter().enumerate() {
                        text.push('|');
                        for cell in row {
                            text.push(' ');
                            text.push_str(&cell.text);
                            text.push_str(" |");
                        }
                        text.push('\n');
                        if ri == 0 {
                            text.push('|');
                            for _ in 0..row.len() {
                                text.push_str(" --- |");
                            }
                            text.push('\n');
                        }
                    }
                    text.push('\n');
                }
            }

            text.push('\n');
        }

        Ok(text.trim().to_string())
    })();

    if let Err(e) = tmp_file.close() {
        tracing::warn!("Failed to clean up temp PPTX file: {}", e);
    }
    result
}

fn extract_xlsx_text(data: &[u8], sheet_name: Option<&str>) -> Result<String, String> {
    use calamine::{open_workbook_auto_from_rs, Data, Reader};
    use std::fmt::Write;

    let cursor = Cursor::new(data);
    let mut wb = open_workbook_auto_from_rs(cursor)
        .map_err(|e| format!("XLSX/XLS parse error: {}", e))?;

    let sheet_names = wb.sheet_names();
    let mut text = String::new();

    let sheets_to_read: Vec<String> = if let Some(name) = sheet_name {
        if sheet_names.contains(&name.to_string()) {
            vec![name.to_string()]
        } else {
            return Err(format!(
                "Sheet '{}' not found. Available sheets: {}",
                name,
                sheet_names.join(", ")
            ));
        }
    } else {
        sheet_names
    };

    for name in sheets_to_read {
        let range = wb
            .worksheet_range(&name)
            .map_err(|e| format!("Sheet '{}' read error: {}", name, e))?;

        text.push_str(&format!("## {}\n\n", name));

        let rows_data = range.rows();
        for row in rows_data {
            let cells: Vec<String> = row
                .iter()
                .map(|cell| match cell {
                    Data::Empty => String::new(),
                    Data::String(s) => s.clone(),
                    Data::Float(f) => f.to_string(),
                    Data::Int(i) => i.to_string(),
                    Data::Bool(b) => b.to_string(),
                    Data::Error(e) => e.to_string(),
                    Data::DateTime(dt) => dt.to_string(),
                    Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
                })
                .collect();

            if cells.iter().any(|c| !c.is_empty()) {
                writeln!(text, "| {} |", cells.join(" | ")).ok();
            }
        }
        text.push('\n');
    }

    Ok(text.trim().to_string())
}

fn extract_pdf_text(data: &[u8]) -> Result<String, String> {
    use lopdf::Document;
    use std::io::Cursor;

    let cursor = Cursor::new(data.to_vec());
    let doc = Document::load_from(cursor)
        .map_err(|e| format!("PDF parse error: {}", e))?;

    let pages: Vec<u32> = doc.get_pages().keys().copied().collect();
    let text = doc
        .extract_text(&pages)
        .map_err(|e| format!("PDF text extraction error: {}", e))?;

    Ok(text.trim().to_string())
}

fn extract_ipynb_text(data: &[u8]) -> Result<String, String> {
    use ipynb::Cell;
    use ipynb::Notebook;

    let notebook: Notebook =
        serde_json::from_slice(data).map_err(|e| format!("IPYNB parse error: {}", e))?;

    let mut text = String::new();

    for (i, cell) in notebook.cells.iter().enumerate() {
        match cell {
            Cell::Markdown(md) => {
                text.push_str(&format!("[Cell {}: Markdown]\n", i + 1));
                text.push_str(&md.source.join(""));
                text.push_str("\n\n");
            }
            Cell::Code(code) => {
                text.push_str(&format!(
                    "[Cell {}: Code - execution_count: {}]\n",
                    i + 1,
                    code.execution_count
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "not executed".to_string())
                ));
                text.push_str(&code.source.join(""));
                text.push('\n');

                if !code.outputs.is_empty() {
                    text.push_str("--- Outputs ---\n");
                    for output in &code.outputs {
                        use ipynb::Output;
                        match output {
                            Output::Stream(stream) => {
                                if let Some(ref txt) = stream.text {
                                    text.push_str(&txt.join(""));
                                }
                            }
                            Output::ExecuteResult(exec) => {
                                if let Some(ref data) = exec.data {
                                    if let Some(text_plain) =
                                        data.get("text/plain").and_then(|v| v.as_str())
                                    {
                                        text.push_str(text_plain);
                                    }
                                }
                            }
                            Output::DisplayData(disp) => {
                                if let Some(ref data) = disp.data {
                                    if let Some(text_plain) =
                                        data.get("text/plain").and_then(|v| v.as_str())
                                    {
                                        text.push_str(text_plain);
                                    }
                                }
                            }
                            Output::Error(err) => {
                                text.push_str(&format!("Error: {}: {}\n", err.ename, err.evalue));
                            }
                        }
                        text.push('\n');
                    }
                }
                text.push('\n');
            }
            Cell::Raw(raw) => {
                text.push_str(&format!("[Cell {}: Raw]\n", i + 1));
                text.push_str(&raw.source.join(""));
                text.push_str("\n\n");
            }
        }
    }

    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_office_format() {
        assert_eq!(detect_office_format("docx"), OfficeFormat::Docx);
        assert_eq!(detect_office_format("DOCX"), OfficeFormat::Docx);
        assert_eq!(detect_office_format("doc"), OfficeFormat::Doc);
        assert_eq!(detect_office_format("pptx"), OfficeFormat::Pptx);
        assert_eq!(detect_office_format("ppt"), OfficeFormat::Ppt);
        assert_eq!(detect_office_format("xlsx"), OfficeFormat::Xlsx);
        assert_eq!(detect_office_format("xls"), OfficeFormat::Xls);
        assert_eq!(detect_office_format("pdf"), OfficeFormat::Pdf);
        assert_eq!(detect_office_format("ipynb"), OfficeFormat::Ipynb);
        assert_eq!(detect_office_format("txt"), OfficeFormat::Unknown);
    }

    #[test]
    fn test_extract_text_unknown() {
        let result = extract_text_from_bytes(b"test", OfficeFormat::Unknown, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_old_format() {
        assert!(is_old_format(OfficeFormat::Doc));
        assert!(is_old_format(OfficeFormat::Ppt));
        assert!(!is_old_format(OfficeFormat::Docx));
        assert!(!is_old_format(OfficeFormat::Pptx));
        assert!(!is_old_format(OfficeFormat::Xls));
    }
}
