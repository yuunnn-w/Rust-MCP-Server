use crate::utils::file_utils::{ensure_path_within_working_dir, strip_unc_prefix};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileEditOperation {
    #[schemars(description = "The file path to edit")]
    pub path: String,
    #[schemars(description = "Edit mode: string_replace (default), line_replace, insert, delete, patch, office_insert, office_replace, office_delete, office_insert_image, office_format, office_insert_table, pdf_delete_page, pdf_insert_image, pdf_insert_text, pdf_replace_text, pdf_merge")]
    pub mode: Option<String>,

    // string_replace mode
    #[schemars(description = "String to find (exact match).")]
    pub old_string: Option<String>,
    #[schemars(description = "Replacement string.")]
    pub new_string: Option<String>,
    #[schemars(description = "Which occurrence: 1=first, 2=second, 0=all.")]
    pub occurrence: Option<usize>,

    // line_replace / insert / delete mode
    #[schemars(description = "Start line number (1-based).")]
    pub start_line: Option<usize>,
    #[schemars(description = "End line number (1-based).")]
    pub end_line: Option<usize>,

    // patch mode
    #[schemars(description = "Unified diff patch string.")]
    pub patch: Option<String>,

    // Complex office mode parameters
    #[schemars(description = "Markdown content for office_insert/office_replace/office_insert_table.")]
    pub markdown: Option<String>,
    #[schemars(description = "Local image path for office_insert_image/pdf_insert_image.")]
    pub image_path: Option<String>,
    #[schemars(description = "Text to find for positioning.")]
    pub find_text: Option<String>,
    #[schemars(description = "Where: before, after, replace.")]
    pub location: Option<String>,
    #[schemars(description = "Element type: paragraph, table, image, heading, any.")]
    pub element_type: Option<String>,
    #[schemars(description = "Format: bold, italic, heading1..heading6, strikethrough.")]
    pub format_type: Option<String>,
    #[schemars(description = "Slide index for PPTX (0-based).")]
    pub slide_index: Option<usize>,
    #[schemars(description = "Page index for PDF (0-based).")]
    pub page_index: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditParams {
    #[schemars(description = "List of file edit operations.")]
    pub operations: Vec<FileEditOperation>,
    #[schemars(description = "File type. Auto-detect by extension if not specified.")]
    pub file_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct FileEditResult {
    file: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replacements: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inserted_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<bool>,
}

pub async fn file_edit(
    params: Parameters<EditParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;

    let mut futures = Vec::new();
    for op in params.operations {
        let file_type = params.file_type.clone();
        futures.push(edit_single_file(op, file_type, working_dir));
    }

    let results = futures::future::join_all(futures).await;

    let json = serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

async fn edit_single_file(
    op: FileEditOperation,
    file_type: Option<String>,
    working_dir: &Path,
) -> FileEditResult {
    let mode = op.mode.as_deref().unwrap_or("string_replace");
    let path = Path::new(&op.path);

    let canonical_path = match ensure_path_within_working_dir(path, working_dir) {
        Ok(p) => p,
        Err(e) => {
            return FileEditResult {
                file: op.path,
                success: false,
                error: Some(e),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            }
        }
    };

    let file_exists = canonical_path.exists() && canonical_path.is_file();

    // Detect office format
    let office_fmt = detect_office_format(&file_type, &canonical_path);

    if let Some(ref fmt) = office_fmt {
        if file_exists {
            // Route to office-specific or complex edit handler
            if mode.starts_with("office_") || mode.starts_with("pdf_") {
                return edit_complex_format(&canonical_path, fmt, &op, mode);
            } else {
                return edit_office_string(&canonical_path, fmt, &op, mode);
            }
        } else {
            return FileEditResult {
                file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!(
                    "Office file '{}' does not exist. Use the Write tool to create it.",
                    op.path
                )),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            };
        }
    }

    // Plain text files
    let can_create_new = matches!(mode, "string_replace" | "line_replace" | "insert");

    if !file_exists {
        if !can_create_new {
            return FileEditResult {
                file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!("File '{}' does not exist", op.path)),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            };
        }

        let new_content = match op.new_string.as_deref() {
            Some(s) => s,
            None => {
                return FileEditResult {
                    file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                    success: false,
                    error: Some(format!(
                        "new_string is required to create new file '{}'",
                        op.path
                    )),
                    mode: Some(mode.to_string()),
                    replacements: None,
                    lines: None,
                    inserted_lines: None,
                    deleted_lines: None,
                    preview: None,
                    total_lines: None,
                    created: None,
                }
            }
        };

        if let Some(parent) = canonical_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return FileEditResult {
                    file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                    success: false,
                    error: Some(format!(
                        "Failed to create parent directories for '{}': {}",
                        op.path, e
                    )),
                    mode: Some(mode.to_string()),
                    replacements: None,
                    lines: None,
                    inserted_lines: None,
                    deleted_lines: None,
                    preview: None,
                    total_lines: None,
                    created: None,
                };
            }
        }

        if let Err(e) = tokio::fs::write(&canonical_path, new_content).await {
            return FileEditResult {
                file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!("Failed to write file '{}': {}", op.path, e)),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            };
        }

        let total_lines = new_content.lines().count();
        let preview = build_preview(new_content, None);

        return FileEditResult {
            file: strip_unc_prefix(&canonical_path.to_string_lossy()),
            success: true,
            error: None,
            mode: Some(mode.to_string()),
            replacements: Some(0),
            lines: None,
            inserted_lines: Some(total_lines),
            deleted_lines: Some(0),
            preview: Some(preview),
            total_lines: Some(total_lines),
            created: Some(true),
        };
    }

    let content = match tokio::fs::read_to_string(&canonical_path).await {
        Ok(c) => c,
        Err(e) => {
            return FileEditResult {
                file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!(
                    "Failed to read file '{}': {}",
                    canonical_path.display(),
                    e
                )),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            }
        }
    };

    let result = match mode {
        "string_replace" => string_replace_mode(&content, &op, &canonical_path).await,
        "line_replace" => line_replace_mode(&content, &op, &canonical_path).await,
        "insert" => insert_mode(&content, &op, &canonical_path).await,
        "delete" => delete_mode(&content, &op, &canonical_path).await,
        "patch" => patch_mode(&content, &op, &canonical_path).await,
        _ => {
            return FileEditResult {
                file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!(
                    "Invalid edit mode '{}'. Use string_replace, line_replace, insert, delete, or patch.",
                    mode
                )),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            }
        }
    };

    match result {
        Ok(mut r) => {
            r.file = strip_unc_prefix(&canonical_path.to_string_lossy());
            r
        }
        Err(e) => FileEditResult {
            file: strip_unc_prefix(&canonical_path.to_string_lossy()),
            success: false,
            error: Some(e),
            mode: Some(mode.to_string()),
            replacements: None,
            lines: None,
            inserted_lines: None,
            deleted_lines: None,
            preview: None,
            total_lines: None,
            created: None,
        },
    }
}

fn detect_office_format(file_type: &Option<String>, path: &Path) -> Option<String> {
    if let Some(ref ft) = file_type {
        let ft_lower = ft.to_lowercase();
        if matches!(ft_lower.as_str(), "docx" | "doc" | "pptx" | "ppt" | "xlsx" | "xls" | "pdf") {
            return Some(ft_lower);
        }
    }

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        return match ext_lower.as_str() {
            "docx" | "doc" | "pptx" | "ppt" | "xlsx" | "xls" | "pdf" => {
                Some(ext_lower)
            }
            _ => None,
        };
    }

    None
}

// ============================================================================
// Complex format editing (office_* and pdf_* modes)
// ============================================================================

