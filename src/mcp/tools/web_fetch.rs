use crate::utils::http_utils::validate_url;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, OnceLock};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WebFetchParams {
    #[schemars(description = "The URL to fetch")]
    pub url: String,
    #[schemars(description = "Maximum characters to return (default: 50000, max: 100000)")]
    pub max_chars: Option<usize>,
    #[schemars(description = "Content extraction mode: text (default), html, or markdown")]
    pub extract_mode: Option<String>,
}

#[derive(Debug, Serialize)]
struct WebFetchOutput {
    url: String,
    title: String,
    text_content: String,
    content_length: usize,
    encoding: String,
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
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    match HTTP_CLIENT.set(client) {
        Ok(()) => Ok(HTTP_CLIENT.get().unwrap()),
        Err(_) => Ok(HTTP_CLIENT.get().unwrap()),
    }
}

fn html_entity_decode(s: &str) -> String {
    let mut result = s.to_string();
    let entities: &[(&str, &str)] = &[
        ("&amp;", "&"), ("&lt;", "<"), ("&gt;", ">"),
        ("&quot;", "\""), ("&#39;", "'"), ("&#x27;", "'"),
        ("&nbsp;", " "), ("&copy;", "\u{00A9}"), ("&reg;", "\u{00AE}"),
        ("&trade;", "\u{2122}"), ("&mdash;", "\u{2014}"), ("&ndash;", "\u{2013}"),
        ("&lsquo;", "\u{2018}"), ("&rsquo;", "\u{2019}"),
        ("&ldquo;", "\u{201C}"), ("&rdquo;", "\u{201D}"),
    ];
    for (entity, replacement) in entities {
        result = result.replace(entity, replacement);
    }
    result
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script_style = false;
    let html_lower = html.to_lowercase();

    let mut i = 0;
    let bytes = html.as_bytes();
    while i < html.len() {
        if !in_tag && i < html.len() && bytes[i] == b'<' {
            if (i + 7 <= html.len() && html_lower[i..i + 7].starts_with("<script"))
                || (i + 6 <= html.len() && html_lower[i..i + 6].starts_with("<style")) {
                in_script_style = true;
                in_tag = true;
            } else {
                in_tag = true;
            }
        } else if in_tag {
            if in_script_style {
                if i + 8 <= html.len() && html_lower[i..i + 8].starts_with("</script") {
                    in_script_style = false;
                    in_tag = false;
                    while i < html.len() && bytes[i] != b'>' {
                        i += 1;
                    }
                }
            } else if i + 7 <= html.len() && html_lower[i..i + 7].starts_with("</style") {
                    in_script_style = false;
                    in_tag = false;
                while i < html.len() && bytes[i] != b'>' {
                    i += 1;
                }
            } else if bytes[i] == b'>' {
                in_tag = false;
                result.push(' ');
            }
        } else {
            result.push(html[i..].chars().next().unwrap());
        }
        i += html[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    }

    let mut cleaned = String::new();
    let mut prev_space = false;
    for ch in html_entity_decode(&result).chars() {
        if ch.is_whitespace() {
            if !prev_space {
                cleaned.push(' ');
                prev_space = true;
            }
        } else {
            cleaned.push(ch);
            prev_space = false;
        }
    }
    cleaned.trim().to_string()
}

fn html_to_markdown(html: &str) -> String {
    let html_lower = html.to_lowercase();
    let bytes = html.as_bytes();
    let mut result = String::new();
    let mut i = 0;
    let mut in_script_style = false;
    let mut list_stack: Vec<&str> = Vec::new();
    let mut link_href: String = String::new();

    while i < html.len() {
        if bytes[i] == b'<'{
            if i + 7 <= html.len() && html_lower[i..i+7].starts_with("<script") {
                in_script_style = true;
                i += 7;
                continue;
            }
            if i + 6 <= html.len() && html_lower[i..i+6].starts_with("<style") {
                in_script_style = true;
                i += 6;
                continue;
            }
            if in_script_style {
                if (i + 8 <= html.len() && html_lower[i..i+8].starts_with("</script"))
                    || (i + 7 <= html.len() && html_lower[i..i+7].starts_with("</style")) {
                    in_script_style = false;
                }
                i += 1;
                continue;
            }

            let mut tag_end = i + 1;
            while tag_end < html.len() && bytes[tag_end] != b'>' {
                tag_end += 1;
            }
            if tag_end >= html.len() { break; }
            let tag_content = &html_lower[i+1..tag_end];
            let tag_name = tag_content.split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");

            if tag_content.ends_with('/') {
                match tag_name {
                    "br" => result.push('\n'),
                    "hr" => {
                        if !result.ends_with("\n\n") { result.push('\n'); }
                        result.push_str("---\n\n");
                    }
                    "img" => {
                        let alt = extract_attr(&html[i+1..tag_end], "alt");
                        let src = extract_attr(&html[i+1..tag_end], "src");
                        result.push_str(&format!("![{}]({})\n", alt, src));
                    }
                    "input" => { result.push_str("[input]"); }
                    "meta" | "link" => {}
                    _ => {}
                }
                i = tag_end + 1;
                continue;
            }

            let is_closing = tag_content.starts_with('/');
            let clean_tag = tag_name;

            match clean_tag {
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    let level = clean_tag.as_bytes()[1] - b'0';
                    if is_closing { result.push('\n'); }
                    else {
                        if !result.ends_with('\n') { result.push('\n'); }
                        result.push_str(&"#".repeat(level as usize));
                        result.push(' ');
                    }
                }
                "p" => {
                    if is_closing { result.push_str("\n\n"); }
                    else if !result.ends_with('\n') { result.push('\n'); }
                }
                "b" | "strong" => result.push_str("**"),
                "i" | "em" => result.push('*'),
                "a" => {
                    if is_closing {
                        result.push_str("](");
                        result.push_str(&link_href);
                        result.push(')');
                        link_href.clear();
                    } else {
                        link_href = extract_attr(&html[i+1..tag_end], "href");
                        result.push('[');
                    }
                }
                "ul" => {
                    if is_closing {
                        list_stack.pop();
                        result.push('\n');
                    } else {
                        list_stack.push("*");
                        if !result.ends_with('\n') { result.push('\n'); }
                    }
                }
                "ol" => {
                    if is_closing {
                        list_stack.pop();
                        result.push('\n');
                    } else {
                        list_stack.push("1.");
                        if !result.ends_with('\n') { result.push('\n'); }
                    }
                }
                "li" => {
                    if is_closing { result.push('\n'); }
                    else {
                        let marker = list_stack.last().copied().unwrap_or("-");
                        result.push_str(marker);
                        result.push(' ');
                    }
                }
                "pre" | "code" => {
                    result.push_str("\n```\n");
                }
                "blockquote" => {
                    if is_closing { result.push('\n'); }
                    else { result.push_str("\n> "); }
                }
                _ => {}
            }
            i = tag_end + 1;
        } else {
            if !in_script_style {
                let ch = html[i..].chars().next().unwrap();
                result.push(ch);
            }
            i += html[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        }
    }

    let decoded = html_entity_decode(&result);
    let mut cleaned = String::new();
    let mut prev_blank = false;
    for line in decoded.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_blank {
                cleaned.push('\n');
                prev_blank = true;
            }
        } else {
            cleaned.push_str(trimmed);
            cleaned.push('\n');
            prev_blank = false;
        }
    }
    cleaned.trim().to_string()
}

