pub mod archive;
pub mod clipboard;
pub mod diff;
pub mod glob;
pub mod execute_python;
pub mod bash;
pub mod edit;
pub mod file_ops;
pub mod read;
pub mod grep;
pub mod file_stat;
pub mod write;
pub mod git_ops;
pub mod note_storage;
pub mod system_info;
pub mod task;
pub mod web_search;
pub mod ask_user;
pub mod web_fetch;
pub mod notebook_edit;
pub mod monitor;

/// Get all available tools with their descriptions and danger levels
pub fn get_all_tools() -> Vec<(String, String, bool)> {
    vec![
        (
            "Glob".to_string(),
            "List directory contents with enhanced filtering (max depth 10). Supports multi-pattern glob/regex matching, exclude patterns, file type/size/time filters, sort order control, and symlink following. Returns text file char_count and line_count for UTF-8 files. Not restricted to working directory.".to_string(),
            false,
        ),
        (
            "Read".to_string(),
            "Read a file with format auto-detection. Mode guide: 'auto' (generic text/image auto-detect); 'text' (plain text with line range/offset); 'media' (base64 image for vision models). For DOC/DOCX: 'doc_text' (markdown with headings/tables/formatting), 'doc_with_images' (markdown with images inline at positions), 'doc_images' (extracted images only). For PPT/PPTX: 'ppt_text' (slide text with tables), 'ppt_images' (slides as images; uses LibreOffice if available, falls back to native extraction of embedded images+text per slide). For PDF: 'pdf_text' (extracted text), 'pdf_images' (pages rendered to images via PDFium). For XLS/XLSX: 'text' (sheet tables). For IPYNB: 'text' (cells with outputs). Recommendation: use FileStat first to check document stats (slide/page count, image count, text length), then choose mode accordingly. Image modes return base64-encoded ImageContent. Batch mode via 'paths' parameter. Not restricted to working directory.".to_string(),
            false,
        ),
        (
            "Grep".to_string(),
            "Search pattern in files with enhanced filtering (max depth 10). Supports regex, case-sensitive, whole-word, multiline modes. Searches office documents (DOCX/PPTX/XLSX/PDF/IPYNB) text content. File filtering via include/exclude glob patterns. Not restricted to working directory.".to_string(),
            false,
        ),
        (
            "Edit".to_string(),
            "Edit files concurrently. Text modes: string_replace, line_replace, insert, delete, patch. Office: office_insert, office_replace, office_delete, office_insert_image, office_format, office_insert_table. PDF: pdf_delete_page, pdf_insert_image, pdf_insert_text, pdf_replace_text. Can create new files.".to_string(),
            true,
        ),
        (
            "Write".to_string(),
            "Write content to files concurrently (create/append/overwrite). Supports office documents: DOCX (docx_paragraphs or office_markdown), XLSX (xlsx_sheets or office_csv), PPTX (pptx_slides), PDF (office_markdown via LibreOffice), IPYNB (ipynb_cells).".to_string(),
            true,
        ),
        (
            "FileOps".to_string(),
            "Copy, move, delete, or rename one or more files concurrently. Supports dry_run preview and conflict_resolution (skip/overwrite/rename). Accepts a list of operations.".to_string(),
            true,
        ),
        (
            "FileStat".to_string(),
            "Get metadata for one or more files or directories concurrently. Use mode=\"exist\" for lightweight existence check. For regular files: returns is_text, char_count, line_count, encoding (UTF-8), size, permissions, timestamps. For office documents (DOCX/PPTX/PDF/XLSX): additionally returns document_stats with document_type, page/slide/sheet count, embedded image count, and text character count to help decide whether to use text or image reading mode. Use FileStat before Read to choose the optimal mode (e.g., if PDF has many images, use pdf_images; if PPTX has many slides but few images, use ppt_text). Not restricted to working directory.".to_string(),
            false,
        ),
        (
            "Git".to_string(),
            "Run git commands (status, diff, log, branch, show). Supports path filtering and max_count for log. Not restricted to working directory.".to_string(),
            false,
        ),

        (
            "Bash".to_string(),
            "Execute shell command with optional working_dir, stdin, max_output_chars, and async_mode. Use Monitor tool for async commands.".to_string(),
            true,
        ),
        (
            "SystemInfo".to_string(),
            "Get system information including processes. Use 'sections' to select sections: system, cpu, memory, disks, network, temperature, processes.".to_string(),
            false,
        ),
        (
            "ExecutePython".to_string(),
            "Execute Python code for calculations, data processing, and logic evaluation. Set __result for return value. All Python standard library modules are available.".to_string(),
            false,
        ),
        (
            "Clipboard".to_string(),
            "Read or write system clipboard content. Supports read_text, write_text, read_image, and clear operations. Optional format parameter (text/html/rtf). Cross-platform.".to_string(),
            false,
        ),
        (
            "Archive".to_string(),
            "Create, extract, list, or append ZIP archives. Supports deflate and zstd compression plus AES-256 password encryption. Restricted to working directory.".to_string(),
            true,
        ),
        (
            "Diff".to_string(),
            "Compare text, files, or directories. Output formats: unified, side_by_side, summary, inline. Supports git_diff_file, ignore_blank_lines, and configurable context_lines. Compares against HEAD.".to_string(),
            false,
        ),
        (
            "NoteStorage".to_string(),
            "The AI assistant's short-term memory scratchpad. Creates, lists, reads, updates, deletes, searches, and appends notes. Supports export to JSON and import from JSON. Notes are stored only in memory and auto-expire after 30 minutes.".to_string(),
            false,
        ),
        (
            "Task".to_string(),
            "Task management with CRUD operations. Use 'operation' parameter: create, list, get, update, delete.".to_string(),
            false,
        ),
        (
            "WebSearch".to_string(),
            "Search the web via DuckDuckGo with optional region/language filters. Returns results with titles, URLs, and snippets.".to_string(),
            false,
        ),
        (
            "AskUser".to_string(),
            "Ask the user a question with optional timeout and default_value. Supports multi-choice options via MCP elicitation.".to_string(),
            false,
        ),
        (
            "WebFetch".to_string(),
            "Fetch content from a URL with extract_mode: text (strips HTML), html (raw), or markdown.".to_string(),
            false,
        ),
        (
            "NotebookEdit".to_string(),
            "Read, write, and edit Jupyter .ipynb notebook files. Operations: read, write, add_cell, edit_cell, delete_cell.".to_string(),
            true,
        ),
        (
            "Monitor".to_string(),
            "Monitor long-running Bash commands started with async=true. Operations: stream, wait, signal.".to_string(),
            false,
        ),
    ]
}