fn edit_complex_format(
    canonical_path: &Path,
    office_fmt: &str,
    op: &FileEditOperation,
    mode: &str,
) -> FileEditResult {
    let display_path = strip_unc_prefix(&canonical_path.to_string_lossy());

    match mode {
        "office_insert" | "office_replace" | "office_insert_table" | "office_insert_image" | "office_format" | "office_delete" => {
            match office_fmt {
                "docx" | "doc" => edit_docx_complex(canonical_path, op, mode),
                "pptx" | "ppt" => edit_pptx_complex(canonical_path, op, mode),
                _ => FileEditResult {
                    file: display_path,
                    success: false,
                    error: Some(format!(
                        "Complex office operations are only supported for DOCX and PPTX, not {}",
                        office_fmt
                    )),
                    mode: Some(mode.to_string()),
                    replacements: None,
                    lines: None,
                    inserted_lines: None,
                    deleted_lines: None,
                    preview: None,
                    total_lines: None,
                    created: None,
                },
            }
        }
        "pdf_delete_page" | "pdf_insert_image" | "pdf_insert_text" | "pdf_replace_text" | "pdf_merge" => {
            edit_pdf_complex(canonical_path, op, mode)
        }
        _ => FileEditResult {
            file: display_path,
            success: false,
            error: Some(format!("Unknown complex edit mode: {}", mode)),
            mode: Some(mode.to_string()),
            replacements: None,
            lines: None,
            inserted_lines: None,
            deleted_lines: None,
            preview: None,
            total_lines: None,
            created: None,
        },
    }
}

// ---- Simple string replace for office files ----

fn edit_office_string(
    canonical_path: &Path,
    office_fmt: &str,
    op: &FileEditOperation,
    mode: &str,
) -> FileEditResult {
    let display_path = strip_unc_prefix(&canonical_path.to_string_lossy());

    if mode != "string_replace" {
        return FileEditResult {
            file: display_path,
            success: false,
            error: Some(format!(
                "Edit mode '{}' is not supported for office documents. Use string_replace mode.",
                mode
            )),
            mode: Some(mode.to_string()),
            replacements: None,
            lines: None,
            inserted_lines: None,
            deleted_lines: None,
            preview: None,
            total_lines: None,
            created: None,
        };
    }

    let old_str = match op.old_string.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => {
            return FileEditResult {
                file: display_path,
                success: false,
                error: Some("old_string is required and must not be empty".to_string()),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            }
        }
    };

    let new_str = op.new_string.as_deref().unwrap_or("");

    let result = match office_fmt {
        "docx" | "doc" => edit_docx_string(canonical_path, old_str, new_str),
        "pptx" | "ppt" => edit_pptx_string(canonical_path, old_str, new_str),
        "xlsx" | "xls" => edit_xlsx_string(canonical_path, old_str, new_str),
        "pdf" => Err("PDF editing via string_replace is not supported. Use pdf_* modes.".to_string()),
        _ => Err(format!("Unsupported office format: {}", office_fmt)),
    };

    match result {
        Ok((message, count)) => FileEditResult {
            file: display_path,
            success: true,
            error: None,
            mode: Some("string_replace".to_string()),
            replacements: Some(count),
            lines: None,
            inserted_lines: None,
            deleted_lines: None,
            preview: Some(vec![message]),
            total_lines: None,
            created: Some(false),
        },
        Err(e) => FileEditResult {
            file: display_path,
            success: false,
            error: Some(e),
            mode: Some("string_replace".to_string()),
            replacements: None,
            lines: None,
            inserted_lines: None,
            deleted_lines: None,
            preview: None,
            total_lines: None,
            created: None,
        },
    }
}

// ---- DOCX string replace (docx-rs based) ----

fn edit_docx_string(path: &Path, old_str: &str, new_str: &str) -> Result<(String, usize), String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read DOCX file: {}", e))?;
    let mut docx = docx_rs::read_docx(&data).map_err(|e| format!("Failed to parse DOCX: {}", e))?;

    let mut replaced_count: usize = 0;

    for child in &mut docx.document.children {
        if let docx_rs::DocumentChild::Paragraph(para) = child {
            replaced_count += replace_paragraph_text(para, old_str, new_str);
        } else if let docx_rs::DocumentChild::Table(table) = child {
            replaced_count += replace_table_text(table, old_str, new_str);
        }
    }

    if replaced_count == 0 {
        return Err(format!(
            "Could not find '{}' in the DOCX document. Please verify the exact text.",
            old_str
        ));
    }

    let mut output = Vec::new();
    docx.build()
        .pack(&mut std::io::Cursor::new(&mut output))
        .map_err(|e| format!("Failed to build DOCX: {}", e))?;
    std::fs::write(path, &output).map_err(|e| format!("Failed to write DOCX: {}", e))?;

    Ok((format!(
        "Replaced {} occurrence(s) of '{}' -> '{}' in DOCX file.",
        replaced_count, old_str, new_str
    ), replaced_count))
}

fn replace_paragraph_text(para: &mut docx_rs::Paragraph, old: &str, new: &str) -> usize {
    let mut count = 0;
    for child in &mut para.children {
        match child {
            docx_rs::ParagraphChild::Run(run) => {
                count += replace_run_text(run, old, new);
            }
            docx_rs::ParagraphChild::Insert(ins) => {
                for ic in &mut ins.children {
                    if let docx_rs::InsertChild::Run(run) = ic {
                        count += replace_run_text(run, old, new);
                    }
                }
            }
            docx_rs::ParagraphChild::Hyperlink(hl) => {
                for hc in &mut hl.children {
                    if let docx_rs::ParagraphChild::Run(run) = hc {
                        count += replace_run_text(run, old, new);
                    }
                }
            }
            _ => {}
        }
    }
    count
}

fn replace_run_text(run: &mut docx_rs::Run, old: &str, new: &str) -> usize {
    let mut count = 0;
    for child in &mut run.children {
        if let docx_rs::RunChild::Text(ref mut t) = child {
            if t.text.contains(old) {
                t.text = t.text.replace(old, new);
                count += 1;
            }
        }
    }
    count
}

fn replace_table_text(table: &mut docx_rs::Table, old: &str, new: &str) -> usize {
    let mut count = 0;
    for row_child in &mut table.rows {
        let docx_rs::TableChild::TableRow(row) = row_child;
        for cell_child in &mut row.cells {
            let docx_rs::TableRowChild::TableCell(cell) = cell_child;
            for content in &mut cell.children {
                if let docx_rs::TableCellContent::Paragraph(para) = content {
                    count += replace_paragraph_text(para, old, new);
                }
            }
        }
    }
    count
}

// ---- PPTX string replace ----

fn edit_pptx_string(path: &Path, old_str: &str, new_str: &str) -> Result<(String, usize), String> {
    use ppt_rs::{Presentation, SlideContent};

    let pres = Presentation::from_path(path)
        .map_err(|e| format!("Failed to open PPTX file: {}", e))?;

    let mut replaced_count: usize = 0;
    let mut new_pres = Presentation::with_title(
        &pres.get_title().replace(old_str, new_str),
    );
    if pres.get_title().contains(old_str) {
        replaced_count += 1;
    }

    for slide in pres.slides() {
        let new_title = slide.title.replace(old_str, new_str);
        if new_title != slide.title {
            replaced_count += 1;
        }

        let mut new_slide = SlideContent::new(&new_title);
        new_slide.shapes = slide.shapes.clone();
        for shape in new_slide.shapes.iter_mut() {
            if let Some(ref mut text) = shape.text {
                if text.contains(old_str) {
                    *text = text.replace(old_str, new_str);
                    replaced_count += 1;
                }
            }
        }
        for bullet in &slide.content {
            let new_bullet = bullet.replace(old_str, new_str);
            if new_bullet != *bullet {
                replaced_count += 1;
            }
            new_slide = new_slide.add_bullet(&new_bullet);
        }

        new_pres = new_pres.add_slide(new_slide);
    }

    new_pres
        .save(path)
        .map_err(|e| format!("Failed to save modified PPTX: {}", e))?;

    Ok((format!(
        "Replaced {} occurrence(s) of '{}' -> '{}' in PPTX file.",
        replaced_count, old_str, new_str
    ), replaced_count))
}

// ---- XLSX/XLS string replace ----