fn extract_attr(tag: &str, attr_name: &str) -> String {
    let tag_lower = tag.to_lowercase();
    let search = format!("{}=", attr_name);
    if let Some(pos) = tag_lower.find(&search) {
        let after = &tag[pos + search.len()..];
        let delimiter = after.chars().next().unwrap_or('"');
        let _content_start = if delimiter == '"' || delimiter == '\'' { 1 } else { 0 };
        let content = if delimiter == '"' || delimiter == '\'' {
            after[1..].split(delimiter).next().unwrap_or("")
        } else {
            after.split(|c: char| c.is_whitespace()).next().unwrap_or("")
        };
        return html_entity_decode(content).trim().to_string();
    }
    String::new()
}

static TITLE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"<title[^>]*>(.*?)</title>").unwrap()
});

fn extract_title(html: &str) -> String {
    if let Some(cap) = TITLE_RE.captures(html) {
        let title = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        return html_entity_decode(title).trim().to_string();
    }
    String::new()
}

fn detect_encoding(headers: &reqwest::header::HeaderMap, body: &[u8]) -> String {
    if let Some(content_type) = headers.get("content-type") {
        if let Ok(ct_str) = content_type.to_str() {
            let ct_lower = ct_str.to_lowercase();
            if let Some(pos) = ct_lower.find("charset") {
                let after = &ct_str[pos + 7..];
                let after = after.trim_start_matches([' ', '=']);
                let charset = after.split(';').next().unwrap_or("").trim();
                if !charset.is_empty() {
                    return charset.to_string();
                }
            }
        }
    }
    let mut encoding = "UTF-8".to_string();
    let body_slice: &[u8] = if body.len() > 1024 { &body[..1024] } else { body };
    if let Ok(text) = std::str::from_utf8(body_slice) {
        let text_lower = text.to_lowercase();
        if let Some(meta_pos) = text_lower.find("<meta") {
            let meta_end = text_lower[meta_pos..].find('>').unwrap_or(text.len() - meta_pos);
            let meta_text = &text_lower[meta_pos..meta_pos + meta_end];
            if let Some(charset_pos) = meta_text.find("charset") {
                let after = &meta_text[charset_pos + 7..];
                let after = after.trim_start_matches([' ', '=']);
                let charset = after.split(['"', '\'', ';', ' ', '>'])
                    .next()
                    .unwrap_or("")
                    .trim();
                if !charset.is_empty() {
                    encoding = charset.to_uppercase();
                }
            }
        }
    }
    encoding
}

