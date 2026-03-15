pub mod base64_codec;
pub mod calculator;
pub mod datetime;
pub mod dir_list;
pub mod execute_command;
pub mod file_ops;
pub mod file_read;
pub mod file_search;
pub mod file_write;
pub mod hash_computer;
pub mod http_request;
pub mod image_read;
pub mod process_list;
pub mod system_info;



/// Get all available tools with their descriptions and danger levels
pub fn get_all_tools() -> Vec<(String, String, bool)> {
    vec![
        (
            "dir_list".to_string(),
            "List directory contents with tree structure (max depth 3)".to_string(),
            false,
        ),
        (
            "file_read".to_string(),
            "Read text file content with line range support".to_string(),
            false,
        ),
        (
            "file_search".to_string(),
            "Search for keyword in file or directory (max depth 3)".to_string(),
            false,
        ),
        (
            "file_write".to_string(),
            "Write content to file (create/append/overwrite)".to_string(),
            true,
        ),
        (
            "file_copy".to_string(),
            "Copy a file to a new location".to_string(),
            true,
        ),
        (
            "file_move".to_string(),
            "Move a file to a new location".to_string(),
            true,
        ),
        (
            "file_delete".to_string(),
            "Delete a file".to_string(),
            true,
        ),
        (
            "file_rename".to_string(),
            "Rename a file".to_string(),
            true,
        ),
        (
            "calculator".to_string(),
            "Calculate mathematical expressions".to_string(),
            false,
        ),
        (
            "http_request".to_string(),
            "Make HTTP GET or POST requests".to_string(),
            false,
        ),
        (
            "datetime".to_string(),
            "Get current date and time in China format".to_string(),
            false,
        ),
        (
            "image_read".to_string(),
            "Read image file and return base64 encoded data".to_string(),
            false,
        ),
        (
            "execute_command".to_string(),
            "Execute shell command in specified directory".to_string(),
            true,
        ),
        (
            "process_list".to_string(),
            "List system processes".to_string(),
            false,
        ),
        (
            "base64_encode".to_string(),
            "Encode string to base64".to_string(),
            false,
        ),
        (
            "base64_decode".to_string(),
            "Decode base64 to string".to_string(),
            false,
        ),
        (
            "hash_compute".to_string(),
            "Compute hash of string or file (MD5, SHA1, SHA256)".to_string(),
            false,
        ),
        (
            "system_info".to_string(),
            "Get system information".to_string(),
            false,
        ),
    ]
}
