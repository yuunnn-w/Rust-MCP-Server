use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::utils::file_utils::ensure_path_within_working_dir;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NotebookEditParams {
    #[schemars(description = "Path to the .ipynb file to read or edit")]
    pub path: String,
    #[schemars(description = "Operation to perform: read, write, add_cell, edit_cell, delete_cell")]
    pub operation: String,
    #[schemars(description = "Cells to write to the notebook (for write operation)")]
    pub cells: Option<Vec<CellInput>>,
    #[schemars(description = "Cell index to edit or delete (0-indexed)")]
    pub cell_index: Option<usize>,
    #[schemars(description = "New cell source content (for add_cell and edit_cell operations)")]
    pub cell_content: Option<String>,
    #[schemars(description = "Cell type for new cells: code, markdown, or raw. Default: code")]
    pub cell_type: Option<String>,
    #[schemars(description = "Output format: text (default) or json")]
    pub output_format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CellInput {
    #[schemars(description = "Cell type: code, markdown, or raw")]
    pub cell_type: String,
    #[schemars(description = "Source lines of the cell")]
    pub source: Vec<String>,
}

fn cell_type_label(cell: &ipynb::Cell) -> &'static str {
    match cell {
        ipynb::Cell::Code(_) => "[code]",
        ipynb::Cell::Markdown(_) => "[markdown]",
        ipynb::Cell::Raw(_) => "[raw]",
    }
}

fn read_notebook(path: &str, working_dir: &Path, output_format: Option<&str>) -> Result<CallToolResult, String> {
    let file_path = ensure_path_within_working_dir(Path::new(path), working_dir)?;
    let data = std::fs::read(&file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path.display(), e))?;
    let notebook: ipynb::Notebook = serde_json::from_slice(&data)
        .map_err(|e| format!("Failed to parse notebook '{}': {}", file_path.display(), e))?;

    if output_format == Some("json") {
        let json = serde_json::to_string_pretty(&notebook)
            .map_err(|e| format!("Failed to serialize notebook: {}", e))?;
        return Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]));
    }

    let mut output = String::new();
    output.push_str(&format!("Notebook: {}\n", file_path.display()));
    output.push_str(&format!("Format: nbformat {}, nbformat_minor {}\n", notebook.nbformat, notebook.nbformat_minor));
    output.push_str(&format!("Total cells: {}\n\n", notebook.cells.len()));

    for (i, cell) in notebook.cells.iter().enumerate() {
        let label = cell_type_label(cell);
        output.push_str(&format!("--- Cell {} {} ---\n", i, label));
        let source = match cell {
            ipynb::Cell::Code(c) => &c.source,
            ipynb::Cell::Markdown(c) => &c.source,
            ipynb::Cell::Raw(c) => &c.source,
        };
        for line in source {
            output.push_str(line);
        }
        output.push('\n');
    }

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(output)]))
}

fn build_cell(input: &CellInput) -> ipynb::Cell {
    let source = input.source.clone();
    let metadata = HashMap::new();
    match input.cell_type.as_str() {
        "markdown" => ipynb::Cell::Markdown(ipynb::MarkdownCell {
            metadata,
            id: None,
            attachments: None,
            source,
        }),
        "raw" => ipynb::Cell::Raw(ipynb::RawCell {
            metadata,
            id: None,
            source,
        }),
        _ => ipynb::Cell::Code(ipynb::CodeCell {
            metadata,
            source,
            id: None,
            execution_count: None,
            outputs: Vec::new(),
        }),
    }
}

fn write_notebook(path: &str, working_dir: &Path, cells: Option<Vec<CellInput>>) -> Result<CallToolResult, String> {
    let cells = cells.ok_or_else(|| "Missing 'cells' parameter for write operation".to_string())?;
    let file_path = ensure_path_within_working_dir(Path::new(path), working_dir)?;

    let notebook = ipynb::Notebook {
        cells: cells.iter().map(build_cell).collect(),
        metadata: HashMap::new(),
        nbformat: 4,
        nbformat_minor: 5,
    };

    let json = serde_json::to_vec_pretty(&notebook)
        .map_err(|e| format!("Failed to serialize notebook: {}", e))?;
    std::fs::write(&file_path, &json)
        .map_err(|e| format!("Failed to write file '{}': {}", file_path.display(), e))?;

    let count = notebook.cells.len();
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!("Wrote {} cells to {}", count, file_path.display())
    )]))
}

