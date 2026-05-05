use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArchiveParams {
    /// Operation: create, extract, list, or append
    pub operation: String,
    /// Path to the ZIP archive
    pub archive_path: String,
    /// For create/append: list of file or directory paths to include
    pub source_paths: Option<Vec<String>>,
    /// For extract: destination directory (default: working_dir)
    pub destination: Option<String>,
    /// Compression level 1-9 (default: 6), only for create
    pub compression_level: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ArchiveListEntry {
    name: String,
    size: u64,
    compressed_size: u64,
    last_modified: String,
}

pub async fn archive(params: Parameters<ArchiveParams>, working_dir: &Path) -> Result<CallToolResult, String> {
    let p = params.0;
    let op = p.operation.to_lowercase();
    let archive_path = crate::utils::file_utils::ensure_path_within_working_dir(
        Path::new(&p.archive_path),
        working_dir,
    ).map_err(|e| e.to_string())?;

    match op.as_str() {
        "create" => {
            let sources = p.source_paths.ok_or("Missing 'source_paths' for create operation")?;
            let level = p.compression_level.unwrap_or(6).clamp(1, 9);
            let file = File::create(&archive_path)
                .map_err(|e| format!("Failed to create archive file: {}", e))?;
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .compression_level(Some(level.try_into().unwrap_or(6)));

            for src in sources {
                let src_path = crate::utils::file_utils::ensure_path_within_working_dir(
                    Path::new(&src),
                    working_dir,
                ).map_err(|e| e.to_string())?;

                if src_path.is_dir() {
                    add_dir_to_zip(&mut writer, &src_path, &src_path, options)
                        .map_err(|e| format!("Failed to add directory '{}': {}", src, e))?;
                } else if src_path.is_file() {
                    let name_in_zip = src_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("file")
                        .to_string();
                    writer.start_file(name_in_zip, options)
                        .map_err(|e| format!("Failed to start file '{}': {}", src, e))?;
                    let mut f = File::open(&src_path)
                        .map_err(|e| format!("Failed to open '{}': {}", src, e))?;
                    let mut buf = Vec::new();
                    f.read_to_end(&mut buf)
                        .map_err(|e| format!("Failed to read '{}': {}", src, e))?;
                    writer.write_all(&buf)
                        .map_err(|e| format!("Failed to write '{}': {}", src, e))?;
                }
            }
            writer.finish()
                .map_err(|e| format!("Failed to finalize archive: {}", e))?;
            Ok(CallToolResult::success(vec![
                rmcp::model::Content::text(format!("Archive created successfully: {}", archive_path.display())),
            ]))
        }
        "extract" => {
            let dest = if let Some(d) = p.destination {
                crate::utils::file_utils::ensure_path_within_working_dir(Path::new(&d), working_dir)
                    .map_err(|e| e.to_string())?
            } else {
                working_dir.to_path_buf()
            };
            std::fs::create_dir_all(&dest)
                .map_err(|e| format!("Failed to create destination directory: {}", e))?;
            let file = File::open(&archive_path)
                .map_err(|e| format!("Failed to open archive: {}", e))?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| format!("Failed to read archive: {}", e))?;
            let mut extracted = Vec::new();
            for i in 0..archive.len() {
                let mut entry = archive.by_index(i)
                    .map_err(|e| format!("Failed to read archive entry {}: {}", i, e))?;
                let out_path = dest.join(entry.name());
                if entry.is_dir() {
                    std::fs::create_dir_all(&out_path)
                        .map_err(|e| format!("Failed to create directory '{}': {}", out_path.display(), e))?;
                } else {
                    if let Some(parent) = out_path.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
                    }
                    let mut out_file = File::create(&out_path)
                        .map_err(|e| format!("Failed to create file '{}': {}", out_path.display(), e))?;
                    let mut buf = [0u8; 8192];
                    loop {
                        let n = entry.read(&mut buf)
                            .map_err(|e| format!("Failed to read from archive: {}", e))?;
                        if n == 0 { break; }
                        out_file.write_all(&buf[..n])
                            .map_err(|e| format!("Failed to write file: {}", e))?;
                    }
                }
                extracted.push(entry.name().to_string());
            }
            Ok(CallToolResult::success(vec![
                rmcp::model::Content::text(format!("Extracted {} entries to {}", extracted.len(), dest.display())),
            ]))
        }
        "list" => {
            let file = File::open(&archive_path)
                .map_err(|e| format!("Failed to open archive: {}", e))?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| format!("Failed to read archive: {}", e))?;
            let mut entries = Vec::new();
            for i in 0..archive.len() {
                let entry = archive.by_index(i)
                    .map_err(|e| format!("Failed to read entry {}: {}", i, e))?;
                entries.push(ArchiveListEntry {
                    name: entry.name().to_string(),
                    size: entry.size(),
                    compressed_size: entry.compressed_size(),
                    last_modified: entry.last_modified().map(|d| d.to_string()).unwrap_or_default(),
                });
            }
            let json = serde_json::to_string_pretty(&entries)
                .map_err(|e| format!("Failed to serialize entries: {}", e))?;
            Ok(CallToolResult::success(vec![
                rmcp::model::Content::text(json),
            ]))
        }
        "append" => {
            let sources = p.source_paths.ok_or("Missing 'source_paths' for append operation")?;
            let level = p.compression_level.unwrap_or(6).clamp(1, 9);
            // Read existing archive into memory
            let mut existing_data = Vec::new();
            if archive_path.exists() {
                let mut f = File::open(&archive_path)
                    .map_err(|e| format!("Failed to open existing archive: {}", e))?;
                f.read_to_end(&mut existing_data)
                    .map_err(|e| format!("Failed to read existing archive: {}", e))?;
            }
            let mut new_data = Vec::new();
            {
                let mut writer = if existing_data.is_empty() {
                    zip::ZipWriter::new(std::io::Cursor::new(&mut new_data))
                } else {
                    zip::ZipWriter::new_append(std::io::Cursor::new(&mut new_data))
                        .map_err(|e| format!("Failed to open archive for append: {}", e))?
                };
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .compression_level(Some(level.try_into().unwrap_or(6)));

                for src in sources {
                    let src_path = crate::utils::file_utils::ensure_path_within_working_dir(
                        Path::new(&src),
                        working_dir,
                    ).map_err(|e| e.to_string())?;
                    if src_path.is_file() {
                        let name_in_zip = src_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("file")
                            .to_string();
                        writer.start_file(name_in_zip, options)
                            .map_err(|e| format!("Failed to start file '{}': {}", src, e))?;
                        let mut f = File::open(&src_path)
                            .map_err(|e| format!("Failed to open '{}': {}", src, e))?;
                        let mut buf = Vec::new();
                        f.read_to_end(&mut buf)
                            .map_err(|e| format!("Failed to read '{}': {}", src, e))?;
                        writer.write_all(&buf)
                            .map_err(|e| format!("Failed to write '{}': {}", src, e))?;
                    } else if src_path.is_dir() {
                        add_dir_to_zip(&mut writer, &src_path, &src_path, options)
                            .map_err(|e| format!("Failed to add directory '{}': {}", src, e))?;
                    }
                }
                writer.finish()
                    .map_err(|e| format!("Failed to finalize archive: {}", e))?;
            }
            // Write back
            let mut f = File::create(&archive_path)
                .map_err(|e| format!("Failed to write archive: {}", e))?;
            f.write_all(&new_data)
                .map_err(|e| format!("Failed to write archive data: {}", e))?;
            Ok(CallToolResult::success(vec![
                rmcp::model::Content::text(format!("Archive appended successfully: {}", archive_path.display())),
            ]))
        }
        _ => Err(format!("Unknown archive operation: '{}'. Supported: create, extract, list, append", p.operation)),
    }
}

