use chrono::{Local, FixedOffset};
use rmcp::model::CallToolResult;

pub async fn datetime() -> Result<CallToolResult, String> {
    // China timezone is UTC+8
    let china_tz = FixedOffset::east_opt(8 * 3600).unwrap();
    let now_china = Local::now().with_timezone(&china_tz);

    let formatted = now_china.format("%Y-%m-%d %H:%M:%S %A").to_string();
    let iso_format = now_china.to_rfc3339();
    let timestamp = now_china.timestamp();

    let result = format!(
        "Current Date and Time (China/Beijing, UTC+8):\n\
        Formatted: {}\n\
        ISO 8601: {}\n\
        Unix Timestamp: {}",
        formatted, iso_format, timestamp
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
                assert!(text.text.contains("China") || text.text.contains("UTC+8"));
                assert!(text.text.contains("20")); // Year should be 20xx
            }
        }
    }
}