fn edit_xlsx_string(path: &Path, old_str: &str, new_str: &str) -> Result<(String, usize), String> {
    use std::io::Cursor;

    use calamine::{open_workbook_auto_from_rs, Data, Reader};
    use rust_xlsxwriter::Workbook;

    let data =
        std::fs::read(path).map_err(|e| format!("Failed to read XLSX file: {}", e))?;
    let cursor = Cursor::new(data);
    let mut wb =
        open_workbook_auto_from_rs(cursor).map_err(|e| format!("Failed to parse XLSX/XLS: {}", e))?;

    let sheet_names = wb.sheet_names();
    let mut all_sheets_data: Vec<(String, Vec<Vec<String>>)> = Vec::new();
    let mut replaced_count: usize = 0;

    for name in &sheet_names {
        let range = wb
            .worksheet_range(name)
            .map_err(|e| format!("Sheet '{}' read error: {}", name, e))?;

        let mut rows_data: Vec<Vec<String>> = Vec::new();
        for row in range.rows() {
            let cells: Vec<String> = row
                .iter()
                .map(|cell| match cell {
                    Data::Empty => String::new(),
                    Data::String(s) => s.clone(),
                    Data::Float(f) => f.to_string(),
                    Data::Int(i) => i.to_string(),
                    Data::Bool(b) => b.to_string(),
                    Data::Error(e) => format!("#ERR:{}", e),
                    Data::DateTime(dt) => dt.to_string(),
                    Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
                })
                .collect();
            rows_data.push(cells);
        }
        all_sheets_data.push((name.clone(), rows_data));
    }

    for (_name, rows) in &mut all_sheets_data {
        for row in rows.iter_mut() {
            for cell in row.iter_mut() {
                if cell.contains(old_str) {
                    let count_before = cell.matches(old_str).count();
                    *cell = cell.replace(old_str, new_str);
                    replaced_count += count_before;
                }
            }
        }
    }

    let mut workbook = Workbook::new();
    for (name, rows) in &all_sheets_data {
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(name)
            .map_err(|e| format!("Failed to set sheet name: {}", e))?;
        for (row_idx, row) in rows.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                if !cell.is_empty() {
                    worksheet
                        .write(row_idx as u32, col_idx as u16, cell.as_str())
                        .map_err(|e| format!("Failed to write cell: {}", e))?;
                }
            }
        }
    }
    workbook
        .save(path)
        .map_err(|e| format!("Failed to save modified XLSX: {}", e))?;

    Ok((format!(
        "Replaced {} occurrence(s) of '{}' -> '{}' in spreadsheet file.",
        replaced_count, old_str, new_str
    ), replaced_count))
}

// ============================================================================
// Complex DOCX editing
// ============================================================================

fn edit_docx_complex(
    canonical_path: &Path,
    op: &FileEditOperation,
    mode: &str,
) -> FileEditResult {
    let display_path = strip_unc_prefix(&canonical_path.to_string_lossy());

    match mode {
        "office_insert" => edit_docx_insert(canonical_path, &display_path, op),
        "office_replace" => edit_docx_replace(canonical_path, &display_path, op),
        "office_delete" => edit_docx_delete(canonical_path, &display_path, op),
        "office_insert_image" => edit_docx_insert_image(canonical_path, &display_path, op),
        "office_format" => edit_docx_format(canonical_path, &display_path, op),
        "office_insert_table" => edit_docx_insert_table(canonical_path, &display_path, op),
        _ => FileEditResult {
            file: display_path,
            success: false,
            error: Some(format!("Unknown DOCX edit mode: {}", mode)),
            mode: Some(mode.to_string()),
            replacements: None,
            lines: None,
            inserted_lines: None,
            deleted_lines: None,
            preview: None,
            total_lines: None,
            created: None,
        },
    }
}

