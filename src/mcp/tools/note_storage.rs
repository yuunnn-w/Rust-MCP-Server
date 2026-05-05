use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NoteStorageParams {
    /// Operation: create, list, read, update, delete, search, append
    pub operation: String,
    /// Note ID (required for read, update, delete, append)
    pub id: Option<u64>,
    /// Note title (for create, update)
    pub title: Option<String>,
    /// Note content (for create, update)
    pub content: Option<String>,
    /// Tags list (for create, update)
    pub tags: Option<Vec<String>>,
    /// Category (for create, update, list filter)
    pub category: Option<String>,
    /// Tag filter (for list)
    pub tag_filter: Option<String>,
    /// Search query (for search)
    pub query: Option<String>,
    /// Content to append (for append)
    pub append_content: Option<String>,
}

pub async fn note_storage(params: Parameters<NoteStorageParams>, state: Arc<crate::mcp::state::ServerState>) -> Result<CallToolResult, String> {
    let p = params.0;
    let op = p.operation.to_lowercase();
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

use rmcp::handler::server::wrapper::Parameters;

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
}