fn add_cell_to_notebook(path: &str, working_dir: &Path, cell_content: Option<String>, cell_type: Option<String>) -> Result<CallToolResult, String> {
    let content = cell_content.ok_or_else(|| "Missing 'cell_content' parameter for add_cell operation".to_string())?;
    let ctype = cell_type.unwrap_or_else(|| "code".to_string());
    let file_path = ensure_path_within_working_dir(Path::new(path), working_dir)?;

    let data = std::fs::read(&file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path.display(), e))?;
    let mut notebook: ipynb::Notebook = serde_json::from_slice(&data)
        .map_err(|e| format!("Failed to parse notebook '{}': {}", file_path.display(), e))?;

    let source: Vec<String> = content.split_inclusive('\n').map(|s| {
        if s.ends_with('\n') { s.to_string() } else { format!("{}\n", s) }
    }).collect();
    let input = CellInput {
        cell_type: ctype,
        source,
    };
    notebook.cells.push(build_cell(&input));

    let json = serde_json::to_vec_pretty(&notebook)
        .map_err(|e| format!("Failed to serialize notebook: {}", e))?;
    std::fs::write(&file_path, &json)
        .map_err(|e| format!("Failed to write file '{}': {}", file_path.display(), e))?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!("Added cell at index {} to {}", notebook.cells.len() - 1, file_path.display())
    )]))
}

fn edit_cell_in_notebook(path: &str, working_dir: &Path, cell_index: Option<usize>, cell_content: Option<String>) -> Result<CallToolResult, String> {
    let index = cell_index.ok_or_else(|| "Missing 'cell_index' parameter for edit_cell operation".to_string())?;
    let content = cell_content.ok_or_else(|| "Missing 'cell_content' parameter for edit_cell operation".to_string())?;
    let file_path = ensure_path_within_working_dir(Path::new(path), working_dir)?;

    let data = std::fs::read(&file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path.display(), e))?;
    let mut notebook: ipynb::Notebook = serde_json::from_slice(&data)
        .map_err(|e| format!("Failed to parse notebook '{}': {}", file_path.display(), e))?;

    if index >= notebook.cells.len() {
        return Err(format!(
            "Cell index {} out of range (notebook has {} cells)",
            index,
            notebook.cells.len()
        ));
    }

    let source: Vec<String> = content.split_inclusive('\n').map(|s| {
        if s.ends_with('\n') { s.to_string() } else { format!("{}\n", s) }
    }).collect();
    match &mut notebook.cells[index] {
        ipynb::Cell::Code(c) => c.source = source,
        ipynb::Cell::Markdown(c) => c.source = source,
        ipynb::Cell::Raw(c) => c.source = source,
    }

    let json = serde_json::to_vec_pretty(&notebook)
        .map_err(|e| format!("Failed to serialize notebook: {}", e))?;
    std::fs::write(&file_path, &json)
        .map_err(|e| format!("Failed to write file '{}': {}", file_path.display(), e))?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!("Edited cell {} in {}", index, file_path.display())
    )]))
}

fn delete_cell_from_notebook(path: &str, working_dir: &Path, cell_index: Option<usize>) -> Result<CallToolResult, String> {
    let index = cell_index.ok_or_else(|| "Missing 'cell_index' parameter for delete_cell operation".to_string())?;
    let file_path = ensure_path_within_working_dir(Path::new(path), working_dir)?;

    let data = std::fs::read(&file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path.display(), e))?;
    let mut notebook: ipynb::Notebook = serde_json::from_slice(&data)
        .map_err(|e| format!("Failed to parse notebook '{}': {}", file_path.display(), e))?;

    if index >= notebook.cells.len() {
        return Err(format!(
            "Cell index {} out of range (notebook has {} cells)",
            index,
            notebook.cells.len()
        ));
    }

    notebook.cells.remove(index);

    let json = serde_json::to_vec_pretty(&notebook)
        .map_err(|e| format!("Failed to serialize notebook: {}", e))?;
    std::fs::write(&file_path, &json)
        .map_err(|e| format!("Failed to write file '{}': {}", file_path.display(), e))?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!("Deleted cell {} from {}", index, file_path.display())
    )]))
}

