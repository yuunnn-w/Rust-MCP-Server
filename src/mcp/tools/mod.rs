pub mod archive;
pub mod base64_codec;
pub mod calculator;
pub mod clipboard;
pub mod datetime;
pub mod diff;
pub mod dir_list;
pub mod env_get;
pub mod execute_python;
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
pub mod note_storage;
pub mod path_exists;
pub mod process_list;
pub mod system_info;

/// Get all available tools with their descriptions and danger levels
pub fn get_all_tools() -> Vec<(String, String, bool)> {
    vec![
        (
            "dir_list".to_string(),
            "List directory contents with filtering and brief mode (max depth 5). Not restricted to working directory.".to_string(),
            false,
        ),
        (
            "file_read".to_string(),
            "Read text file content with line numbers and large range support. Not restricted to working directory.".to_string(),
            false,
        ),
        (
            "file_search".to_string(),
            "Search for keyword and return matching content fragments with context (max depth 5). Not restricted to working directory.".to_string(),
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
            "Get file or directory metadata (size, permissions, timestamps). Not restricted to working directory.".to_string(),
            false,
        ),
        (
            "path_exists".to_string(),
            "Check if a path exists and get its type (file/dir/symlink/none). Not restricted to working directory.".to_string(),
            false,
        ),
        (
            "json_query".to_string(),
            "Query a JSON file using JSON Pointer syntax. Not restricted to working directory.".to_string(),
            false,
        ),
        (
            "git_ops".to_string(),
            "Run git commands (status, diff, log, branch, show) in a repository. Not restricted to working directory.".to_string(),
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
            "Read image file and return base64 data or metadata only. Not restricted to working directory.".to_string(),
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
            "Compute hash of string or file (MD5, SHA1, SHA256). Prefix file path with 'file:' for files. Not restricted to working directory.".to_string(),
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
        (
            "execute_python".to_string(),
            "Execute Python code in a sandboxed environment for calculations, data processing, and logic evaluation. By default, filesystem access is disabled (safe). Set __result for return value. Available standard library modules: math, random, statistics, datetime, itertools, functools, collections, re, string, json, fractions, decimal, typing, hashlib, base64, bisect, heapq, copy, pprint, enum, types, dataclasses, inspect, sys.".to_string(),
            false,
        ),
        (
            "clipboard".to_string(),
            "Read or write system clipboard content (text or image). Cross-platform support for Windows, Linux, and macOS.".to_string(),
            false,
        ),
        (
            "archive".to_string(),
            "Create, extract, list, or append ZIP archives. Supports deflate and zstd compression. Restricted to working directory.".to_string(),
            true,
        ),
        (
            "diff".to_string(),
            "Compare text, files, or directories with unified diff, side-by-side, summary, or inline word-level output. Supports git HEAD comparison.".to_string(),
            false,
        ),
        (
            "note_storage".to_string(),
            "The AI assistant's short-term memory scratchpad. Use it to temporarily store intermediate results, task sub-steps, context snippets, or working hypotheses during the current conversation or task. Notes are stored only in memory and are automatically erased if not used for 30 minutes. Do not use this for long-term persistence—use it as a thinking workspace to offload complex reasoning or maintain state across multiple tool calls within a session.".to_string(),
            false,
        ),
    ]
}
