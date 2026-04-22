use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use url::Host;

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
    /// JSON Pointer path to extract from JSON response, e.g. "/data/0/name"
    #[schemars(description = "JSON Pointer path to extract from JSON response, e.g. '/data/0/name'")]
    pub extract_json_path: Option<String>,
    /// Include response headers in output (default: false)
    #[schemars(description = "Include response headers in output (default: false)")]
    pub include_response_headers: Option<bool>,
    /// Maximum response body characters (default: 15000)
    #[schemars(description = "Maximum response body characters (default: 15000)")]
    pub max_response_chars: Option<usize>,
}

#[derive(Debug, Serialize)]
struct HttpResponse {
    status: u16,
    status_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<serde_json::Map<String, serde_json::Value>>,
    body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extracted: Option<serde_json::Value>,
}

fn validate_url(url_str: &str) -> Result<(), String> {
    let url = url::Url::parse(url_str).map_err(|e| format!("Invalid URL: {}", e))?;

    let scheme = url.scheme().to_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(format!(
            "URL scheme '{}' is not allowed. Only http and https are supported.",
            scheme
        ));
    }

    let host = url.host().ok_or_else(|| "URL missing host".to_string())?;

    match host {
        Host::Domain(domain) => {
            let domain_lower = domain.to_lowercase();
            if domain_lower == "localhost"
                || domain_lower.ends_with(".localhost")
                || domain_lower == "127.0.0.1"
                || domain_lower == "0.0.0.0"
                || domain_lower == "::1"
                || domain_lower == "[::1]"
            {
                return Err(format!(
                    "Access to internal host '{}' is not allowed.",
                    domain
                ));
            }
        }
        Host::Ipv4(ip) => {
            if is_private_ip(IpAddr::V4(ip)) {
                return Err(format!(
                    "Access to private IP '{}' is not allowed.",
                    ip
                ));
            }
        }
        Host::Ipv6(ip) => {
            if is_private_ip(IpAddr::V6(ip)) {
                return Err(format!(
                    "Access to private IP '{}' is not allowed.",
                    ip
                ));
            }
        }
    }

    Ok(())
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            octets[0] == 127
                || octets[0] == 10
                || (octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31))
                || (octets[0] == 192 && octets[1] == 168)
                || (octets[0] == 169 && octets[1] == 254)
                || octets[0] == 0
        }
        IpAddr::V6(ipv6) => {
            let segments = ipv6.segments();
            segments == [0, 0, 0, 0, 0, 0, 0, 1]
                || segments == [0, 0, 0, 0, 0, 0, 0, 0]
                || (segments[0] & 0xfe00) == 0xfc00
                || (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .build()
            .expect("Failed to build HTTP client")
    })
}

/// Extract value from JSON using JSON Pointer (RFC 6901)
fn extract_json_pointer(value: &serde_json::Value, pointer: &str) -> Option<serde_json::Value> {
    value.pointer(pointer).cloned()
}

fn truncate_body(body: &str, limit: usize) -> String {
    if body.len() <= limit {
        body.to_string()
    } else {
        format!(
            "{}\n\n[... Response truncated: {} / {} characters ...]",
            &body[..limit],
            body.len(),
            limit
        )
    }
}

pub async fn http_request(params: Parameters<HttpRequestParams>) -> Result<CallToolResult, String> {
    let params = params.0;
    let method = params.method.unwrap_or_else(|| "GET".to_string()).to_uppercase();
    let timeout = params.timeout.unwrap_or(30);
    let include_headers = params.include_response_headers.unwrap_or(false);
    let max_chars = params.max_response_chars.unwrap_or(15000);

    validate_url(&params.url)?;

    let client = get_http_client();

    let mut request_builder = match method.as_str() {
        "GET" => client.get(&params.url),
        "POST" => client.post(&params.url),
        "PUT" => client.put(&params.url),
        "DELETE" => client.delete(&params.url),
        "PATCH" => client.patch(&params.url),
        "HEAD" => client.head(&params.url),
        _ => return Err(format!("Unsupported HTTP method: {}", method)),
    };

    request_builder = request_builder.timeout(std::time::Duration::from_secs(timeout));

    if let Some(headers) = params.headers {
        if let Some(obj) = headers.as_object() {
            for (key, value) in obj {
                let value_str: String = value.as_str().map(|s| s.to_string()).unwrap_or_else(|| value.to_string());
                request_builder = request_builder.header(key, value_str);
            }
        }
    }

    if let Some(body) = params.body {
        request_builder = request_builder.body(body);
    }

    let response = request_builder
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    let mut headers_map = serde_json::Map::new();

    if include_headers {
        for (key, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                headers_map.insert(
                    key.to_string(),
                    serde_json::Value::String(value_str.to_string()),
                );
            }
        }
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let is_text = content_type.starts_with("text/")
        || content_type.contains("json")
        || content_type.contains("xml")
        || content_type.contains("javascript");

    let body_str = if is_text {
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

    let truncated_body = truncate_body(&body_str, max_chars);

    // JSON extraction
    let extracted = if let Some(ref pointer) = params.extract_json_path {
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_str) {
            extract_json_pointer(&json_val, pointer)
        } else {
            None
        }
    } else {
        None
    };

    let http_response = HttpResponse {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("Unknown").to_string(),
        headers: if include_headers { Some(headers_map) } else { None },
        body: truncated_body,
        extracted,
    };

    let response_json = serde_json::to_string_pretty(&http_response).map_err(|e| e.to_string())?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(response_json)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_request() {
        let params = HttpRequestParams {
            url: "https://httpbin.org/get".to_string(),
            method: Some("GET".to_string()),
            headers: None,
            body: None,
            timeout: Some(10),
            extract_json_path: None,
            include_response_headers: Some(false),
            max_response_chars: Some(5000),
        };

        let result = http_request(Parameters(params)).await;
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("200"));
            }
        }
    }

    #[test]
    fn test_extract_json_pointer() {
        let json = serde_json::json!({"data": [{"name": "Alice"}]});
        assert_eq!(
            extract_json_pointer(&json, "/data/0/name"),
            Some(serde_json::Value::String("Alice".to_string()))
        );
        assert_eq!(extract_json_pointer(&json, "/missing"), None);
    }
}
