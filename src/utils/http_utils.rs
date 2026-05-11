// Shared HTTP utilities — single source of truth for SSRF validation.
// validate_url() and is_private_ip() are the canonical implementations
// used by both web_fetch and http_request MCP tools.

use std::net::IpAddr;
use url::Host;

pub fn validate_url(url_str: &str) -> Result<(), String> {
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

pub fn is_private_ip(ip: IpAddr) -> bool {
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
            if segments[0..5] == [0, 0, 0, 0, 0] && segments[5] == 0xffff {
                let mapped_ipv4 = u32::from_be_bytes([
                    (segments[6] >> 8) as u8,
                    (segments[6] & 0xff) as u8,
                    (segments[7] >> 8) as u8,
                    (segments[7] & 0xff) as u8,
                ]);
                return is_private_ip(IpAddr::V4(std::net::Ipv4Addr::from(mapped_ipv4)));
            }
            segments == [0, 0, 0, 0, 0, 0, 0, 1]
                || segments == [0, 0, 0, 0, 0, 0, 0, 0]
                || (segments[0] & 0xfe00) == 0xfc00
                || (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_allows_public() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://httpbin.org/get").is_ok());
    }

    #[test]
    fn test_validate_url_rejects_localhost() {
        assert!(validate_url("http://localhost:8080").is_err());
        assert!(validate_url("http://127.0.0.1").is_err());
    }

    #[test]
    fn test_validate_url_rejects_private_ip() {
        assert!(validate_url("http://10.0.0.1").is_err());
        assert!(validate_url("http://192.168.1.1").is_err());
        assert!(validate_url("http://172.16.0.1").is_err());
    }

    #[test]
    fn test_is_private_ip() {
        assert!(is_private_ip(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 1))));
        assert!(is_private_ip(IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 0, 1))));
        assert!(!is_private_ip(IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8))));
    }
}
