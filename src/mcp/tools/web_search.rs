use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, OnceLock};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WebSearchParams {
    #[schemars(description = "Search query (required, max 500 chars)")]
    pub query: String,
    #[schemars(description = "Number of results to return (1-20, default: 10)")]
    pub num_results: Option<usize>,
    #[schemars(description = "Reserved for future use (currently only DuckDuckGo is supported)")]
    #[allow(dead_code)]
    pub search_engine: Option<String>,
    #[schemars(description = "Region code for search results (e.g. cn, us)")]
    pub region: Option<String>,
    #[schemars(description = "Language code for search results (e.g. zh-CN, en-US)")]
    pub language: Option<String>,
}

#[derive(Debug, Serialize)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

#[derive(Debug, Serialize)]
struct WebSearchOutput {
    results: Vec<SearchResult>,
    query: String,
    total_results: usize,
}

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_http_client() -> Result<&'static reqwest::Client, String> {
    if let Some(client) = HTTP_CLIENT.get() {
        return Ok(client);
    }
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; Rust-MCP-Server/0.4.0)")
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    match HTTP_CLIENT.set(client) {
        Ok(()) => Ok(HTTP_CLIENT.get().unwrap()),
        Err(_) => Ok(HTTP_CLIENT.get().unwrap()),
    }
}

fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push(hex_char(byte >> 4));
                result.push(hex_char(byte & 0x0F));
            }
        }
    }
    result
}

fn hex_char(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'A' + (n - 10)) as char,
    }
}

fn html_entity_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
}

static TITLE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"<a[^>]*class="result__a"[^>]*href="([^"]*)"[^>]*>([^<]*)</a>"#
    ).unwrap()
});
static SNIPPET_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"<a[^>]*class="result__snippet"[^>]*>([^<]*)</a>"#
    ).unwrap()
});

fn parse_duckduckgo_html(html: &str, max_results: usize) -> Vec<SearchResult> {
    let title_matches: Vec<_> = TITLE_RE.captures_iter(html).collect();
    let snippet_matches: Vec<_> = SNIPPET_RE.captures_iter(html).collect();

    let count = title_matches.len().min(max_results);
    let mut results = Vec::with_capacity(count);

    for i in 0..count {
        let url = title_matches[i]
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let title = title_matches[i]
            .get(2)
            .map(|m| html_entity_decode(m.as_str()))
            .unwrap_or_default();
        let snippet = if i < snippet_matches.len() {
            snippet_matches[i]
                .get(1)
                .map(|m| html_entity_decode(m.as_str()))
                .unwrap_or_default()
        } else {
            String::new()
        };
        results.push(SearchResult { title, url, snippet });
    }

    results
}

pub async fn web_search(params: Parameters<WebSearchParams>) -> Result<CallToolResult, String> {
    let p = params.0;
    let query = p.query.trim().to_string();
    if query.is_empty() {
        return Err("Query must not be empty".to_string());
    }
    if query.len() > 500 {
        return Err("Query must be 500 characters or fewer".to_string());
    }

    let num_results = p.num_results.unwrap_or(10).clamp(1, 20);

    let encoded_query = url_encode(&query);
    let mut search_url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);
    if let (Some(region), Some(language)) = (p.region.as_ref(), p.language.as_ref()) {
        search_url.push_str(&format!("&kl={}-{}", region.to_lowercase(), language.to_lowercase()));
    } else if let Some(ref region) = p.region {
        search_url.push_str(&format!("&kl={}", region.to_lowercase()));
    }

    let client = get_http_client()?;
    let response = client
        .get(&search_url)
        .send()
        .await
        .map_err(|e| format!("Search request failed: {}", e))?;

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read search response: {}", e))?;

    let results = parse_duckduckgo_html(&body, num_results);

    let output = WebSearchOutput {
        total_results: results.len(),
        query,
        results,
    };

    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("hello world"), "hello+world");
        assert_eq!(url_encode("rust & go"), "rust+%26+go");
        assert!(url_encode("test").contains("test"));
    }

    #[test]
    fn test_html_entity_decode() {
        assert_eq!(html_entity_decode("foo &amp; bar"), "foo & bar");
        assert_eq!(html_entity_decode("&lt;div&gt;"), "<div>");
    }

    #[test]
    fn test_parse_empty_html() {
        let results = parse_duckduckgo_html("", 10);
        assert!(results.is_empty());
    }
}
