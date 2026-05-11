use crate::utils::file_utils::{ensure_path_within_working_dir, strip_unc_prefix};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_FILE_SIZE: usize = 100 * 1024 * 1024;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileWriteItem {
    #[schemars(description = "The file path to write")]
    pub path: String,
    #[schemars(description = "The content to write")]
    pub content: String,
    #[schemars(description = "Write mode: new (default), append, or overwrite")]
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SheetInput {
    #[schemars(description = "Sheet name")]
    pub name: String,
    #[schemars(description = "Rows of values")]
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SlideInput {
    #[schemars(description = "Slide title")]
    pub title: Option<String>,
    #[schemars(description = "Bullet points for slide content")]
    pub content: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CellInput {
    #[schemars(description = "Cell type: code, markdown, or raw")]
    pub cell_type: String,
    #[schemars(description = "Source lines")]
    pub source: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteParams {
    #[schemars(description = "List of files to write concurrently (for text files)")]
    pub files: Vec<FileWriteItem>,

    #[schemars(description = "File type for office formats. Auto-detect by extension.")]
    pub file_type: Option<String>,
    #[schemars(description = "For docx: array of paragraph texts")]
    pub docx_paragraphs: Option<Vec<String>>,
    #[schemars(description = "For xlsx: array of sheet data")]
    pub xlsx_sheets: Option<Vec<SheetInput>>,
    #[schemars(description = "For pptx: array of slide data")]
    pub pptx_slides: Option<Vec<SlideInput>>,
    #[schemars(description = "For ipynb: array of cells")]
    pub ipynb_cells: Option<Vec<CellInput>>,

    #[schemars(description = "Markdown content for creating DOCX or PDF files.")]
    pub office_markdown: Option<String>,
    #[schemars(description = "CSV content for creating XLSX files. Separate sheets with '--- SheetName ---' lines.")]
    pub office_csv: Option<String>,
}

#[derive(Debug, Serialize)]
struct FileWriteResult {
    path: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes_written: Option<usize>,
}

pub async fn file_write(
    params: Parameters<WriteParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;

    let file_type = detect_file_type(&params)?;
    if let Some(ft) = file_type {
        let result = create_office_file(&params, &ft, working_dir)?;
        return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?,
        )]));
    }

    let mut futures = Vec::new();
    for item in params.files {
        futures.push(write_single_file(item, working_dir));
    }

    let results = futures::future::join_all(futures).await;

    let json = serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

fn detect_file_type(params: &WriteParams) -> Result<Option<String>, String> {
    if let Some(ref ft) = params.file_type {
        let ft_lower = ft.to_lowercase();
        if matches!(ft_lower.as_str(), "docx" | "pptx" | "xlsx" | "ipynb" | "pdf") {
            return Ok(Some(ft_lower));
        }
    }

    if params.office_markdown.is_some() {
        return Err("office_markdown provided but no file_type specified. Use file_type=\"docx\" or file_type=\"pdf\".".to_string());
    }
    if params.office_csv.is_some() {
        return Err("office_csv provided but no file_type specified. Use file_type=\"xlsx\".".to_string());
    }

    let defined_fields: Vec<&str> = {
        let mut fields = Vec::new();
        if params.docx_paragraphs.is_some() {
            fields.push("docx_paragraphs");
        }
        if params.xlsx_sheets.is_some() {
            fields.push("xlsx_sheets");
        }
        if params.pptx_slides.is_some() {
            fields.push("pptx_slides");
        }
        if params.ipynb_cells.is_some() {
            fields.push("ipynb_cells");
        }
        fields
    };

    if defined_fields.len() > 1 {
        return Err(format!(
            "Ambiguous office format: multiple content fields provided ({}). Please specify file_type explicitly.",
            defined_fields.join(", ")
        ));
    }

    if params.docx_paragraphs.is_some() {
        return Ok(Some("docx".to_string()));
    }
    if params.xlsx_sheets.is_some() {
        return Ok(Some("xlsx".to_string()));
    }
    if params.pptx_slides.is_some() {
        return Ok(Some("pptx".to_string()));
    }
    if params.ipynb_cells.is_some() {
        return Ok(Some("ipynb".to_string()));
    }

    Ok(None)
}

fn create_office_file(
    params: &WriteParams,
    file_type: &str,
    working_dir: &Path,
) -> Result<FileWriteResult, String> {
    let path = params
        .files
        .first()
        .map(|f| f.path.as_str())
        .ok_or("No file path specified for office document creation")?;

    let path = Path::new(path);
    let canonical_path = ensure_path_within_working_dir(path, working_dir)?;

    if let Some(parent) = canonical_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directories: {}", e))?;
    }

    let display_path = strip_unc_prefix(&canonical_path.to_string_lossy());

    match file_type {
        "docx" => {
            if let Some(ref md) = params.office_markdown {
                create_docx_from_markdown(md, &canonical_path)?;
                return Ok(FileWriteResult {
                    path: display_path.clone(),
                    success: true,
                    error: None,
                    message: Some(format!("DOCX file created from markdown at '{}'", display_path)),
                    bytes_written: None,
                });
            }
            let paragraphs = params
                .docx_paragraphs
                .as_ref()
                .ok_or("docx_paragraphs or office_markdown is required for docx creation")?;
            create_docx(paragraphs, &canonical_path)?;
            Ok(FileWriteResult {
                path: display_path.clone(),
                success: true,
                error: None,
                message: Some(format!("DOCX file '{}' created with {} paragraph(s)", display_path, paragraphs.len())),
                bytes_written: None,
            })
        }
        "xlsx" => {
            if let Some(ref csv) = params.office_csv {
                create_xlsx_from_csv(csv, &canonical_path)?;
                return Ok(FileWriteResult {
                    path: display_path.clone(),
                    success: true,
                    error: None,
                    message: Some(format!("XLSX file created from CSV at '{}'", display_path)),
                    bytes_written: None,
                });
            }
            let sheets = params
                .xlsx_sheets
                .as_ref()
                .ok_or("xlsx_sheets or office_csv is required for xlsx creation")?;
            create_xlsx(sheets, &canonical_path)?;
            Ok(FileWriteResult {
                path: display_path.clone(),
                success: true,
                error: None,
                message: Some(format!("XLSX file '{}' created with {} sheet(s)", display_path, sheets.len())),
                bytes_written: None,
            })
        }
        "pptx" => {
            let slides = params
                .pptx_slides
                .as_ref()
                .ok_or("pptx_slides is required for pptx creation")?;
            create_pptx(slides, &canonical_path)?;
            Ok(FileWriteResult {
                path: display_path.clone(),
                success: true,
                error: None,
                message: Some(format!("PPTX file '{}' created with {} slide(s)", display_path, slides.len())),
                bytes_written: None,
            })
        }
        "ipynb" => {
            let cells = params
                .ipynb_cells
                .as_ref()
                .ok_or("ipynb_cells is required for ipynb creation")?;
            create_ipynb(cells, &canonical_path)?;
            Ok(FileWriteResult {
                path: display_path.clone(),
                success: true,
                error: None,
                message: Some(format!("IPYNB file '{}' created with {} cell(s)", display_path, cells.len())),
                bytes_written: None,
            })
        }
        "pdf" => {
            let md = params
                .office_markdown
                .as_ref()
                .ok_or("office_markdown is required for PDF creation")?;
            create_pdf_from_markdown(md, &canonical_path)?;
            Ok(FileWriteResult {
                path: display_path.clone(),
                success: true,
                error: None,
                message: Some(format!("PDF file created from markdown at '{}'", display_path)),
                bytes_written: None,
            })
        }
        _ => Err(format!("Unsupported office file type: {}", file_type)),
    }
}

