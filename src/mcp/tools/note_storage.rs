use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NoteStorageParams {
    #[schemars(description = "Operation: create, list, read, update, delete, search, append")]
    pub operation: String,
    #[schemars(description = "Note ID (required for read, update, delete, append)")]
    pub id: Option<u64>,
    #[schemars(description = "Note title (for create, update)")]
    pub title: Option<String>,
    #[schemars(description = "Note content (for create, update)")]
    pub content: Option<String>,
    #[schemars(description = "Tags list (for create, update)")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Category (for create, update, list filter)")]
    pub category: Option<String>,
    #[schemars(description = "Tag filter (for list)")]
    pub tag_filter: Option<String>,
    #[schemars(description = "Search query (for search)")]
    pub query: Option<String>,
    #[schemars(description = "Content to append (for append)")]
    pub append_content: Option<String>,
    #[schemars(description = "Export all notes as JSON (set to true)")]
    pub export: Option<bool>,
    #[schemars(description = "JSON data to import notes from")]
    pub import_data: Option<String>,
}

pub async fn note_storage(params: Parameters<NoteStorageParams>, state: Arc<crate::mcp::state::ServerState>) -> Result<CallToolResult, String> {
    let p = params.0;
    let op = p.operation.to_lowercase();

    if p.export.unwrap_or(false) {
        let notes = state.note_list(None, None).await;
        let json = serde_json::to_string_pretty(&notes).map_err(|e| e.to_string())?;
        return Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]));
    }

    if let Some(ref import_json) = p.import_data {
        let note_values: Vec<serde_json::Value> = serde_json::from_str(import_json)
            .map_err(|e| format!("Failed to parse import_data JSON: {}", e))?;
        // Pre-parse and validate all notes before creating any (atomic import)
        let mut parsed_notes: Vec<(String, String, Vec<String>, String)> = Vec::new();
        for (idx, note_val) in note_values.iter().enumerate() {
            let title = match &note_val["title"] {
                serde_json::Value::String(s) if !s.trim().is_empty() => s.clone(),
                serde_json::Value::String(_) => return Err(format!("Import failed at index {}: title must not be empty", idx)),
                serde_json::Value::Null => "Untitled".to_string(),
                _ => return Err(format!("Import failed at index {}: 'title' must be a string or null", idx)),
            };
            let content = match &note_val["content"] {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                _ => return Err(format!("Import failed at index {}: 'content' must be a string or null", idx)),
            };
            let tags: Vec<String> = match &note_val["tags"] {
                serde_json::Value::Array(arr) => {
                    let mut tags = Vec::new();
                    for (ti, tag_val) in arr.iter().enumerate() {
                        match tag_val {
                            serde_json::Value::String(s) => tags.push(s.clone()),
                            serde_json::Value::Null => {}
                            _ => return Err(format!("Import failed at index {}: 'tags[{}]' must be a string", idx, ti)),
                        }
                    }
                    tags
                }
                serde_json::Value::Null => Vec::new(),
                _ => return Err(format!("Import failed at index {}: 'tags' must be an array or null", idx)),
            };
            let category = match &note_val["category"] {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                _ => return Err(format!("Import failed at index {}: 'category' must be a string or null", idx)),
            };
            parsed_notes.push((title, content, tags, category));
        }
        let mut imported = 0usize;
        for (title, content, tags, category) in parsed_notes {
            state.note_create(title, content, tags, category).await?;
            imported += 1;
        }
        return Ok(CallToolResult::success(vec![
            rmcp::model::Content::text(format!("Imported {} notes successfully", imported)),
        ]));
    }

    match op.as_str() {
        "create" => {
            let title = p.title.ok_or("Missing 'title' for create")?;
            let content = p.content.unwrap_or_default();
            let tags = p.tags.unwrap_or_default();
            let category = p.category.unwrap_or_default();
            let note = state.note_create(title, content, tags, category).await?;
            let json = serde_json::to_string_pretty(&note).map_err(|e| e.to_string())?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
        }
        "list" => {
            let notes = state.note_list(p.tag_filter, p.category).await;
            let json = serde_json::to_string_pretty(&notes).map_err(|e| e.to_string())?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
        }
        "read" => {
            let id = p.id.ok_or("Missing 'id' for read")?;
            match state.note_read(id).await {
                Some(note) => {
                    let json = serde_json::to_string_pretty(&note).map_err(|e| e.to_string())?;
                    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
                }
                None => Err(format!("Note {} not found", id)),
            }
        }
        "update" => {
            let id = p.id.ok_or("Missing 'id' for update")?;
            let note = state.note_update(id, p.title, p.content, p.tags, p.category).await?;
            let json = serde_json::to_string_pretty(&note).map_err(|e| e.to_string())?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
        }
        "delete" => {
            let id = p.id.ok_or("Missing 'id' for delete")?;
            state.note_delete(id).await?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(format!("Note {} deleted", id))]))
        }
        "search" => {
            let query = p.query.ok_or("Missing 'query' for search")?;
            let notes = state.note_search(&query).await;
            let json = serde_json::to_string_pretty(&notes).map_err(|e| e.to_string())?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
        }
        "append" => {
            let id = p.id.ok_or("Missing 'id' for append")?;
            let append = p.append_content.ok_or("Missing 'append_content' for append")?;
            let note = state.note_append(id, &append).await?;
            let json = serde_json::to_string_pretty(&note).map_err(|e| e.to_string())?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
        }
        _ => Err(format!("Unknown note_storage operation: '{}'. Supported: create, list, read, update, delete, search, append", p.operation)),
    }
}

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;
    use crate::mcp::state::ServerState;

    #[tokio::test]
    async fn test_note_lifecycle() {
        let config = AppConfig {
            webui_host: "127.0.0.1".to_string(),
            webui_port: 2233,
            mcp_transport: "http".to_string(),
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 3344,
            max_concurrency: 10,
            disable_tools: vec![],
            working_dir: std::path::PathBuf::from("."),
            log_level: "info".to_string(),
            disable_webui: false,
            allow_dangerous_commands: vec![],
            allowed_hosts: None,
            disable_allowed_hosts: false,
            preset: "minimal".to_string(),
            system_prompt: None,
        };
        let state = ServerState::new(config);

        // Create
        let note = state.note_create("Test".to_string(), "Content".to_string(), vec!["tag1".to_string()], "cat".to_string()).await.unwrap();
        assert_eq!(note.title, "Test");

        // Read
        let read = state.note_read(note.id).await.unwrap();
        assert_eq!(read.content, "Content");

        // Update
        let updated = state.note_update(note.id, Some("New Title".to_string()), None, None, None).await.unwrap();
        assert_eq!(updated.title, "New Title");

        // Append
        let appended = state.note_append(note.id, "More content").await.unwrap();
        assert!(appended.content.contains("More content"));

        // Search
        let results = state.note_search("Content").await;
        assert!(!results.is_empty());

        // List
        let list = state.note_list(None, None).await;
        assert_eq!(list.len(), 1);

        // Delete
        state.note_delete(note.id).await.unwrap();
        assert!(state.note_read(note.id).await.is_none());
    }

    #[tokio::test]
    async fn test_note_timeout() {
        let config = AppConfig {
            webui_host: "127.0.0.1".to_string(),
            webui_port: 2233,
            mcp_transport: "http".to_string(),
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 3344,
            max_concurrency: 10,
            disable_tools: vec![],
            working_dir: std::path::PathBuf::from("."),
            log_level: "info".to_string(),
            disable_webui: false,
            allow_dangerous_commands: vec![],
            allowed_hosts: None,
            disable_allowed_hosts: false,
            preset: "minimal".to_string(),
            system_prompt: None,
        };
        let state = ServerState::new(config);
        state.note_create("Test".to_string(), "Content".to_string(), vec![], "".to_string()).await.unwrap();

        // Manually set last access to 31 minutes ago to trigger timeout
        {
            let mut guard = state.notes_last_access.write().await;
            *guard = std::time::Instant::now() - std::time::Duration::from_secs(31 * 60);
        }

        state.check_notes_timeout().await;
        let list = state.note_list(None, None).await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_note_search_title() {
        let config = AppConfig {
            webui_host: "127.0.0.1".to_string(),
            webui_port: 2233,
            mcp_transport: "http".to_string(),
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 3344,
            max_concurrency: 10,
            disable_tools: vec![],
            working_dir: std::path::PathBuf::from("."),
            log_level: "info".to_string(),
            disable_webui: false,
            allow_dangerous_commands: vec![],
            allowed_hosts: None,
            disable_allowed_hosts: false,
            preset: "minimal".to_string(),
            system_prompt: None,
        };
        let state = ServerState::new(config);
        state.note_create("Rust Tips".to_string(), "Some content".to_string(), vec!["coding".to_string()], "dev".to_string()).await.unwrap();

        let results = state.note_search("rust").await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Tips");
    }

    #[tokio::test]
    async fn test_note_search_tags() {
        let config = AppConfig {
            webui_host: "127.0.0.1".to_string(),
            webui_port: 2233,
            mcp_transport: "http".to_string(),
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 3344,
            max_concurrency: 10,
            disable_tools: vec![],
            working_dir: std::path::PathBuf::from("."),
            log_level: "info".to_string(),
            disable_webui: false,
            allow_dangerous_commands: vec![],
            allowed_hosts: None,
            disable_allowed_hosts: false,
            preset: "minimal".to_string(),
            system_prompt: None,
        };
        let state = ServerState::new(config);
        state.note_create("Note A".to_string(), "Content A".to_string(), vec!["urgent".to_string(), "work".to_string()], "general".to_string()).await.unwrap();

        let results = state.note_search("urge").await;
        assert_eq!(results.len(), 1);
        assert!(results[0].tags.contains(&"urgent".to_string()));
    }

    #[tokio::test]
    async fn test_note_search_category() {
        let config = AppConfig {
            webui_host: "127.0.0.1".to_string(),
            webui_port: 2233,
            mcp_transport: "http".to_string(),
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 3344,
            max_concurrency: 10,
            disable_tools: vec![],
            working_dir: std::path::PathBuf::from("."),
            log_level: "info".to_string(),
            disable_webui: false,
            allow_dangerous_commands: vec![],
            allowed_hosts: None,
            disable_allowed_hosts: false,
            preset: "minimal".to_string(),
            system_prompt: None,
        };
        let state = ServerState::new(config);
        state.note_create("Note B".to_string(), "Content B".to_string(), vec![], "Reference".to_string()).await.unwrap();

        let results = state.note_search("reference").await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].category, "Reference");
    }

    #[tokio::test]
    async fn test_note_search_empty() {
        let config = AppConfig {
            webui_host: "127.0.0.1".to_string(),
            webui_port: 2233,
            mcp_transport: "http".to_string(),
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 3344,
            max_concurrency: 10,
            disable_tools: vec![],
            working_dir: std::path::PathBuf::from("."),
            log_level: "info".to_string(),
            disable_webui: false,
            allow_dangerous_commands: vec![],
            allowed_hosts: None,
            disable_allowed_hosts: false,
            preset: "minimal".to_string(),
            system_prompt: None,
        };
        let state = ServerState::new(config);
        state.note_create("Note C".to_string(), "Content C".to_string(), vec![], "misc".to_string()).await.unwrap();

        let results = state.note_search("nonexistent").await;
        assert!(results.is_empty());
    }
}
