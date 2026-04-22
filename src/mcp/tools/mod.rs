pub mod base64_codec;
pub mod calculator;
pub mod datetime;
pub mod dir_list;
pub mod env_get;
pub mod execute_command;
pub mod file_edit;
pub mod file_ops;
pub mod file_read;
pub mod file_search;
pub mod file_stat;
pub mod file_write;
pub mod git_ops;
pub mod hash_computer;
pub mod http_request;
pub mod image_read;
pub mod json_query;
pub mod path_exists;
pub mod process_list;
pub mod system_info;

/// Get all available tools with their descriptions and danger levels
pub fn get_all_tools() -> Vec<(String, String, bool)> {
    vec![
        (
            "dir_list".to_string(),
            "List directory contents with filtering and brief mode (max depth 5)".to_string(),
            false,
        ),
        (
            "file_read".to_string(),
            "Read text file content with line numbers and large range support".to_string(),
            false,
        ),
        (
            "file_search".to_string(),
            "Search for keyword and return matching content fragments with context (max depth 5)".to_string(),
            false,
        ),
        (
            "file_edit".to_string(),
            "Edit a file using string_replace, line_replace, insert, delete, or patch mode".to_string(),
            true,
        ),
        (
            "file_write".to_string(),
            "Write content to file (create/append/overwrite)".to_string(),
            true,
        ),
        (
            "file_ops".to_string(),
            "Copy, move, delete, or rename files".to_string(),
            true,
        ),
        (
            "file_stat".to_string(),
            "Get file or directory metadata (size, permissions, timestamps)".to_string(),
            false,
        ),
        (
            "path_exists".to_string(),
            "Check if a path exists and get its type (file/dir/symlink/none)".to_string(),
            false,
        ),
        (
            "json_query".to_string(),
            "Query a JSON file using JSON Pointer syntax".to_string(),
            false,
        ),
        (
            "git_ops".to_string(),
            "Run git commands (status, diff, log, branch, show) in a repository".to_string(),
            false,
        ),
        (
            "calculator".to_string(),
            "Calculate mathematical expressions".to_string(),
            false,
        ),
        (
            "http_request".to_string(),
            "Make HTTP requests with optional JSON extraction and response limiting".to_string(),
            false,
        ),
        (
            "datetime".to_string(),
            "Get current date and time in China format".to_string(),
            false,
        ),
        (
            "image_read".to_string(),
            "Read image file and return base64 data or metadata only".to_string(),
            false,
        ),
        (
            "execute_command".to_string(),
            "Execute shell command in specified directory (use with caution)".to_string(),
            true,
        ),
        (
            "process_list".to_string(),
            "List system processes".to_string(),
            false,
        ),
        (
            "base64_codec".to_string(),
            "Encode or decode base64 strings".to_string(),
            false,
        ),
        (
            "hash_compute".to_string(),
            "Compute hash of string or file (MD5, SHA1, SHA256). Prefix file path with 'file:' for files".to_string(),
            false,
        ),
        (
            "system_info".to_string(),
            "Get system information".to_string(),
            false,
        ),
        (
            "env_get".to_string(),
            "Get the value of an environment variable".to_string(),
            false,
        ),
    ]
}
