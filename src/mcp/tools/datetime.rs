use chrono::Local;
use rmcp::model::CallToolResult;

pub async fn datetime() -> Result<CallToolResult, String> {
    let now = Local::now();

    let formatted = now.format("%Y-%m-%d %H:%M:%S %A").to_string();
    let iso_format = now.to_rfc3339();
    let timestamp = now.timestamp();

    let result = format!(
        "Current Date and Time ({}):\n\
        Formatted: {}\n\
        ISO 8601: {}\n\
        Unix Timestamp: {}",
        now.format("%:z"),
        formatted,
        iso_format,
        timestamp
    );

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(result)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_datetime() {
        let result = datetime().await;
        assert!(result.is_ok());
        
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("Current Date and Time"));
                assert!(text.text.contains("20")); // Year should be 20xx
            }
        }
    }
}