fn edit_docx_insert(
    path: &Path,
    display_path: &str,
    op: &FileEditOperation,
) -> FileEditResult {
    let markdown = match op.markdown.as_deref() {
        Some(m) => m,
        None => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some("markdown parameter is required for office_insert mode".to_string()),
                mode: Some("office_insert".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let find_text = op.find_text.as_deref().unwrap_or("");
    let location = op.location.as_deref().unwrap_or("after");

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to read DOCX: {}", e)),
                mode: Some("office_insert".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let mut docx = match docx_rs::read_docx(&data) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to parse DOCX: {}", e)),
                mode: Some("office_insert".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };
    let new_children: Vec<docx_rs::DocumentChild> =
        markdown_to_docx_children(markdown);

    // Handle insert positioning: find target element by text, insert before/after/at end
    if !find_text.is_empty() {
        let children = &mut docx.document.children;
        let target_idx = children.iter().position(|child| {
            if let docx_rs::DocumentChild::Paragraph(para) = child {
                let para_text = para.children.iter()
                    .filter_map(|pc| {
                        if let docx_rs::ParagraphChild::Run(run) = pc {
                            Some(run.children.iter().filter_map(|rc| {
                                if let docx_rs::RunChild::Text(t) = rc {
                                    Some(t.text.as_str())
                                } else {
                                    None
                                }
                            }).collect::<String>())
                        } else {
                            None
                        }
                    })
                    .collect::<String>();
                para_text.contains(find_text)
            } else {
                false
            }
        });
        let insert_idx = match location {
            "before" => target_idx.unwrap_or(children.len()),
            "after" => target_idx.map(|i| i + 1).unwrap_or(children.len()),
            _ => children.len(),
        };
        docx.document.children.splice(
            insert_idx..insert_idx,
            new_children,
        );
    } else {
        docx.document.children.extend(new_children);
    }

    let mut output = Vec::new();
    if let Err(e) = docx.build().pack(&mut std::io::Cursor::new(&mut output)) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to build DOCX: {}", e)),
            mode: Some("office_insert".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }
    if let Err(e) = std::fs::write(path, &output) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to write DOCX: {}", e)),
            mode: Some("office_insert".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }

    FileEditResult {
        file: display_path.to_string(),
        success: true,
        error: None,
        mode: Some("office_insert".to_string()),
        replacements: Some(0),
        lines: None,
        inserted_lines: None,
        deleted_lines: None,
        preview: Some(vec![format!(
            "Inserted markdown content at end of DOCX document."
        )]),
        total_lines: None,
        created: Some(false),
    }
}

fn edit_docx_replace(
    path: &Path,
    display_path: &str,
    op: &FileEditOperation,
) -> FileEditResult {
    let find_text = match op.find_text.as_deref() {
        Some(t) if !t.is_empty() => t,
        _ => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some("find_text is required for office_replace mode".to_string()),
                mode: Some("office_replace".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let new_text = op.new_string.as_deref().unwrap_or("");

    match edit_docx_string(path, find_text, new_text) {
        Ok((msg, count)) => FileEditResult {
            file: display_path.to_string(),
            success: true,
            error: None,
            mode: Some("office_replace".to_string()),
            replacements: Some(count),
            lines: None,
            inserted_lines: None,
            deleted_lines: None,
            preview: Some(vec![msg]),
            total_lines: None,
            created: Some(false),
        },
        Err(e) => FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(e),
            mode: Some("office_replace".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        },
    }
}

fn edit_docx_delete(
    path: &Path,
    display_path: &str,
    op: &FileEditOperation,
) -> FileEditResult {
    if let Some(ref find_text) = op.find_text {
        if let Some(ref el_type) = op.element_type {
            if el_type == "table" {
                return edit_docx_delete_table(path, display_path, find_text);
            }
        }
    }

    let find_text = match op.find_text.as_deref() {
        Some(t) if !t.is_empty() => t,
        _ => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some("find_text is required for office_delete mode".to_string()),
                mode: Some("office_delete".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    match edit_docx_string(path, find_text, "") {
        Ok((msg, count)) => FileEditResult {
            file: display_path.to_string(),
            success: true,
            error: None,
            mode: Some("office_delete".to_string()),
            replacements: Some(count),
            lines: None, inserted_lines: None, deleted_lines: None,
            preview: Some(vec![msg]),
            total_lines: None, created: Some(false),
        },
        Err(e) => FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(e),
            mode: Some("office_delete".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        },
    }
}

fn edit_docx_delete_table(
    path: &Path,
    display_path: &str,
    find_text: &str,
) -> FileEditResult {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to read DOCX: {}", e)),
                mode: Some("office_delete".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let mut docx = match docx_rs::read_docx(&data) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to parse DOCX: {}", e)),
                mode: Some("office_delete".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let mut found = false;
    docx.document.children.retain(|child| {
        if let docx_rs::DocumentChild::Table(table) = child {
            let table_text = collect_table_text(table);
            if table_text.contains(find_text) {
                found = true;
                return false;
            }
        }
        true
    });

    if !found {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("No table containing '{}' found in the document.", find_text)),
            mode: Some("office_delete".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }

    let mut output = Vec::new();
    if let Err(e) = docx.build().pack(&mut std::io::Cursor::new(&mut output)) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to build DOCX: {}", e)),
            mode: Some("office_delete".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }
    if let Err(e) = std::fs::write(path, &output) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to write DOCX: {}", e)),
            mode: Some("office_delete".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }

    FileEditResult {
        file: display_path.to_string(),
        success: true,
        error: None,
        mode: Some("office_delete".to_string()),
        replacements: Some(1),
        lines: None, inserted_lines: None, deleted_lines: Some(1),
        preview: Some(vec!["Deleted table from DOCX document.".to_string()]),
        total_lines: None, created: Some(false),
    }
}

fn collect_table_text(table: &docx_rs::Table) -> String {
    let mut text = String::new();
    for row_child in &table.rows {
        let docx_rs::TableChild::TableRow(row) = row_child;
        for cell_child in &row.cells {
            let docx_rs::TableRowChild::TableCell(cell) = cell_child;
            for content in &cell.children {
                if let docx_rs::TableCellContent::Paragraph(para) = content {
                    for child in &para.children {
                        if let docx_rs::ParagraphChild::Run(run) = child {
                            for rc in &run.children {
                                if let docx_rs::RunChild::Text(t) = rc {
                                    text.push_str(&t.text);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    text
}

fn edit_docx_insert_image(
    path: &Path,
    display_path: &str,
    op: &FileEditOperation,
) -> FileEditResult {
    let img_path = match op.image_path.as_deref() {
        Some(p) => p,
        None => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some("image_path is required for office_insert_image mode".to_string()),
                mode: Some("office_insert_image".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let img_data = match std::fs::read(img_path) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to read image file '{}': {}", img_path, e)),
                mode: Some("office_insert_image".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to read DOCX: {}", e)),
                mode: Some("office_insert_image".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let mut docx = match docx_rs::read_docx(&data) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to parse DOCX: {}", e)),
                mode: Some("office_insert_image".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let pic = docx_rs::Pic::new(&img_data);

    if let Some(find_text) = op.find_text.as_deref() {
        let location = op.location.as_deref().unwrap_or("after");
        let mut insert_idx = None;
        for (i, child) in docx.document.children.iter().enumerate() {
            if let docx_rs::DocumentChild::Paragraph(para) = child {
                let text = collect_paragraph_text(para);
                if text.contains(find_text) {
                    insert_idx = if location == "before" { Some(i) } else { Some(i + 1) };
                    break;
                }
            }
        }

        let img_para = docx_rs::Paragraph::new().add_run(
            docx_rs::Run::new().add_image(pic),
        );
        if let Some(idx) = insert_idx {
            docx.document.children.insert(idx, docx_rs::DocumentChild::Paragraph(Box::new(img_para)));
        } else {
            docx.document.children.push(docx_rs::DocumentChild::Paragraph(Box::new(img_para)));
        }
    } else {
        let img_para = docx_rs::Paragraph::new().add_run(
            docx_rs::Run::new().add_image(pic),
        );
        docx.document.children.push(docx_rs::DocumentChild::Paragraph(Box::new(img_para)));
    }

    let mut output = Vec::new();
    if let Err(e) = docx.build().pack(&mut std::io::Cursor::new(&mut output)) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to build DOCX: {}", e)),
            mode: Some("office_insert_image".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }
    if let Err(e) = std::fs::write(path, &output) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to write DOCX: {}", e)),
            mode: Some("office_insert_image".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }

    FileEditResult {
        file: display_path.to_string(),
        success: true,
        error: None,
        mode: Some("office_insert_image".to_string()),
        replacements: Some(0),
        lines: None, inserted_lines: None, deleted_lines: None,
        preview: Some(vec![format!("Inserted image from '{}' into DOCX.", img_path)]),
        total_lines: None, created: Some(false),
    }
}

fn edit_docx_format(
    path: &Path,
    display_path: &str,
    op: &FileEditOperation,
) -> FileEditResult {
    let find_text = match op.find_text.as_deref() {
        Some(t) if !t.is_empty() => t,
        _ => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some("find_text is required for office_format mode".to_string()),
                mode: Some("office_format".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let format_type = match op.format_type.as_deref() {
        Some(f) => f,
        None => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some("format_type is required. Options: bold, italic, heading1..heading6, strikethrough".to_string()),
                mode: Some("office_format".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to read DOCX: {}", e)),
                mode: Some("office_format".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let mut docx = match docx_rs::read_docx(&data) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to parse DOCX: {}", e)),
                mode: Some("office_format".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let occurrence = op.occurrence.unwrap_or(1);
    apply_format_to_docx(&mut docx, find_text, format_type, occurrence);

    let mut output = Vec::new();
    if let Err(e) = docx.build().pack(&mut std::io::Cursor::new(&mut output)) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to build DOCX: {}", e)),
            mode: Some("office_format".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }
    if let Err(e) = std::fs::write(path, &output) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to write DOCX: {}", e)),
            mode: Some("office_format".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }

    FileEditResult {
        file: display_path.to_string(),
        success: true,
        error: None,
        mode: Some("office_format".to_string()),
        replacements: Some(1),
        lines: None, inserted_lines: None, deleted_lines: None,
        preview: Some(vec![format!(
            "Applied format '{}' to text '{}'.", format_type, find_text
        )]),
        total_lines: None, created: Some(false),
    }
}

fn apply_format_to_docx(
    docx: &mut docx_rs::Docx,
    find_text: &str,
    format_type: &str,
    occurrence: usize,
) {
    let mut found = 0;
    for child in &mut docx.document.children {
        if let docx_rs::DocumentChild::Paragraph(ref mut para) = child {
            if found >= occurrence && occurrence != 0 {
                break;
            }
            found += apply_format_to_paragraph(para, find_text, format_type, occurrence - found);
        }
    }

    if format_type.starts_with("heading") {
        let level: usize = format_type
            .trim_start_matches("heading")
            .parse()
            .unwrap_or(1);
        let style_id = format!("Heading{}", level);

        let mut found_heading = 0;
        for child in &mut docx.document.children {
            if let docx_rs::DocumentChild::Paragraph(ref mut para) = child {
                let text = collect_paragraph_text(para);
                if text.contains(find_text) {
                    found_heading += 1;
                    if occurrence == 0 || found_heading == occurrence {
                        para.property = para.property.clone().style(&style_id);
                        if occurrence != 0 {
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn apply_format_to_paragraph(
    para: &mut docx_rs::Paragraph,
    find_text: &str,
    format_type: &str,
    _occurrence: usize,
) -> usize {
    let mut found = 0;
    for child in &mut para.children {
        if let docx_rs::ParagraphChild::Run(ref mut run) = child {
            for rc in &mut run.children {
                if let docx_rs::RunChild::Text(ref t) = rc {
                    if t.text.contains(find_text) {
                        match format_type {
                            "bold" => run.run_property = run.run_property.clone().bold(),
                            "italic" => run.run_property = run.run_property.clone().italic(),
                            "strikethrough" => run.run_property = run.run_property.clone().strike(),
                            _ => {}
                        }
                        found += 1;
                    }
                }
            }
        }
    }
    found
}

fn edit_docx_insert_table(
    path: &Path,
    display_path: &str,
    op: &FileEditOperation,
) -> FileEditResult {
    let markdown = match op.markdown.as_deref() {
        Some(m) => m,
        None => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some("markdown parameter is required for office_insert_table mode".to_string()),
                mode: Some("office_insert_table".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to read DOCX: {}", e)),
                mode: Some("office_insert_table".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let mut docx = match docx_rs::read_docx(&data) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to parse DOCX: {}", e)),
                mode: Some("office_insert_table".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let table = markdown_table_to_docx(markdown);

    if let Some(find_text) = op.find_text.as_deref() {
        let location = op.location.as_deref().unwrap_or("after");
        let mut insert_idx = None;
        for (i, child) in docx.document.children.iter().enumerate() {
            if let docx_rs::DocumentChild::Paragraph(para) = child {
                let text = collect_paragraph_text(para);
                if text.contains(find_text) {
                    insert_idx = if location == "before" { Some(i) } else { Some(i + 1) };
                    break;
                }
            }
        }
        if let Some(idx) = insert_idx {
            docx.document.children.insert(idx, docx_rs::DocumentChild::Table(Box::new(table)));
        } else {
            docx.document.children.push(docx_rs::DocumentChild::Table(Box::new(table)));
        }
    } else {
        docx.document.children.push(docx_rs::DocumentChild::Table(Box::new(table)));
    }

    let mut output = Vec::new();
    if let Err(e) = docx.build().pack(&mut std::io::Cursor::new(&mut output)) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to build DOCX: {}", e)),
            mode: Some("office_insert_table".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }
    if let Err(e) = std::fs::write(path, &output) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to write DOCX: {}", e)),
            mode: Some("office_insert_table".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }

    FileEditResult {
        file: display_path.to_string(),
        success: true,
        error: None,
        mode: Some("office_insert_table".to_string()),
        replacements: Some(0),
        lines: None, inserted_lines: None, deleted_lines: None,
        preview: Some(vec!["Inserted table into DOCX.".to_string()]),
        total_lines: None, created: Some(false),
    }
}

fn markdown_to_docx_children(markdown: &str) -> Vec<docx_rs::DocumentChild> {
    let mut children = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("|") && trimmed.ends_with("|") {
            continue;
        }
        if trimmed.starts_with("|---") {
            continue;
        }

        let para = if let Some(text) = trimmed.strip_prefix("# ") {
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text(text).bold().size(32))
                .style("Heading1")
        } else if let Some(text) = trimmed.strip_prefix("## ") {
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text(text).bold().size(28))
                .style("Heading2")
        } else {
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text(trimmed))
        };

        children.push(docx_rs::DocumentChild::Paragraph(Box::new(para)));
    }
    children
}

fn markdown_table_to_docx(markdown: &str) -> docx_rs::Table {
    let mut rows_data: Vec<Vec<String>> = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("|---") || trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            let cells: Vec<String> = trimmed
                .split('|')
                .skip(1)
                .filter(|s| !s.is_empty() && *s != "---" && !s.trim().starts_with("---"))
                .map(|s| s.trim().to_string())
                .collect();
            if !cells.is_empty() {
                rows_data.push(cells);
            }
        }
    }

    let mut table_rows: Vec<docx_rs::TableRow> = Vec::new();
    for row_data in rows_data {
        let cells: Vec<docx_rs::TableCell> = row_data
            .into_iter()
            .map(|text| {
                docx_rs::TableCell::new().add_paragraph(
                    docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_text(text)),
                )
            })
            .collect();
        table_rows.push(docx_rs::TableRow::new(cells));
    }

    docx_rs::Table::new(table_rows).set_grid(vec![2000])
}

fn collect_paragraph_text(para: &docx_rs::Paragraph) -> String {
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

// ---- PPTX complex editing stub ----

fn edit_pptx_complex(
    path: &Path,
    op: &FileEditOperation,
    mode: &str,
) -> FileEditResult {
    let display_path = strip_unc_prefix(&path.to_string_lossy());

    if mode == "office_insert_image" {
        let _img_path = match op.image_path.as_deref() {
            Some(p) => p,
            None => {
                return FileEditResult {
                    file: display_path,
                    success: false,
                    error: Some("image_path is required".to_string()),
                    mode: Some(mode.to_string()),
                    replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                    preview: None, total_lines: None, created: None,
                }
            }
        };

        let _slide_idx = op.slide_index.unwrap_or(0);

    return FileEditResult {
            file: display_path,
            success: false,
            error: Some("PPTX image insertion is not yet implemented.".to_string()),
            mode: Some(mode.to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        };
    }

    FileEditResult {
        file: display_path,
        success: false,
        error: Some(format!("Complex PPTX editing mode '{}' is not yet implemented.", mode)),
        mode: Some(mode.to_string()),
        replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
        preview: None, total_lines: None, created: None,
    }
}

// ============================================================================
// PDF editing via lopdf
// ============================================================================

fn edit_pdf_complex(
    canonical_path: &Path,
    op: &FileEditOperation,
    mode: &str,
) -> FileEditResult {
    let display_path = strip_unc_prefix(&canonical_path.to_string_lossy());

    match mode {
        "pdf_delete_page" => edit_pdf_delete_page(canonical_path, &display_path, op),
        "pdf_insert_image" => edit_pdf_insert_image(canonical_path, &display_path, op),
        "pdf_insert_text" => edit_pdf_insert_text(canonical_path, &display_path, op),
        "pdf_replace_text" => edit_pdf_replace_text(canonical_path, &display_path, op),
        "pdf_merge" => FileEditResult {
            file: display_path,
            success: false,
            error: Some("pdf_merge is not yet implemented.".to_string()),
            mode: Some("pdf_merge".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        },
        _ => FileEditResult {
            file: display_path,
            success: false,
            error: Some(format!("Unknown PDF edit mode: {}", mode)),
            mode: Some(mode.to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        },
    }
}

fn edit_pdf_delete_page(
    path: &Path,
    display_path: &str,
    op: &FileEditOperation,
) -> FileEditResult {
    let page_idx = op.page_index.unwrap_or(0);

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to read PDF: {}", e)),
                mode: Some("pdf_delete_page".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let mut doc = match lopdf::Document::load_mem(&data) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to parse PDF: {}", e)),
                mode: Some("pdf_delete_page".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let pages_before = doc.get_pages().len();
    let page_nums: Vec<u32> = vec![(page_idx + 1) as u32];
    doc.delete_pages(&page_nums);

    let mut output = Vec::new();
    if let Err(e) = doc.save_to(&mut output) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to save PDF: {}", e)),
            mode: Some("pdf_delete_page".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }
    if let Err(e) = std::fs::write(path, &output) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to write PDF: {}", e)),
            mode: Some("pdf_delete_page".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }

    FileEditResult {
        file: display_path.to_string(),
        success: true,
        error: None,
        mode: Some("pdf_delete_page".to_string()),
        replacements: Some(0),
        lines: None, inserted_lines: Some(0), deleted_lines: Some(1),
        preview: Some(vec![format!(
            "Deleted page {} from PDF ({} pages before).",
            page_idx + 1,
            pages_before,
        )]),
        total_lines: None, created: Some(false),
    }
}

fn edit_pdf_insert_image(
    _path: &Path,
    display_path: &str,
    _op: &FileEditOperation,
) -> FileEditResult {
    FileEditResult {
        file: display_path.to_string(),
        success: false,
        error: Some("PDF image insertion requires the lopdf 'embed_image' feature with compatible image crate.".to_string()),
        mode: Some("pdf_insert_image".to_string()),
        replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
        preview: None, total_lines: None, created: None,
    }
}

fn edit_pdf_insert_text(
    path: &Path,
    display_path: &str,
    op: &FileEditOperation,
) -> FileEditResult {
    let text = match op.new_string.as_deref() {
        Some(t) => t,
        None => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some("new_string is required".to_string()),
                mode: Some("pdf_insert_text".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let page_idx = op.page_index.unwrap_or(0);

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to read PDF: {}", e)),
                mode: Some("pdf_insert_text".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let mut doc = match lopdf::Document::load_mem(&data) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to parse PDF: {}", e)),
                mode: Some("pdf_insert_text".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let pages = doc.get_pages();
    let page_ids: Vec<&u32> = pages.keys().collect();
    if page_idx >= page_ids.len() {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Page index {} out of range", page_idx)),
            mode: Some("pdf_insert_text".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }

    let page_id = *page_ids[page_idx];

    let content = format!(
        "BT /F1 12 Tf 72 700 Td ({}) Tj ET",
        text.replace('(', "\\(").replace(')', "\\)")
    );

    if let Err(e) = doc.add_page_contents((page_id, 0), content.as_bytes().to_vec()) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to add text content: {}", e)),
            mode: Some("pdf_insert_text".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }

    let mut output = Vec::new();
    if let Err(e) = doc.save_to(&mut output) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to save PDF: {}", e)),
            mode: Some("pdf_insert_text".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }
    if let Err(e) = std::fs::write(path, &output) {
        return FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to write PDF: {}", e)),
            mode: Some("pdf_insert_text".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        }
    }

    FileEditResult {
        file: display_path.to_string(),
        success: true,
        error: None,
        mode: Some("pdf_insert_text".to_string()),
        replacements: Some(0),
        lines: None, inserted_lines: Some(1), deleted_lines: Some(0),
        preview: Some(vec![format!(
            "Inserted text into page {} of PDF.", page_idx + 1
        )]),
        total_lines: None, created: Some(false),
    }
}

fn edit_pdf_replace_text(
    path: &Path,
    display_path: &str,
    op: &FileEditOperation,
) -> FileEditResult {
    let find_text = match op.old_string.as_deref() {
        Some(t) if !t.is_empty() => t,
        _ => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some("old_string is required for pdf_replace_text".to_string()),
                mode: Some("pdf_replace_text".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let replace_with = op.new_string.as_deref().unwrap_or("");

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to read PDF: {}", e)),
                mode: Some("pdf_replace_text".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let mut doc = match lopdf::Document::load_mem(&data) {
        Ok(d) => d,
        Err(e) => {
            return FileEditResult {
                file: display_path.to_string(),
                success: false,
                error: Some(format!("Failed to parse PDF: {}", e)),
                mode: Some("pdf_replace_text".to_string()),
                replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                preview: None, total_lines: None, created: None,
            }
        }
    };

    let page_id: u32 = op.page_index.unwrap_or(0) as u32 + 1;
    match doc.replace_partial_text(page_id, find_text, replace_with, None) {
        Ok(count) => {
            let mut output = Vec::new();
            if let Err(e) = doc.save_to(&mut output) {
                return FileEditResult {
                    file: display_path.to_string(),
                    success: false,
                    error: Some(format!("Failed to save PDF: {}", e)),
                    mode: Some("pdf_replace_text".to_string()),
                    replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                    preview: None, total_lines: None, created: None,
                }
            }
            if let Err(e) = std::fs::write(path, &output) {
                return FileEditResult {
                    file: display_path.to_string(),
                    success: false,
                    error: Some(format!("Failed to write PDF: {}", e)),
                    mode: Some("pdf_replace_text".to_string()),
                    replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
                    preview: None, total_lines: None, created: None,
                }
            }
            FileEditResult {
                file: display_path.to_string(),
                success: true,
                error: None,
                mode: Some("pdf_replace_text".to_string()),
                replacements: Some(count),
                lines: None, inserted_lines: None, deleted_lines: None,
                preview: Some(vec![format!(
                    "Replaced {} occurrence(s) of '{}' -> '{}' in PDF page {}.",
                    count, find_text, replace_with, page_id
                )]),
                total_lines: None, created: Some(false),
            }
        }
        Err(e) => FileEditResult {
            file: display_path.to_string(),
            success: false,
            error: Some(format!("Failed to replace text: {}", e)),
            mode: Some("pdf_replace_text".to_string()),
            replacements: None, lines: None, inserted_lines: None, deleted_lines: None,
            preview: None, total_lines: None, created: None,
        },
    }
}

// ============================================================================
// String replace mode
// ============================================================================
async fn string_replace_mode(
    content: &str,
    op: &FileEditOperation,
    canonical_path: &Path,
) -> Result<FileEditResult, String> {
    let old = op
        .old_string
        .as_deref()
        .ok_or("old_string is required for string_replace mode")?;
    let new = op
        .new_string
        .as_deref()
        .unwrap_or("");
    let occurrence = op.occurrence.unwrap_or(1);

    if old.is_empty() {
        return Err("old_string cannot be empty".to_string());
    }

    let mut occurrences: Vec<usize> = Vec::new();
    let mut search_start = 0;
    let mut current_line = 1;
    let mut last_pos = 0;
    while let Some(pos) = content[search_start..].find(old) {
        let absolute_pos = search_start + pos;
        for ch in content[last_pos..absolute_pos].chars() {
            if ch == '\n' {
                current_line += 1;
            }
        }
        occurrences.push(current_line);
        search_start = absolute_pos + old.len();
        last_pos = absolute_pos;
        if search_start >= content.len() {
            break;
        }
    }

    if occurrences.is_empty() {
        return Err(format!(
            "Could not find the specified old_string in '{}'. Please verify the exact text you want to replace.",
            canonical_path.display()
        ));
    }

    let mut replaced_lines: Vec<usize> = Vec::new();

    let replaced_content = if occurrence == 0 {
        replaced_lines = occurrences.clone();
        content.replace(old, new)
    } else {
        if occurrence > occurrences.len() {
            return Err(format!(
                "Requested occurrence {} but only {} occurrence(s) found at line(s): {:?}",
                occurrence,
                occurrences.len(),
                occurrences
            ));
        }
        let target_line = occurrences[occurrence - 1];
        let mut count = 0;
        let mut result = String::new();
        let mut search_start = 0;
        while let Some(pos) = content[search_start..].find(old) {
            let absolute_pos = search_start + pos;
            count += 1;
            result.push_str(&content[search_start..absolute_pos]);
            if count == occurrence {
                result.push_str(new);
                replaced_lines.push(target_line);
            } else {
                result.push_str(old);
            }
            search_start = absolute_pos + old.len();
        }
        result.push_str(&content[search_start..]);
        result
    };

    tokio::fs::write(&canonical_path, &replaced_content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", canonical_path.display(), e))?;

    let preview = build_preview(&replaced_content, replaced_lines.first().copied());
    let total_lines = replaced_content.lines().count();

    Ok(FileEditResult {
        file: canonical_path.to_string_lossy().to_string(),
        success: true,
        error: None,
        mode: Some("string_replace".to_string()),
        replacements: Some(replaced_lines.len()),
        lines: Some(replaced_lines),
        inserted_lines: None,
        deleted_lines: None,
        preview: Some(preview),
        total_lines: Some(total_lines),
        created: Some(false),
    })
}

// ============================================================================
// Line replace mode
// ============================================================================
async fn line_replace_mode(
    content: &str,
    op: &FileEditOperation,
    canonical_path: &Path,
) -> Result<FileEditResult, String> {
    let start_line = op
        .start_line
        .ok_or("start_line is required for line_replace mode")?;
    let end_line = op
        .end_line
        .ok_or("end_line is required for line_replace mode")?;
    let new_content = op
        .new_string
        .as_deref()
        .ok_or("new_string is required for line_replace mode")?;

    if start_line == 0 || end_line == 0 {
        return Err("Line numbers are 1-based and must be >= 1".to_string());
    }
    if start_line > end_line {
        return Err("start_line must be <= end_line".to_string());
    }

    let line_ending = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = content.lines().collect();
    let total_lines_before = lines.len();

    if start_line > total_lines_before {
        return Err(format!(
            "start_line {} is beyond file length ({} lines)",
            start_line, total_lines_before
        ));
    }

    let end_line = end_line.min(total_lines_before);
    let start_idx = start_line - 1;
    let end_idx = end_line;

    let mut result_lines: Vec<&str> = Vec::new();
    result_lines.extend_from_slice(&lines[..start_idx]);

    let new_lines: Vec<&str> = new_content.lines().collect();
    for nl in &new_lines {
        result_lines.push(nl);
    }

    result_lines.extend_from_slice(&lines[end_idx..]);

    let mut replaced_content = result_lines.join(line_ending);
    if content.ends_with('\n') && !replaced_content.ends_with('\n') && !replaced_content.is_empty() {
        replaced_content.push_str(line_ending);
    }

    tokio::fs::write(&canonical_path, &replaced_content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", canonical_path.display(), e))?;

    let preview = build_preview(&replaced_content, Some(start_line));
    let total_lines = replaced_content.lines().count();

    Ok(FileEditResult {
        file: canonical_path.to_string_lossy().to_string(),
        success: true,
        error: None,
        mode: Some("line_replace".to_string()),
        replacements: Some(1),
        lines: Some((start_line..=end_line).collect()),
        inserted_lines: Some(new_lines.len()),
        deleted_lines: Some(end_line - start_line + 1),
        preview: Some(preview),
        total_lines: Some(total_lines),
        created: Some(false),
    })
}

// ============================================================================
// Insert mode
// ============================================================================
async fn insert_mode(
    content: &str,
    op: &FileEditOperation,
    canonical_path: &Path,
) -> Result<FileEditResult, String> {
    let start_line = op
        .start_line
        .ok_or("start_line is required for insert mode")?;
    let new_content = op
        .new_string
        .as_deref()
        .ok_or("new_string is required for insert mode")?;

    if start_line == 0 {
        return Err("start_line must be >= 1 (1-based)".to_string());
    }

    let line_ending = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = content.lines().collect();
    let total_lines_before = lines.len();

    let insert_idx = if start_line > total_lines_before {
        total_lines_before
    } else {
        start_line - 1
    };

    let mut result_lines: Vec<&str> = Vec::new();
    result_lines.extend_from_slice(&lines[..insert_idx]);

    let new_lines: Vec<&str> = new_content.lines().collect();
    for nl in &new_lines {
        result_lines.push(nl);
    }

    result_lines.extend_from_slice(&lines[insert_idx..]);

    let mut replaced_content = result_lines.join(line_ending);
    if content.ends_with('\n') && !replaced_content.ends_with('\n') && !replaced_content.is_empty() {
        replaced_content.push_str(line_ending);
    }

    tokio::fs::write(&canonical_path, &replaced_content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", canonical_path.display(), e))?;

    let preview = build_preview(&replaced_content, Some(start_line));
    let total_lines = replaced_content.lines().count();

    Ok(FileEditResult {
        file: canonical_path.to_string_lossy().to_string(),
        success: true,
        error: None,
        mode: Some("insert".to_string()),
        replacements: Some(0),
        lines: Some(vec![start_line]),
        inserted_lines: Some(new_lines.len()),
        deleted_lines: Some(0),
        preview: Some(preview),
        total_lines: Some(total_lines),
        created: Some(false),
    })
}

// ============================================================================
// Delete mode
// ============================================================================
async fn delete_mode(
    content: &str,
    op: &FileEditOperation,
    canonical_path: &Path,
) -> Result<FileEditResult, String> {
    let start_line = op
        .start_line
        .ok_or("start_line is required for delete mode")?;
    let end_line = op
        .end_line
        .ok_or("end_line is required for delete mode")?;

    if start_line == 0 || end_line == 0 {
        return Err("Line numbers are 1-based and must be >= 1".to_string());
    }
    if start_line > end_line {
        return Err("start_line must be <= end_line".to_string());
    }

    let line_ending = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = content.lines().collect();
    let total_lines_before = lines.len();

    if start_line > total_lines_before {
        return Err(format!(
            "start_line {} is beyond file length ({} lines)",
            start_line, total_lines_before
        ));
    }

    let end_line = end_line.min(total_lines_before);
    let start_idx = start_line - 1;
    let end_idx = end_line;

    let mut result_lines: Vec<&str> = Vec::new();
    result_lines.extend_from_slice(&lines[..start_idx]);
    result_lines.extend_from_slice(&lines[end_idx..]);

    let mut replaced_content = result_lines.join(line_ending);
    if content.ends_with('\n') && !replaced_content.ends_with('\n') && !replaced_content.is_empty() {
        replaced_content.push_str(line_ending);
    }

    tokio::fs::write(&canonical_path, &replaced_content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", canonical_path.display(), e))?;

    let preview_start = start_idx.saturating_sub(1);
    let preview = build_preview_at(&replaced_content, preview_start);
    let total_lines = replaced_content.lines().count();

    Ok(FileEditResult {
        file: canonical_path.to_string_lossy().to_string(),
        success: true,
        error: None,
        mode: Some("delete".to_string()),
        replacements: Some(0),
        lines: Some((start_line..=end_line).collect()),
        inserted_lines: Some(0),
        deleted_lines: Some(end_line - start_line + 1),
        preview: Some(preview),
        total_lines: Some(total_lines),
        created: Some(false),
    })
}

// ============================================================================
// Patch mode (unified diff)
// ============================================================================
async fn patch_mode(
    content: &str,
    op: &FileEditOperation,
    canonical_path: &Path,
) -> Result<FileEditResult, String> {
    let patch_str = op
        .patch
        .as_deref()
        .ok_or("patch is required for patch mode")?;

    let replaced_content = apply_unified_diff(content, patch_str)?;

    tokio::fs::write(&canonical_path, &replaced_content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", canonical_path.display(), e))?;

    let preview = build_preview(&replaced_content, None);
    let total_lines = replaced_content.lines().count();

    Ok(FileEditResult {
        file: canonical_path.to_string_lossy().to_string(),
        success: true,
        error: None,
        mode: Some("patch".to_string()),
        replacements: Some(1),
        lines: None,
        inserted_lines: None,
        deleted_lines: None,
        preview: Some(preview),
        total_lines: Some(total_lines),
        created: Some(false),
    })
}

fn apply_unified_diff(content: &str, patch: &str) -> Result<String, String> {
    let patch_lines: Vec<&str> = patch.lines().collect();
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut i = 0;

    while i < patch_lines.len() {
        let line = patch_lines[i];
        if line.starts_with("---") || line.starts_with("+++") {
            i += 1;
            continue;
        }
        if line.starts_with("@@") {
            let hunk = parse_hunk_header(line)?;
            i += 1;
            let mut hunk_lines: Vec<DiffLine> = Vec::new();
            while i < patch_lines.len() {
                let l = patch_lines[i];
                if l.starts_with("@@") || l.starts_with("---") || l.starts_with("+++") {
                    break;
                }
                if l.is_empty() {
                    hunk_lines.push(DiffLine::Context(""));
                    i += 1;
                    continue;
                }
                let first_char = l.chars().next()
                    .ok_or_else(|| format!("Unexpected empty line in patch hunk at line {}", i))?;
                match first_char {
                    ' ' => hunk_lines.push(DiffLine::Context(&l[1..])),
                    '-' => hunk_lines.push(DiffLine::Delete(&l[1..])),
                    '+' => hunk_lines.push(DiffLine::Add(&l[1..])),
                    '\\' => {}
                    _ => return Err(format!("Unexpected line in patch hunk: {}", l)),
                }
                i += 1;
            }
            hunks.push(Hunk {
                old_start: hunk.0,
                lines: hunk_lines,
            });
        } else {
            i += 1;
        }
    }

    if hunks.is_empty() {
        return Err("No valid hunks found in patch".to_string());
    }

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    for hunk in hunks.iter().rev() {
        apply_hunk(&mut lines, hunk)?;
    }

    let mut result = lines.join("\n");
    if content.ends_with('\n') && !result.is_empty() {
        result.push('\n');
    }
    Ok(result)
}

#[derive(Debug)]
enum DiffLine<'a> {
    Context(&'a str),
    Delete(&'a str),
    Add(&'a str),
}

struct Hunk<'a> {
    old_start: usize,
    lines: Vec<DiffLine<'a>>,
}

fn parse_hunk_header(line: &str) -> Result<(usize, usize, usize, usize), String> {
    let line = line.trim();
    if !line.starts_with("@@") || !line[2..].contains("@@") {
        return Err(format!("Invalid hunk header: {}", line));
    }
    let inner = &line[3..];
    let end = inner
        .find(" @@")
        .ok_or_else(|| format!("Invalid hunk header: {}", line))?;
    let inner = &inner[..end];

    let parts: Vec<&str> = inner.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(format!("Invalid hunk header format: {}", line));
    }

    let old_part = parts[0].trim_start_matches('-');
    let new_part = parts[1].trim_start_matches('+');

    let (old_start, old_count) = parse_hunk_range(old_part)?;
    let (new_start, new_count) = parse_hunk_range(new_part)?;

    Ok((old_start, old_count, new_start, new_count))
}

fn parse_hunk_range(s: &str) -> Result<(usize, usize), String> {
    let comma = s.find(',');
    let start: usize = s[..comma.unwrap_or(s.len())]
        .parse()
        .map_err(|_| format!("Invalid hunk range number: {}", s))?;
    let count: usize = if let Some(c) = comma {
        s[c + 1..]
            .parse()
            .map_err(|_| format!("Invalid hunk count: {}", s))?
    } else {
        1
    };
    Ok((start, count))
}

fn apply_hunk(lines: &mut Vec<String>, hunk: &Hunk) -> Result<(), String> {
    let start_idx = hunk.old_start.saturating_sub(1);

    let mut line_idx = start_idx;
    for diff_line in &hunk.lines {
        match diff_line {
            DiffLine::Context(expected) => {
                if line_idx >= lines.len() {
                    return Err(format!(
                        "Patch context mismatch at line {}: expected '{}' but file has only {} lines",
                        line_idx + 1,
                        expected,
                        lines.len()
                    ));
                }
                if lines[line_idx].as_str() != *expected {
                    return Err(format!(
                        "Patch context mismatch at line {}: expected '{}' but found '{}'",
                        line_idx + 1,
                        expected,
                        lines[line_idx]
                    ));
                }
                line_idx += 1;
            }
            DiffLine::Delete(expected) => {
                if line_idx >= lines.len() {
                    return Err(format!(
                        "Patch delete mismatch at line {}: expected '{}' but file has only {} lines",
                        line_idx + 1,
                        expected,
                        lines.len()
                    ));
                }
                if lines[line_idx].as_str() != *expected {
                    return Err(format!(
                        "Patch delete mismatch at line {}: expected '{}' but found '{}'",
                        line_idx + 1,
                        expected,
                        lines[line_idx]
                    ));
                }
                line_idx += 1;
            }
            DiffLine::Add(_) => {}
        }
    }

    let mut new_lines: Vec<String> = Vec::new();
    new_lines.extend_from_slice(&lines[..start_idx]);

    let mut line_idx = start_idx;
    for diff_line in &hunk.lines {
        match diff_line {
            DiffLine::Context(_text) => {
                new_lines.push(lines[line_idx].clone());
                line_idx += 1;
            }
            DiffLine::Delete(_) => {
                line_idx += 1;
            }
            DiffLine::Add(text) => {
                new_lines.push((*text).to_string());
            }
        }
    }

    new_lines.extend_from_slice(&lines[line_idx..]);
    *lines = new_lines;

    Ok(())
}

// ============================================================================
// Preview helpers
// ============================================================================
fn build_preview(content: &str, around_line: Option<usize>) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let preview_start = around_line
        .map(|l| l.saturating_sub(2))
        .unwrap_or(0)
        .min(total_lines);
    let preview_end = (preview_start + 5).min(total_lines);

    lines[preview_start..preview_end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:4} | {}", preview_start + i + 1, line))
        .collect()
}

fn build_preview_at(content: &str, start_idx: usize) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let preview_start = start_idx.min(total_lines);
    let preview_end = (preview_start + 5).min(total_lines);

    lines[preview_start..preview_end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:4} | {}", preview_start + i + 1, line))
        .collect()
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_params(operations: Vec<FileEditOperation>) -> EditParams {
        EditParams {
            operations,
            file_type: None,
        }
    }

    #[tokio::test]
    async fn test_string_replace_single() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Hello World\nFoo Bar\nHello World")
            .await
            .unwrap();

        let params = make_params(vec![FileEditOperation {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("string_replace".to_string()),
            old_string: Some("Hello World".to_string()),
            new_string: Some("Hi Universe".to_string()),
            occurrence: Some(1),
            start_line: None,
            end_line: None,
            patch: None,
            markdown: None,
            image_path: None,
            find_text: None,
            location: None,
            element_type: None,
            format_type: None,
            slide_index: None,
            page_index: None,
        }]);

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hi Universe\nFoo Bar\nHello World");
    }

    #[tokio::test]
    async fn test_line_replace() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "line1\nline2\nline3\nline4\nline5\n")
            .await
            .unwrap();

        let params = make_params(vec![FileEditOperation {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("line_replace".to_string()),
            old_string: None,
            new_string: Some("replaced2\nreplaced3".to_string()),
            occurrence: None,
            start_line: Some(2),
            end_line: Some(3),
            patch: None,
            markdown: None,
            image_path: None,
            find_text: None,
            location: None,
            element_type: None,
            format_type: None,
            slide_index: None,
            page_index: None,
        }]);

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "line1\nreplaced2\nreplaced3\nline4\nline5\n");
    }

    #[tokio::test]
    async fn test_insert() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "line1\nline2\n").await.unwrap();

        let params = make_params(vec![FileEditOperation {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("insert".to_string()),
            old_string: None,
            new_string: Some("inserted\n".to_string()),
            occurrence: None,
            start_line: Some(2),
            end_line: None,
            patch: None,
            markdown: None,
            image_path: None,
            find_text: None,
            location: None,
            element_type: None,
            format_type: None,
            slide_index: None,
            page_index: None,
        }]);

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "line1\ninserted\nline2\n");
    }

    #[tokio::test]
    async fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "line1\nline2\nline3\nline4\n")
            .await
            .unwrap();

        let params = make_params(vec![FileEditOperation {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("delete".to_string()),
            old_string: None,
            new_string: None,
            occurrence: None,
            start_line: Some(2),
            end_line: Some(3),
            patch: None,
            markdown: None,
            image_path: None,
            find_text: None,
            location: None,
            element_type: None,
            format_type: None,
            slide_index: None,
            page_index: None,
        }]);

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "line1\nline4\n");
    }

    #[tokio::test]
    async fn test_patch_mode() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "line1\nline2\nline3\nline4\n")
            .await
            .unwrap();

        let patch = r#"--- a/test.txt
+++ b/test.txt
@@ -1,4 +1,4 @@
 line1
-line2
+line2_modified
 line3
-line4
+line4_modified
"#;

        let params = make_params(vec![FileEditOperation {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("patch".to_string()),
            old_string: None,
            new_string: None,
            occurrence: None,
            start_line: None,
            end_line: None,
            patch: Some(patch.to_string()),
            markdown: None,
            image_path: None,
            find_text: None,
            location: None,
            element_type: None,
            format_type: None,
            slide_index: None,
            page_index: None,
        }]);

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("line2_modified"));
        assert!(content.contains("line4_modified"));
    }

    #[tokio::test]
    async fn test_patch_mode_multi_hunk() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "a\nb\nc\nd\ne\nf\n")
            .await
            .unwrap();

        let patch = r#"--- a/test.txt
+++ b/test.txt
@@ -1,2 +1,2 @@
 a
-b
+B
@@ -5,2 +5,2 @@
 e
-f
+F
"#;

        let params = make_params(vec![FileEditOperation {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("patch".to_string()),
            old_string: None,
            new_string: None,
            occurrence: None,
            start_line: None,
            end_line: None,
            patch: Some(patch.to_string()),
            markdown: None,
            image_path: None,
            find_text: None,
            location: None,
            element_type: None,
            format_type: None,
            slide_index: None,
            page_index: None,
        }]);

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok(), "{:?}", result);

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "a\nB\nc\nd\ne\nF\n");
    }
}