fn create_docx(paragraphs: &[String], path: &Path) -> Result<(), String> {
    use docx_rs::*;

    let mut docx = Docx::new();
    for para_text in paragraphs {
        docx = docx.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text(para_text)),
        );
    }
    let mut output = Vec::new();
    docx.build()
        .pack(&mut std::io::Cursor::new(&mut output))
        .map_err(|e| format!("Failed to build DOCX: {}", e))?;
    std::fs::write(path, &output).map_err(|e| format!("Failed to write DOCX file: {}", e))?;
    Ok(())
}

fn create_docx_from_markdown(markdown: &str, path: &Path) -> Result<(), String> {
    use docx_rs::*;

    let mut docx = Docx::new();

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
            Paragraph::new()
                .add_run(Run::new().add_text(text).bold().size(32))
                .style("Heading1")
        } else if let Some(text) = trimmed.strip_prefix("## ") {
            Paragraph::new()
                .add_run(Run::new().add_text(text).bold().size(28))
                .style("Heading2")
        } else if let Some(text) = trimmed.strip_prefix("### ") {
            Paragraph::new()
                .add_run(Run::new().add_text(text).bold().size(24))
                .style("Heading3")
        } else if let Some(text) = trimmed.strip_prefix("#### ") {
            Paragraph::new()
                .add_run(Run::new().add_text(text).bold().size(22))
                .style("Heading4")
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            Paragraph::new()
                .add_run(Run::new().add_text(&trimmed[2..]))
        } else {
            Paragraph::new()
                .add_run(Run::new().add_text(trimmed))
        };

        docx = docx.add_paragraph(para);
    }

    let mut output = Vec::new();
    docx.build()
        .pack(&mut std::io::Cursor::new(&mut output))
        .map_err(|e| format!("Failed to build DOCX: {}", e))?;
    std::fs::write(path, &output).map_err(|e| format!("Failed to write DOCX file: {}", e))?;
    Ok(())
}

