use serde::{Deserialize, Serialize};

/// A predefined tool preset profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPreset {
    pub name: String,
    pub description: String,
    pub tools_enabled: Vec<String>,
    /// Whether execute_python filesystem access is enabled in this preset
    pub python_fs_access_enabled: bool,
}

impl ToolPreset {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        tools_enabled: Vec<String>,
        python_fs_access_enabled: bool,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            tools_enabled,
            python_fs_access_enabled,
        }
    }
}

/// Get all predefined presets
pub fn get_all_presets() -> Vec<ToolPreset> {
    vec![
        ToolPreset::new(
            "minimal",
            "Minimal safe mode: read-only, computation, and sandboxed Python tools only",
            vec![
                "calculator".to_string(),
                "dir_list".to_string(),
                "file_read".to_string(),
                "file_search".to_string(),
                "file_stat".to_string(),
                "path_exists".to_string(),
                "json_query".to_string(),
                "image_read".to_string(),
                "env_get".to_string(),
                "base64_codec".to_string(),
                "hash_compute".to_string(),
                "datetime".to_string(),
                "diff".to_string(),
                "note_storage".to_string(),
                "clipboard".to_string(),
                "execute_python".to_string(),
            ],
            false,
        ),
        ToolPreset::new(
            "coding",
            "Coding & development: full file operations, git, commands, Python with FS access, HTTP, clipboard, archive",
            vec![
                "calculator".to_string(),
                "dir_list".to_string(),
                "file_read".to_string(),
                "file_search".to_string(),
                "file_stat".to_string(),
                "path_exists".to_string(),
                "json_query".to_string(),
                "image_read".to_string(),
                "env_get".to_string(),
                "base64_codec".to_string(),
                "hash_compute".to_string(),
                "datetime".to_string(),
                "diff".to_string(),
                "note_storage".to_string(),
                "clipboard".to_string(),
                "execute_python".to_string(),
                "file_edit".to_string(),
                "file_write".to_string(),
                "file_ops".to_string(),
                "git_ops".to_string(),
                "execute_command".to_string(),
                "http_request".to_string(),
                "archive".to_string(),
            ],
            true,
        ),
        ToolPreset::new(
            "document",
            "Document processing: file editing, images, search, encoding, clipboard, diff, notes, calculator",
            vec![
                "file_read".to_string(),
                "file_search".to_string(),
                "file_write".to_string(),
                "file_edit".to_string(),
                "image_read".to_string(),
                "dir_list".to_string(),
                "path_exists".to_string(),
                "file_stat".to_string(),
                "hash_compute".to_string(),
                "base64_codec".to_string(),
                "datetime".to_string(),
                "json_query".to_string(),
                "diff".to_string(),
                "note_storage".to_string(),
                "clipboard".to_string(),
                "calculator".to_string(),
            ],
            false,
        ),
        ToolPreset::new(
            "data_analysis",
            "Data analysis: Python with FS access, calculations, HTTP, JSON, file reading/writing, diff, clipboard, archive",
            vec![
                "calculator".to_string(),
                "execute_python".to_string(),
                "json_query".to_string(),
                "file_read".to_string(),
                "file_stat".to_string(),
                "hash_compute".to_string(),
                "base64_codec".to_string(),
                "http_request".to_string(),
                "env_get".to_string(),
                "dir_list".to_string(),
                "diff".to_string(),
                "note_storage".to_string(),
                "clipboard".to_string(),
                "file_write".to_string(),
                "file_edit".to_string(),
                "image_read".to_string(),
                "path_exists".to_string(),
                "archive".to_string(),
            ],
            true,
        ),
        ToolPreset::new(
            "system_admin",
            "System administration: system info, processes, commands, Python with FS access, file operations, archive",
            vec![
                "system_info".to_string(),
                "process_list".to_string(),
                "execute_command".to_string(),
                "env_get".to_string(),
                "execute_python".to_string(),
                "dir_list".to_string(),
                "file_read".to_string(),
                "file_stat".to_string(),
                "calculator".to_string(),
                "base64_codec".to_string(),
                "hash_compute".to_string(),
                "datetime".to_string(),
                "http_request".to_string(),
                "archive".to_string(),
                "diff".to_string(),
                "note_storage".to_string(),
                "file_write".to_string(),
                "file_edit".to_string(),
                "file_ops".to_string(),
                "clipboard".to_string(),
            ],
            true,
        ),
        ToolPreset::new(
            "full_power",
            "Full power: all 25 tools enabled",
            vec![
                "dir_list".to_string(),
                "file_read".to_string(),
                "file_search".to_string(),
                "file_edit".to_string(),
                "file_write".to_string(),
                "file_ops".to_string(),
                "file_stat".to_string(),
                "path_exists".to_string(),
                "json_query".to_string(),
                "git_ops".to_string(),
                "calculator".to_string(),
                "http_request".to_string(),
                "datetime".to_string(),
                "image_read".to_string(),
                "execute_command".to_string(),
                "process_list".to_string(),
                "base64_codec".to_string(),
                "hash_compute".to_string(),
                "system_info".to_string(),
                "env_get".to_string(),
                "execute_python".to_string(),
                "clipboard".to_string(),
                "archive".to_string(),
                "diff".to_string(),
                "note_storage".to_string(),
            ],
            true,
        ),
    ]
}

/// Get a preset by name
pub fn get_preset(name: &str) -> Option<ToolPreset> {
    get_all_presets().into_iter().find(|p| p.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_counts() {
        let presets = get_all_presets();
        assert_eq!(presets.len(), 6);
        assert!(get_preset("minimal").is_some());
        assert!(get_preset("coding").is_some());
        assert!(get_preset("document").is_some());
        assert!(get_preset("data_analysis").is_some());
        assert!(get_preset("system_admin").is_some());
        assert!(get_preset("full_power").is_some());
        assert!(get_preset("nonexistent").is_none());
    }

    #[test]
    fn test_full_power_has_all_tools() {
        let full = get_preset("full_power").unwrap();
        assert_eq!(full.tools_enabled.len(), 25);
    }

    #[test]
    fn test_preset_python_fs_access() {
        assert_eq!(get_preset("minimal").unwrap().python_fs_access_enabled, false);
        assert_eq!(get_preset("coding").unwrap().python_fs_access_enabled, true);
        assert_eq!(get_preset("document").unwrap().python_fs_access_enabled, false);
        assert_eq!(get_preset("data_analysis").unwrap().python_fs_access_enabled, true);
        assert_eq!(get_preset("system_admin").unwrap().python_fs_access_enabled, true);
        assert_eq!(get_preset("full_power").unwrap().python_fs_access_enabled, true);
    }
}
