use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskParams {
    #[schemars(description = "Operation: create, list, get, update, delete")]
    pub operation: String,
    #[schemars(description = "Task title (required for create, max 200 chars)")]
    pub title: Option<String>,
    #[schemars(description = "Task description (optional, max 5000 chars)")]
    pub description: Option<String>,
    #[schemars(description = "Task priority: low, medium, high (default: medium)")]
    pub priority: Option<String>,
    #[schemars(description = "Tags for the task (optional, max 5 tags, each max 50 chars)")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Task ID (required for get, update, delete)")]
    pub id: Option<u64>,
    #[schemars(description = "Filter by status: pending, in_progress, completed")]
    pub status: Option<String>,
    #[schemars(description = "Filter by priority: low, medium, high")]
    pub priority_filter: Option<String>,
    #[schemars(description = "Filter by tag")]
    pub tag_filter: Option<String>,
    #[schemars(description = "Maximum number of tasks to return")]
    pub limit: Option<usize>,
    #[schemars(description = "New status: pending, in_progress, completed")]
    pub new_status: Option<String>,
    #[schemars(description = "New title (max 200 chars)")]
    pub new_title: Option<String>,
    #[schemars(description = "New description (max 5000 chars)")]
    pub new_description: Option<String>,
    #[schemars(description = "New priority: low, medium, high")]
    pub new_priority: Option<String>,
    #[schemars(description = "Tags to add to the task")]
    pub add_tags: Option<Vec<String>>,
    #[schemars(description = "Tags to remove from the task")]
    pub remove_tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskListOutput {
    tasks: Vec<crate::mcp::state::Task>,
    total_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskDeleteOutput {
    deleted: bool,
    id: u64,
}

pub async fn task(
    params: Parameters<TaskParams>,
    state: Arc<crate::mcp::state::ServerState>,
) -> Result<CallToolResult, String> {
    let p = params.0;
    let operation = p.operation.to_lowercase();

    match operation.as_str() {
        "create" => {
            let title = p.title.ok_or("title is required for create operation")?;
            let description = p.description.unwrap_or_default();
            let priority = p.priority.unwrap_or_else(|| "medium".to_string());
            let tags = p.tags.unwrap_or_default();
            let task = state.task_create(title, description, priority, tags).await?;
            let json = serde_json::to_string_pretty(&task).map_err(|e| e.to_string())?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
        }
        "list" => {
            let status_filter = p.status;
            let priority_filter = p.priority_filter;
            let tag_filter = p.tag_filter;
            let mut tasks = state.task_list(status_filter, priority_filter, tag_filter, None).await;
            if let Some(lim) = p.limit {
                tasks.truncate(lim);
            }
            let total = tasks.len();
            let output = TaskListOutput { tasks, total_count: total };
            let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
        }
        "get" => {
            let id = p.id.ok_or("id is required for get operation")?;
            let task = state.task_read(id).await
                .ok_or_else(|| format!("Task {} not found", id))?;
            let json = serde_json::to_string_pretty(&task).map_err(|e| e.to_string())?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
        }
        "update" => {
            let id = p.id.ok_or("id is required for update operation")?;
            let status = p.new_status;
            let title = p.new_title;
            let description = p.new_description;
            let new_priority = p.new_priority;
            let add_tags = p.add_tags;
            let remove_tags = p.remove_tags;
            let task = state.task_update(id, status, title, description, new_priority, add_tags, remove_tags).await?;
            let json = serde_json::to_string_pretty(&task).map_err(|e| e.to_string())?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
        }
        "delete" => {
            let id = p.id.ok_or("id is required for delete operation")?;
            state.task_delete(id).await?;
            let output = TaskDeleteOutput { deleted: true, id };
            let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
        }
        _ => Err(format!("Unknown operation: '{}'. Must be one of: create, list, get, update, delete", p.operation)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::mcp::state::ServerState;
    use rmcp::handler::server::wrapper::Parameters;

    fn create_test_config() -> AppConfig {
        AppConfig {
            webui_host: "127.0.0.1".to_string(),
            webui_port: 2233,
            mcp_transport: "http".to_string(),
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 8080,
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
        }
    }

    #[tokio::test]
    async fn test_task_create() {
        let state = ServerState::new(create_test_config());

        let params = TaskParams {
            operation: "create".to_string(),
            title: Some("Test Task".to_string()),
            description: Some("A test description".to_string()),
            priority: Some("high".to_string()),
            tags: Some(vec!["rust".to_string(), "testing".to_string()]),
            id: None,
            status: None,
            priority_filter: None,
            tag_filter: None,
            limit: None,
            new_status: None,
            new_title: None,
            new_description: None,
            new_priority: None,
            add_tags: None,
            remove_tags: None,
        };

        let result = task(Parameters(params), state.clone()).await;
        assert!(result.is_ok());

        let call_result = result.unwrap();
        let json = call_result.content.first().and_then(|c| c.as_text()).unwrap();
        let created: crate::mcp::state::Task = serde_json::from_str(&json.text).unwrap();
        assert!(created.id > 0);
        assert_eq!(created.title, "Test Task");
        assert_eq!(created.description, "A test description");
        assert_eq!(created.priority, "high");
        assert_eq!(created.status, "pending");
        assert!(created.tags.contains(&"rust".to_string()));
        assert!(created.tags.contains(&"testing".to_string()));
    }

    #[tokio::test]
    async fn test_task_get() {
        let state = ServerState::new(create_test_config());
        let created = state.task_create(
            "Get Test".to_string(),
            "Description for get".to_string(),
            "low".to_string(),
            vec!["backend".to_string()],
        ).await.unwrap();

        let params = TaskParams {
            operation: "get".to_string(),
            title: None,
            description: None,
            priority: None,
            tags: None,
            id: Some(created.id),
            status: None,
            priority_filter: None,
            tag_filter: None,
            limit: None,
            new_status: None,
            new_title: None,
            new_description: None,
            new_priority: None,
            add_tags: None,
            remove_tags: None,
        };

        let result = task(Parameters(params), state.clone()).await;
        assert!(result.is_ok());

        let call_result = result.unwrap();
        let json = call_result.content.first().and_then(|c| c.as_text()).unwrap();
        let fetched: crate::mcp::state::Task = serde_json::from_str(&json.text).unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.title, "Get Test");
        assert_eq!(fetched.description, "Description for get");
        assert_eq!(fetched.priority, "low");
        assert_eq!(fetched.status, "pending");
        assert!(fetched.tags.contains(&"backend".to_string()));
    }

    #[tokio::test]
    async fn test_task_list() {
        let state = ServerState::new(create_test_config());

        let t1 = state.task_create("Task 1".to_string(), "Desc 1".to_string(), "medium".to_string(), vec![]).await.unwrap();
        let t2 = state.task_create("Task 2".to_string(), "Desc 2".to_string(), "high".to_string(), vec![]).await.unwrap();
        let t3 = state.task_create("Task 3".to_string(), "Desc 3".to_string(), "low".to_string(), vec![]).await.unwrap();
        state.task_update(t2.id, Some("in_progress".to_string()), None, None, None, None, None).await.unwrap();
        state.task_update(t3.id, Some("completed".to_string()), None, None, None, None, None).await.unwrap();

        let params = TaskParams {
            operation: "list".to_string(),
            title: None, description: None, priority: None, tags: None, id: None,
            status: None, priority_filter: None, tag_filter: None, limit: None,
            new_status: None, new_title: None, new_description: None, new_priority: None,
            add_tags: None, remove_tags: None,
        };
        let result = task(Parameters(params), state.clone()).await;
        assert!(result.is_ok());
        let json = result.unwrap().content.first().and_then(|c| c.as_text()).unwrap().text.clone();
        let output: super::TaskListOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(output.total_count, 3);
        assert_eq!(output.tasks.len(), 3);

        let params = TaskParams {
            operation: "list".to_string(),
            title: None, description: None, priority: None, tags: None, id: None,
            status: Some("pending".to_string()), priority_filter: None, tag_filter: None, limit: None,
            new_status: None, new_title: None, new_description: None, new_priority: None,
            add_tags: None, remove_tags: None,
        };
        let result = task(Parameters(params), state.clone()).await;
        assert!(result.is_ok());
        let json = result.unwrap().content.first().and_then(|c| c.as_text()).unwrap().text.clone();
        let output: super::TaskListOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(output.total_count, 1);
        assert_eq!(output.tasks[0].status, "pending");
        assert_eq!(output.tasks[0].id, t1.id);

        let params = TaskParams {
            operation: "list".to_string(),
            title: None, description: None, priority: None, tags: None, id: None,
            status: Some("completed".to_string()), priority_filter: None, tag_filter: None, limit: None,
            new_status: None, new_title: None, new_description: None, new_priority: None,
            add_tags: None, remove_tags: None,
        };
        let result = task(Parameters(params), state.clone()).await;
        assert!(result.is_ok());
        let json = result.unwrap().content.first().and_then(|c| c.as_text()).unwrap().text.clone();
        let output: super::TaskListOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(output.total_count, 1);
        assert_eq!(output.tasks[0].status, "completed");
    }

    #[tokio::test]
    async fn test_task_list_filters() {
        let state = ServerState::new(create_test_config());

        state.task_create("High Task".to_string(), "Desc".to_string(), "high".to_string(), vec!["urgent".to_string()]).await.unwrap();
        state.task_create("Low Task".to_string(), "Desc".to_string(), "low".to_string(), vec!["backlog".to_string()]).await.unwrap();
        state.task_create("Rust Task".to_string(), "Desc".to_string(), "medium".to_string(), vec!["rust".to_string()]).await.unwrap();

        let params = TaskParams {
            operation: "list".to_string(),
            title: None, description: None, priority: None, tags: None, id: None,
            status: None, priority_filter: Some("high".to_string()), tag_filter: None, limit: None,
            new_status: None, new_title: None, new_description: None, new_priority: None,
            add_tags: None, remove_tags: None,
        };
        let result = task(Parameters(params), state.clone()).await;
        assert!(result.is_ok());
        let json = result.unwrap().content.first().and_then(|c| c.as_text()).unwrap().text.clone();
        let output: super::TaskListOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(output.total_count, 1);
        assert_eq!(output.tasks[0].priority, "high");
        assert_eq!(output.tasks[0].title, "High Task");

        let params = TaskParams {
            operation: "list".to_string(),
            title: None, description: None, priority: None, tags: None, id: None,
            status: None, priority_filter: None, tag_filter: Some("rust".to_string()), limit: None,
            new_status: None, new_title: None, new_description: None, new_priority: None,
            add_tags: None, remove_tags: None,
        };
        let result = task(Parameters(params), state.clone()).await;
        assert!(result.is_ok());
        let json = result.unwrap().content.first().and_then(|c| c.as_text()).unwrap().text.clone();
        let output: super::TaskListOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(output.total_count, 1);
        assert_eq!(output.tasks[0].title, "Rust Task");

        let params = TaskParams {
            operation: "list".to_string(),
            title: None, description: None, priority: None, tags: None, id: None,
            status: None, priority_filter: None, tag_filter: Some("nonexistent".to_string()), limit: None,
            new_status: None, new_title: None, new_description: None, new_priority: None,
            add_tags: None, remove_tags: None,
        };
        let result = task(Parameters(params), state.clone()).await;
        assert!(result.is_ok());
        let json = result.unwrap().content.first().and_then(|c| c.as_text()).unwrap().text.clone();
        let output: super::TaskListOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(output.total_count, 0);
    }

    #[tokio::test]
    async fn test_task_update() {
        let state = ServerState::new(create_test_config());
        let created = state.task_create(
            "Original".to_string(),
            "Original desc".to_string(),
            "low".to_string(),
            vec!["tag1".to_string()],
        ).await.unwrap();

        let params = TaskParams {
            operation: "update".to_string(),
            title: None, description: None, priority: None, tags: None,
            id: Some(created.id), status: None, priority_filter: None, tag_filter: None, limit: None,
            new_status: Some("in_progress".to_string()),
            new_title: Some("Updated Title".to_string()),
            new_description: None,
            new_priority: Some("high".to_string()),
            add_tags: Some(vec!["tag2".to_string()]),
            remove_tags: None,
        };

        let result = task(Parameters(params), state.clone()).await;
        assert!(result.is_ok());
        let json = result.unwrap().content.first().and_then(|c| c.as_text()).unwrap().text.clone();
        let updated: crate::mcp::state::Task = serde_json::from_str(&json).unwrap();
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.status, "in_progress");
        assert_eq!(updated.priority, "high");
        assert!(updated.tags.contains(&"tag1".to_string()));
        assert!(updated.tags.contains(&"tag2".to_string()));

        let fetched = state.tasks.get(&created.id).unwrap().clone();
        assert_eq!(fetched.title, "Updated Title");
        assert_eq!(fetched.status, "in_progress");
    }

    #[tokio::test]
    async fn test_task_delete() {
        let state = ServerState::new(create_test_config());
        let created = state.task_create(
            "Delete Me".to_string(),
            "To be deleted".to_string(),
            "medium".to_string(),
            vec![],
        ).await.unwrap();

        let params = TaskParams {
            operation: "delete".to_string(),
            title: None, description: None, priority: None, tags: None,
            id: Some(created.id), status: None, priority_filter: None, tag_filter: None, limit: None,
            new_status: None, new_title: None, new_description: None, new_priority: None,
            add_tags: None, remove_tags: None,
        };
        let result = task(Parameters(params), state.clone()).await;
        assert!(result.is_ok());
        let json = result.unwrap().content.first().and_then(|c| c.as_text()).unwrap().text.clone();
        let del_output: super::TaskDeleteOutput = serde_json::from_str(&json).unwrap();
        assert!(del_output.deleted);
        assert_eq!(del_output.id, created.id);

        let params = TaskParams {
            operation: "get".to_string(),
            title: None, description: None, priority: None, tags: None,
            id: Some(created.id), status: None, priority_filter: None, tag_filter: None, limit: None,
            new_status: None, new_title: None, new_description: None, new_priority: None,
            add_tags: None, remove_tags: None,
        };
        let result = task(Parameters(params), state.clone()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_task_invalid_operation() {
        let state = ServerState::new(create_test_config());

        let params = TaskParams {
            operation: "invalid_op".to_string(),
            title: None, description: None, priority: None, tags: None, id: None,
            status: None, priority_filter: None, tag_filter: None, limit: None,
            new_status: None, new_title: None, new_description: None, new_priority: None,
            add_tags: None, remove_tags: None,
        };
        let result = task(Parameters(params), state).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Unknown operation"));
        assert!(err.contains("invalid_op"));
    }

    #[tokio::test]
    async fn test_task_get_nonexistent() {
        let state = ServerState::new(create_test_config());

        let params = TaskParams {
            operation: "get".to_string(),
            title: None, description: None, priority: None, tags: None,
            id: Some(99999), status: None, priority_filter: None, tag_filter: None, limit: None,
            new_status: None, new_title: None, new_description: None, new_priority: None,
            add_tags: None, remove_tags: None,
        };
        let result = task(Parameters(params), state).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