fn create_xlsx(sheets: &[SheetInput], path: &Path) -> Result<(), String> {
    use rust_xlsxwriter::Workbook;

    let mut workbook = Workbook::new();
    for sheet in sheets {
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(&sheet.name)
            .map_err(|e| format!("Failed to set sheet name '{}': {}", sheet.name, e))?;
        for (row_idx, row) in sheet.rows.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                worksheet
                    .write(row_idx as u32, col_idx as u16, cell.as_str())
                    .map_err(|e| {
                        format!(
                            "Failed to write cell ({}, {}) in sheet '{}': {}",
                            row_idx, col_idx, sheet.name, e
                        )
                    })?;
            }
        }
    }
    workbook
        .save(path)
        .map_err(|e| format!("Failed to save XLSX file: {}", e))?;
    Ok(())
}

fn create_xlsx_from_csv(csv: &str, path: &Path) -> Result<(), String> {
    use rust_xlsxwriter::Workbook;

    let mut workbook = Workbook::new();
    let mut current_sheet_name = "Sheet1".to_string();
    let mut current_rows: Vec<Vec<String>> = Vec::new();
    let mut is_first_sheet = true;
    let mut all_sheets: Vec<(String, Vec<Vec<String>>)> = Vec::new();

    for line in csv.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("--- ") && trimmed.ends_with(" ---") {
            if !is_first_sheet || !current_rows.is_empty() {
                all_sheets.push((current_sheet_name.clone(), std::mem::take(&mut current_rows)));
            }
            current_sheet_name = trimmed
                .trim_start_matches("--- ")
                .trim_end_matches(" ---")
                .to_string();
            is_first_sheet = false;
            continue;
        }
        if trimmed.is_empty() && current_rows.is_empty() {
            continue;
        }
        let cells: Vec<String> = trimmed
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect();
        current_rows.push(cells);
    }
    if !current_rows.is_empty() || is_first_sheet {
        all_sheets.push((current_sheet_name, current_rows));
    }

    for (name, rows) in &all_sheets {
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
        .map_err(|e| format!("Failed to save XLSX file: {}", e))?;
    Ok(())
}

