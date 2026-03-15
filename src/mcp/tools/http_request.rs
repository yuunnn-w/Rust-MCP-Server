use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HttpRequestParams {
    /// URL to request
    #[schemars(description = "The URL to request")]
    pub url: String,
    /// HTTP method: GET (default) or POST
    #[schemars(description = "HTTP method: GET or POST (default: GET)")]
    pub method: Option<String>,
    /// Request headers as JSON object
    #[schemars(description = "Request headers as JSON object")]
    pub headers: Option<serde_json::Value>,
    /// Request body (for POST)
    #[schemars(description = "Request body (for POST)")]
    pub body: Option<String>,
    /// Timeout in seconds (default: 30)
    #[schemars(description = "Timeout in seconds (default: 30)")]
    pub timeout: Option<u64>,
}

#[derive(Debug, Serialize)]
struct HttpResponse {
    status: u16,
    status_text: String,
    headers: serde_json::Map<String, serde_json::Value>,
    body: String,
    is_text: bool,
}

pub async fn http_request(params: Parameters<HttpRequestParams>) -> Result<CallToolResult, String> {
    let params = params.0;
    let method = params.method.unwrap_or_else(|| "GET".to_string()).to_uppercase();
    let timeout = params.timeout.unwrap_or(30);

    // Create client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Build request
    let mut request_builder = match method.as_str() {
        "GET" => client.get(&params.url),
        "POST" => client.post(&params.url),
        "PUT" => client.put(&params.url),
        "DELETE" => client.delete(&params.url),
        "PATCH" => client.patch(&params.url),
        "HEAD" => client.head(&params.url),
        _ => return Err(format!("Unsupported HTTP method: {}", method)),
    };

    // Add headers
    if let Some(headers) = params.headers {
        if let Some(obj) = headers.as_object() {
            for (key, value) in obj {
                let value_str: String = value.as_str().map(|s| s.to_string()).unwrap_or_else(|| value.to_string());
                request_builder = request_builder.header(key, value_str);
            }
        }
    }

    // Add body for POST/PUT/PATCH
    if let Some(body) = params.body {
        request_builder = request_builder.body(body);
    }

    // Send request
    let response = request_builder
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    // Build response
    let status = response.status();
    let mut headers_map = serde_json::Map::new();

    for (key, value) in response.headers() {
        if let Ok(value_str) = value.to_str() {
            headers_map.insert(
                key.to_string(),
                serde_json::Value::String(value_str.to_string()),
            );
        }
    }

    // Get content type
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let is_text = content_type.starts_with("text/")
        || content_type.contains("json")
        || content_type.contains("xml")
        || content_type.contains("javascript");

    // Get response body
    let body = if is_text {
        response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?
    } else {
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;
        format!("[Binary data: {} bytes]", bytes.len())
    };

    let http_response = HttpResponse {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("Unknown").to_string(),
        headers: headers_map,
        body,
        is_text,
    };

    let response_json =
        serde_json::to_string_pretty(&http_response).map_err(|e| e.to_string())?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        response_json,
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_request() {
        // This test requires network access
        let params = HttpRequestParams {
            url: "https://httpbin.org/get".to_string(),
            method: Some("GET".to_string()),
            headers: None,
            body: None,
            timeout: Some(10),
        };

        let result = http_request(Parameters(params)).await;
        // May fail due to network issues, so we just check it doesn't panic
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("200"));
            }
        }
    }
}