pub async fn notebook_edit(
    params: Parameters<NotebookEditParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;

    match params.operation.as_str() {
        "read" => read_notebook(&params.path, working_dir, params.output_format.as_deref()),
        "write" => write_notebook(&params.path, working_dir, params.cells),
        "add_cell" => add_cell_to_notebook(&params.path, working_dir, params.cell_content, params.cell_type),
        "edit_cell" => edit_cell_in_notebook(&params.path, working_dir, params.cell_index, params.cell_content),
        "delete_cell" => delete_cell_from_notebook(&params.path, working_dir, params.cell_index),
        _ => Err(format!(
            "Unknown operation: {}. Supported: read, write, add_cell, edit_cell, delete_cell",
            params.operation
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_temp_notebook(dir: &TempDir, name: &str, cells_json: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let raw = format!(
            r#"{{"cells":{},"metadata":{{}},"nbformat":4,"nbformat_minor":5}}"#,
            cells_json
        );
        std::fs::write(&path, &raw).unwrap();
        path
    }

    #[test]
    fn test_read_notebook_text() {
        let dir = TempDir::new().unwrap();
        let cells = r##"[
            {"cell_type":"code","metadata":{},"source":["print(\"hello\")\n"],"execution_count":null,"outputs":[],"id":"a"},
            {"cell_type":"markdown","metadata":{},"source":["# Title\n"],"id":"b"}
        ]"##;
        let path = make_temp_notebook(&dir, "test.ipynb", cells);
        let working_dir = dir.path().to_path_buf();

        let result = read_notebook(
            path.to_str().unwrap(),
            &working_dir,
            None,
        ).unwrap();

        assert!(!result.is_error.unwrap_or(false));
    }

    #[test]
    fn test_read_notebook_json() {
        let dir = TempDir::new().unwrap();
        let cells = r#"[
            {"cell_type":"code","metadata":{},"source":["print(\"x\")\n"],"execution_count":null,"outputs":[],"id":"c"}
        ]"#;
        let path = make_temp_notebook(&dir, "test2.ipynb", cells);
        let working_dir = dir.path().to_path_buf();

        let result = read_notebook(
            path.to_str().unwrap(),
            &working_dir,
            Some("json"),
        ).unwrap();

        assert!(!result.is_error.unwrap_or(false));
    }

    #[test]
    fn test_write_and_read_notebook() {
        let dir = TempDir::new().unwrap();
        let working_dir = dir.path().to_path_buf();
        let path = dir.path().join("new.ipynb");
        let path_str = path.to_str().unwrap().to_string();

        let cells = vec![
            CellInput {
                cell_type: "code".to_string(),
                source: vec!["print(1)\n".to_string()],
            },
            CellInput {
                cell_type: "markdown".to_string(),
                source: vec!["## Hello\n".to_string()],
            },
        ];

        let write_result = write_notebook(&path_str, &working_dir, Some(cells)).unwrap();
        assert!(!write_result.is_error.unwrap_or(false));

        let read_result = read_notebook(&path_str, &working_dir, None).unwrap();
        assert!(!read_result.is_error.unwrap_or(false));
    }

    #[test]
    fn test_add_cell() {
        let dir = TempDir::new().unwrap();
        let cells = r#"[
            {"cell_type":"code","metadata":{},"source":["x=1\n"],"execution_count":null,"outputs":[],"id":"d"}
        ]"#;
        let path = make_temp_notebook(&dir, "base.ipynb", cells);
        let path_str = path.to_str().unwrap().to_string();
        let working_dir = dir.path().to_path_buf();

        let result = add_cell_to_notebook(
            &path_str,
            &working_dir,
            Some("y = 2".to_string()),
            Some("code".to_string()),
        ).unwrap();
        assert!(!result.is_error.unwrap_or(false));

        let data = std::fs::read_to_string(&path).unwrap();
        let nb: ipynb::Notebook = serde_json::from_str(&data).unwrap();
        assert_eq!(nb.cells.len(), 2);
    }

    #[test]
    fn test_edit_cell() {
        let dir = TempDir::new().unwrap();
        let cells = r#"[
            {"cell_type":"code","metadata":{},"source":["old\n"],"execution_count":null,"outputs":[],"id":"e"}
        ]"#;
        let path = make_temp_notebook(&dir, "edit.ipynb", cells);
        let path_str = path.to_str().unwrap().to_string();
        let working_dir = dir.path().to_path_buf();

        let result = edit_cell_in_notebook(
            &path_str,
            &working_dir,
            Some(0),
            Some("new".to_string()),
        ).unwrap();
        assert!(!result.is_error.unwrap_or(false));

        let data = std::fs::read_to_string(&path).unwrap();
        let nb: ipynb::Notebook = serde_json::from_str(&data).unwrap();
        let source = match &nb.cells[0] {
            ipynb::Cell::Code(c) => &c.source,
            ipynb::Cell::Markdown(c) => &c.source,
            ipynb::Cell::Raw(c) => &c.source,
        };
        assert_eq!(source.join(""), "new\n");
    }

    #[test]
    fn test_delete_cell() {
        let dir = TempDir::new().unwrap();
        let cells = r#"[
            {"cell_type":"code","metadata":{},"source":["a\n"],"execution_count":null,"outputs":[],"id":"f"},
            {"cell_type":"code","metadata":{},"source":["b\n"],"execution_count":null,"outputs":[],"id":"g"}
        ]"#;
        let path = make_temp_notebook(&dir, "del.ipynb", cells);
        let path_str = path.to_str().unwrap().to_string();
        let working_dir = dir.path().to_path_buf();

        let result = delete_cell_from_notebook(
            &path_str,
            &working_dir,
            Some(0),
        ).unwrap();
        assert!(!result.is_error.unwrap_or(false));

        let data = std::fs::read_to_string(&path).unwrap();
        let nb: ipynb::Notebook = serde_json::from_str(&data).unwrap();
        assert_eq!(nb.cells.len(), 1);
    }

    #[test]
    fn test_edit_cell_out_of_range() {
        let dir = TempDir::new().unwrap();
        let cells = r#"[
            {"cell_type":"code","metadata":{},"source":["a\n"],"execution_count":null,"outputs":[],"id":"h"}
        ]"#;
        let path = make_temp_notebook(&dir, "oor.ipynb", cells);
        let path_str = path.to_str().unwrap().to_string();
        let working_dir = dir.path().to_path_buf();

        let result = edit_cell_in_notebook(
            &path_str,
            &working_dir,
            Some(5),
            Some("nope".to_string()),
        );
        assert!(result.is_err());
    }
}