fn create_pptx(slides: &[SlideInput], path: &Path) -> Result<(), String> {
    use ppt_rs::{Presentation, SlideContent};

    let empty_title = String::new();
    let title = slides
        .first()
        .and_then(|s| s.title.as_ref())
        .unwrap_or(&empty_title);

    let mut pres = Presentation::with_title(title);
    for slide_input in slides {
        let slide_title = slide_input.title.as_deref().unwrap_or("");
        let mut slide = SlideContent::new(slide_title);
        if let Some(ref content) = slide_input.content {
            for bullet in content {
                slide = slide.add_bullet(bullet);
            }
        }
        pres = pres.add_slide(slide);
    }
    pres
        .save(path)
        .map_err(|e| format!("Failed to save PPTX file: {}", e))?;
    Ok(())
}

fn create_pdf_from_markdown(markdown: &str, path: &Path) -> Result<(), String> {
    use crate::utils::office_converter;

    let soffice = office_converter::find_libreoffice()
        .ok_or_else(|| "LibreOffice is required to create PDF files from markdown. Please install LibreOffice.".to_string())?;

    let tmp_md = tempfile::Builder::new()
        .prefix("mcp_md_")
        .suffix(".md")
        .tempfile()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    std::io::Write::write_all(&mut tmp_md.as_file(), markdown.as_bytes())
        .map_err(|e| format!("Failed to write temp markdown: {}", e))?;

    let out_dir = tempfile::tempdir()
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let output = std::process::Command::new(&soffice)
        .args(["--headless", "--convert-to", "pdf", "--outdir"])
        .arg(out_dir.path())
        .arg(tmp_md.path())
        .output()
        .map_err(|e| format!("Failed to run LibreOffice for PDF conversion: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "PDF conversion failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stem = tmp_md
        .path()
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let pdf_path = out_dir.path().join(format!("{}.pdf", stem));
    std::fs::copy(&pdf_path, path)
        .map_err(|e| format!("Failed to copy PDF to destination: {}", e))?;

    Ok(())
}

fn create_ipynb(cells: &[CellInput], path: &Path) -> Result<(), String> {
    use ipynb::{Cell, CodeCell, MarkdownCell, Notebook, RawCell};
    use std::collections::HashMap;

    let notebook_cells: Vec<Cell> = cells
        .iter()
        .map(|ci| match ci.cell_type.as_str() {
            "markdown" => Ok(Cell::Markdown(MarkdownCell {
                source: ci.source.clone(),
                metadata: HashMap::new(),
                id: None,
                attachments: None,
            })),
            "code" => Ok(Cell::Code(CodeCell {
                source: ci.source.clone(),
                metadata: HashMap::new(),
                id: None,
                execution_count: None,
                outputs: vec![],
            })),
            "raw" => Ok(Cell::Raw(RawCell {
                source: ci.source.clone(),
                metadata: HashMap::new(),
                id: None,
            })),
            other => Err(format!("Unknown cell type: {}", other)),
        })
        .collect::<Result<Vec<_>, String>>()?;

    let notebook = Notebook {
        cells: notebook_cells,
        metadata: HashMap::new(),
        nbformat: 4,
        nbformat_minor: 5,
    };

    let json = serde_json::to_string_pretty(&notebook)
        .map_err(|e| format!("Failed to serialize IPYNB: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write IPYNB file: {}", e))?;
    Ok(())
}

async fn write_single_file(item: FileWriteItem, working_dir: &Path) -> FileWriteResult {
    if item.content.len() > MAX_FILE_SIZE {
        return FileWriteResult {
            path: item.path.clone(),
            success: false,
            error: Some(format!(
                "Content size {} exceeds maximum allowed size of {} bytes",
                item.content.len(),
                MAX_FILE_SIZE
            )),
            message: None,
            bytes_written: None,
        };
    }

    let path = Path::new(&item.path);
    let mode = item.mode.as_deref().unwrap_or("new");

    let canonical_path = match ensure_path_within_working_dir(path, working_dir) {
        Ok(p) => p,
        Err(e) => {
            return FileWriteResult {
                path: item.path,
                success: false,
                error: Some(e),
                message: None,
                bytes_written: None,
            }
        }
    };

    match mode {
        "new" => {
            if canonical_path.exists() {
                return FileWriteResult {
                    path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                    success: false,
                    error: Some(format!(
                        "File '{}' already exists. Use 'overwrite' or 'append' mode.",
                        item.path
                    )),
                    message: None,
                    bytes_written: None,
                };
            }
        }
        "append" | "overwrite" => {}
        _ => {
            return FileWriteResult {
                path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!(
                    "Invalid mode '{}'. Use 'new', 'append', or 'overwrite'.",
                    mode
                )),
                message: None,
                bytes_written: None,
            }
        }
    }

    if let Some(parent) = canonical_path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            return FileWriteResult {
                path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!("Failed to create parent directories: {}", e)),
                message: None,
                bytes_written: None,
            };
        }
    }

    let write_result = match mode {
        "new" | "overwrite" => {
            tokio::fs::write(&canonical_path, &item.content).await
        }
        "append" => {
            use tokio::io::AsyncWriteExt;
            let mut file = match tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&canonical_path)
                .await
            {
                Ok(f) => f,
                Err(e) => {
                    return FileWriteResult {
                        path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                        success: false,
                        error: Some(format!("Failed to open file for append: {}", e)),
                        message: None,
                        bytes_written: None,
                    }
                }
            };

            let write_res = file.write_all(item.content.as_bytes()).await;
            if let Err(e) = write_res {
                return FileWriteResult {
                    path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                    success: false,
                    error: Some(format!("Failed to write file: {}", e)),
                    message: None,
                    bytes_written: None,
                };
            }
            if let Err(e) = file.flush().await {
                return FileWriteResult {
                    path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                    success: false,
                    error: Some(format!("Flush failed: {}", e)),
                    message: None,
                    bytes_written: None,
                };
            }
            Ok(())
        }
        _ => unreachable!(),
    };

    if let Err(e) = write_result {
        return FileWriteResult {
            path: strip_unc_prefix(&canonical_path.to_string_lossy()),
            success: false,
            error: Some(format!("Failed to write file: {}", e)),
            message: None,
            bytes_written: None,
        };
    }

    let action = match mode {
        "new" => "created",
        "append" => "appended to",
        "overwrite" => "overwritten",
        _ => "written",
    };

    FileWriteResult {
        path: strip_unc_prefix(&canonical_path.to_string_lossy()),
        success: true,
        error: None,
        message: Some(format!(
            "File '{}' {} successfully.",
            strip_unc_prefix(&canonical_path.to_string_lossy()),
            action
        )),
        bytes_written: Some(item.content.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_write_new() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let params = WriteParams {
            files: vec![FileWriteItem {
                path: file_path.to_string_lossy().to_string(),
                content: "Hello, World!".to_string(),
                mode: Some("new".to_string()),
            }],
            file_type: None,
            docx_paragraphs: None,
            xlsx_sheets: None,
            pptx_slides: None,
            ipynb_cells: None,
            office_markdown: None,
            office_csv: None,
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_file_write_append() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Initial ").await.unwrap();

        let params = WriteParams {
            files: vec![FileWriteItem {
                path: file_path.to_string_lossy().to_string(),
                content: "Appended".to_string(),
                mode: Some("append".to_string()),
            }],
            file_type: None,
            docx_paragraphs: None,
            xlsx_sheets: None,
            pptx_slides: None,
            ipynb_cells: None,
            office_markdown: None,
            office_csv: None,
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Initial Appended");
    }

    #[tokio::test]
    async fn test_file_write_outside_working_dir() {
        let temp_dir = TempDir::new().unwrap();

        let params = WriteParams {
            files: vec![FileWriteItem {
                path: "/etc/test.txt".to_string(),
                content: "test".to_string(),
                mode: Some("new".to_string()),
            }],
            file_type: None,
            docx_paragraphs: None,
            xlsx_sheets: None,
            pptx_slides: None,
            ipynb_cells: None,
            office_markdown: None,
            office_csv: None,
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"success\": false"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_write_multiple() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("a.txt");
        let file2 = temp_dir.path().join("b.txt");

        let params = WriteParams {
            files: vec![
                FileWriteItem {
                    path: file1.to_string_lossy().to_string(),
                    content: "File A".to_string(),
                    mode: Some("new".to_string()),
                },
                FileWriteItem {
                    path: file2.to_string_lossy().to_string(),
                    content: "File B".to_string(),
                    mode: Some("new".to_string()),
                },
            ],
            file_type: None,
            docx_paragraphs: None,
            xlsx_sheets: None,
            pptx_slides: None,
            ipynb_cells: None,
            office_markdown: None,
            office_csv: None,
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(file1.exists());
        assert!(file2.exists());
    }

    #[tokio::test]
    async fn test_create_docx() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.docx");

        let params = WriteParams {
            files: vec![FileWriteItem {
                path: file_path.to_string_lossy().to_string(),
                content: String::new(),
                mode: None,
            }],
            file_type: Some("docx".to_string()),
            docx_paragraphs: Some(vec![
                "Heading Paragraph".to_string(),
                "This is the second paragraph.".to_string(),
            ]),
            xlsx_sheets: None,
            pptx_slides: None,
            ipynb_cells: None,
            office_markdown: None,
            office_csv: None,
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok(), "Result: {:?}", result);
        assert!(file_path.exists());
        assert!(file_path.metadata().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_create_xlsx() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.xlsx");

        let params = WriteParams {
            files: vec![FileWriteItem {
                path: file_path.to_string_lossy().to_string(),
                content: String::new(),
                mode: None,
            }],
            file_type: Some("xlsx".to_string()),
            docx_paragraphs: None,
            xlsx_sheets: Some(vec![SheetInput {
                name: "Sheet1".to_string(),
                rows: vec![
                    vec!["Name".to_string(), "Age".to_string()],
                    vec!["Alice".to_string(), "30".to_string()],
                ],
            }]),
            pptx_slides: None,
            ipynb_cells: None,
            office_markdown: None,
            office_csv: None,
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok(), "Result: {:?}", result);
        assert!(file_path.exists());
        assert!(file_path.metadata().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_create_pptx() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.pptx");

        let params = WriteParams {
            files: vec![FileWriteItem {
                path: file_path.to_string_lossy().to_string(),
                content: String::new(),
                mode: None,
            }],
            file_type: Some("pptx".to_string()),
            docx_paragraphs: None,
            xlsx_sheets: None,
            pptx_slides: Some(vec![
                SlideInput {
                    title: Some("Slide 1".to_string()),
                    content: Some(vec!["Bullet 1".to_string(), "Bullet 2".to_string()]),
                },
                SlideInput {
                    title: Some("Slide 2".to_string()),
                    content: Some(vec!["Another bullet".to_string()]),
                },
            ]),
            ipynb_cells: None,
            office_markdown: None,
            office_csv: None,
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok(), "Result: {:?}", result);
        assert!(file_path.exists());
        assert!(file_path.metadata().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_create_ipynb() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.ipynb");

        let params = WriteParams {
            files: vec![FileWriteItem {
                path: file_path.to_string_lossy().to_string(),
                content: String::new(),
                mode: None,
            }],
            file_type: Some("ipynb".to_string()),
            docx_paragraphs: None,
            xlsx_sheets: None,
            pptx_slides: None,
            ipynb_cells: Some(vec![
                CellInput {
                    cell_type: "markdown".to_string(),
                    source: vec!["# My Notebook".to_string()],
                },
                CellInput {
                    cell_type: "code".to_string(),
                    source: vec!["print('Hello')".to_string()],
                },
            ]),
            office_markdown: None,
            office_csv: None,
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok(), "Result: {:?}", result);
        assert!(file_path.exists());
        assert!(file_path.metadata().unwrap().len() > 0);

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        let nb: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(nb["nbformat"].as_i64().unwrap(), 4);
        assert_eq!(nb["cells"].as_array().unwrap().len(), 2);
    }
}