pub async fn web_fetch(params: Parameters<WebFetchParams>) -> Result<CallToolResult, String> {
    let p = params.0;
    let url = p.url.trim().to_string();
    if url.is_empty() {
        return Err("URL must not be empty".to_string());
    }

    validate_url(&url)?;

    let max_chars = p.max_chars.unwrap_or(50000).min(100000);

    let client = get_http_client()?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "HTTP request failed with status {} {}",
            response.status().as_u16(),
            response.status().canonical_reason().unwrap_or("Unknown")
        ));
    }

    let response_headers = response.headers().clone();

    let content_length_hint = response_headers
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let max_response_bytes: u64 = 50 * 1024 * 1024;

    if content_length_hint > max_response_bytes {
        return Err(format!(
            "Response body too large: Content-Length {} exceeds {} MB limit",
            content_length_hint,
            max_response_bytes / (1024 * 1024)
        ));
    }

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if body_bytes.len() as u64 > max_response_bytes {
        return Err(format!(
            "Response body too large: {} bytes exceeds {} MB limit",
            body_bytes.len(),
            max_response_bytes / (1024 * 1024)
        ));
    }

    let encoding = detect_encoding(&response_headers, &body_bytes);

    let body_text = String::from_utf8_lossy(&body_bytes).to_string();

    let title = extract_title(&body_text);

    let extract_mode = p.extract_mode.as_deref().unwrap_or("text");
    let text_content = match extract_mode {
        "html" => body_text.clone(),
        "markdown" => html_to_markdown(&body_text),
        _ => strip_html_tags(&body_text),
    };

    let content_length = text_content.chars().count();
    let text_content = if content_length > max_chars {
        let mut truncated: String = text_content.chars().take(max_chars).collect();
        truncated.push_str(&format!(
            "\n\n[... Content truncated: {} / {} characters ...]",
            max_chars, content_length
        ));
        truncated
    } else {
        text_content
    };

    let output = WebFetchOutput {
        url,
        title,
        text_content,
        content_length,
        encoding,
    };

    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_tags() {
        let html = "<html><head><title>Test</title></head><body><p>Hello</p><script>var x=1;</script><p>World</p></body></html>";
        let text = strip_html_tags(html);
        assert!(!text.contains("<"));
        assert!(!text.contains("var x=1"));
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn test_extract_title() {
        let html = "<html><head><title>Test Title</title></head><body></body></html>";
        let title = extract_title(html);
        assert_eq!(title, "Test Title");
    }

    #[test]
    fn test_extract_title_entities() {
        let html = "<html><head><title>Foo &amp; Bar</title></head><body></body></html>";
        let title = extract_title(html);
        assert_eq!(title, "Foo & Bar");
    }
}
