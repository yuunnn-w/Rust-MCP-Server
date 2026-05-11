use serde::{Deserialize, Serialize};

/// A predefined tool preset profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPreset {
    pub name: String,
    pub description: String,
    pub tools_enabled: Vec<String>,
    /// Whether ExecutePython filesystem access is enabled in this preset
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
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "FileStat".to_string(),
                "Git".to_string(),
                "ExecutePython".to_string(),
                "NoteStorage".to_string(),
                "Task".to_string(),
                "AskUser".to_string(),
            ],
            false,
        ),
        ToolPreset::new(
            "coding",
            "Coding & development: full file operations, git, commands, Python with FS access, web tools, Archive, NotebookEdit",
            vec![
                "Read".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "FileOps".to_string(),
                "FileStat".to_string(),
                "Bash".to_string(),
                "ExecutePython".to_string(),
                "SystemInfo".to_string(),
                "Git".to_string(),
                "Clipboard".to_string(),
                "Archive".to_string(),
                "Diff".to_string(),
                "NoteStorage".to_string(),
                "Task".to_string(),
                "AskUser".to_string(),
                "NotebookEdit".to_string(),
                "Monitor".to_string(),
            ],
            true,
        ),
        ToolPreset::new(
            "data_analysis",
            "Data analysis: Python with FS access, web tools, file reading/writing, Diff, Archive, NotebookEdit",
            vec![
                "Read".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "FileStat".to_string(),
                "ExecutePython".to_string(),
                "SystemInfo".to_string(),
                "WebFetch".to_string(),
                "Diff".to_string(),
                "Archive".to_string(),
                "Task".to_string(),
                "NoteStorage".to_string(),
                "NotebookEdit".to_string(),
            ],
            true,
        ),
        ToolPreset::new(
            "system_admin",
            "System administration: system info, processes, commands, Python with FS access, file operations, Archive, Monitor",
            vec![
                "Read".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "FileOps".to_string(),
                "FileStat".to_string(),
                "Bash".to_string(),
                "ExecutePython".to_string(),
                "SystemInfo".to_string(),
                "Git".to_string(),
                "Archive".to_string(),
                "Diff".to_string(),
                "NoteStorage".to_string(),
                "Task".to_string(),
                "AskUser".to_string(),
                "Monitor".to_string(),
            ],
            true,
        ),
        ToolPreset::new(
            "research",
            "Research & documentation: web search, content fetching, file reading, notes, task tracking, user elicitation, NotebookEdit",
            vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "FileStat".to_string(),
                "WebSearch".to_string(),
                "WebFetch".to_string(),
                "NoteStorage".to_string(),
                "Task".to_string(),
                "AskUser".to_string(),
                "NotebookEdit".to_string(),
            ],
            false,
        ),
        ToolPreset::new(
            "full_power",
            "Full power: all 21 tools enabled",
            vec![
                "Glob".to_string(),
                "Read".to_string(),
                "Grep".to_string(),
                "Edit".to_string(),
                "Write".to_string(),
                "FileOps".to_string(),
                "FileStat".to_string(),
            "Git".to_string(),
            "Bash".to_string(),
                "SystemInfo".to_string(),
                "ExecutePython".to_string(),
                "Clipboard".to_string(),
                "Archive".to_string(),
                "Diff".to_string(),
                "NoteStorage".to_string(),
                "Task".to_string(),
                "WebSearch".to_string(),
                "AskUser".to_string(),
                "WebFetch".to_string(),
                "NotebookEdit".to_string(),
                "Monitor".to_string(),
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
        assert!(get_preset("data_analysis").is_some());
        assert!(get_preset("system_admin").is_some());
        assert!(get_preset("research").is_some());
        assert!(get_preset("full_power").is_some());
        assert!(get_preset("nonexistent").is_none());
        assert!(get_preset("document").is_none());
    }

    #[test]
    fn test_full_power_has_all_tools() {
        let full = get_preset("full_power").unwrap();
        assert_eq!(full.tools_enabled.len(), 21);
    }

    #[test]
    fn test_preset_python_fs_access() {
        assert!(!get_preset("minimal").unwrap().python_fs_access_enabled);
        assert!(get_preset("coding").unwrap().python_fs_access_enabled);
        assert!(get_preset("data_analysis").unwrap().python_fs_access_enabled);
        assert!(get_preset("system_admin").unwrap().python_fs_access_enabled);
        assert!(!get_preset("research").unwrap().python_fs_access_enabled);
        assert!(get_preset("full_power").unwrap().python_fs_access_enabled);
    }
}