fn add_dir_to_zip<W: Write + std::io::Seek>(
    writer: &mut zip::ZipWriter<W>,
    base_dir: &Path,
    current_dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    for entry in std::fs::read_dir(current_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name_in_zip = path.strip_prefix(base_dir)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .to_string();
        if path.is_dir() {
            writer.add_directory(name_in_zip + "/", options)
                .map_err(|e| e.to_string())?;
            add_dir_to_zip(writer, base_dir, &path, options)?;
        } else if path.is_file() {
            writer.start_file(name_in_zip, options)
                .map_err(|e| e.to_string())?;
            let mut f = File::open(&path).map_err(|e| e.to_string())?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            writer.write_all(&buf).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

use rmcp::handler::server::wrapper::Parameters;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_archive_create_and_list() {
        let temp_dir = tempfile::tempdir().unwrap();
        let working_dir = temp_dir.path();
        let archive_path = working_dir.join("test.zip");
        let test_file = working_dir.join("hello.txt");
        std::fs::write(&test_file, "Hello, Archive!").unwrap();

        // Create
        let create_params = Parameters(ArchiveParams {
            operation: "create".to_string(),
            archive_path: archive_path.to_string_lossy().to_string(),
            source_paths: Some(vec![test_file.to_string_lossy().to_string()]),
            destination: None,
            compression_level: None,
        });
        let result = archive(create_params, working_dir).await;
        assert!(result.is_ok(), "Create failed: {:?}", result);

        // List
        let list_params = Parameters(ArchiveParams {
            operation: "list".to_string(),
            archive_path: archive_path.to_string_lossy().to_string(),
            source_paths: None,
            destination: None,
            compression_level: None,
        });
        let result = archive(list_params, working_dir).await;
        assert!(result.is_ok());
    }
}
