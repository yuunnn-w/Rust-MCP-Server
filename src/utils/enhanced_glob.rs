use std::cell::RefCell;
use std::collections::HashMap;
use regex::Regex;

thread_local! {
    static REGEX_CACHE: RefCell<HashMap<String, Regex>> = RefCell::new(HashMap::new());
}

pub struct GlobMatcher {
    include_patterns: Vec<Regex>,
    exclude_patterns: Vec<Regex>,
}

impl GlobMatcher {
    pub fn new(
        patterns: &[String],
        exclude_patterns: &[String],
        use_regex: bool,
        case_sensitive: bool,
    ) -> Result<Self, String> {
        let compile = |raw: &str| -> Result<Regex, String> {
            let cache_key = format!("{}_{}_{}", raw, use_regex, case_sensitive);
            REGEX_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                if let Some(r) = cache.get(&cache_key) {
                    return Ok(r.clone());
                }
                let re_str = if use_regex {
                    raw.to_string()
                } else {
                    glob_to_regex(raw)
                };
                let pattern = if case_sensitive {
                    format!("^{}$", re_str)
                } else {
                    format!("(?i)^{}$", re_str)
                };
                let re = Regex::new(&pattern)
                    .map_err(|e| format!("Invalid pattern '{}': {}", raw, e))?;
                cache.insert(cache_key, re.clone());
                Ok(re)
            })
        };

        let include_patterns: Vec<Regex> = if patterns.is_empty() {
            Vec::new()
        } else {
            patterns
                .iter()
                .map(|p| compile(p))
                .collect::<Result<Vec<_>, _>>()?
        };

        let exclude_patterns: Vec<Regex> = if exclude_patterns.is_empty() {
            Vec::new()
        } else {
            exclude_patterns
                .iter()
                .map(|p| compile(p))
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(Self {
            include_patterns,
            exclude_patterns,
        })
    }

    pub fn matches(&self, name: &str) -> bool {
        if self.is_excluded(name) {
            return false;
        }
        if self.include_patterns.is_empty() {
            return true;
        }
        self.include_patterns.iter().any(|re| re.is_match(name))
    }

    pub fn is_excluded(&self, name: &str) -> bool {
        self.exclude_patterns.iter().any(|re| re.is_match(name))
    }
}

pub fn glob_to_regex(pattern: &str) -> String {
    let mut regex_str = String::with_capacity(pattern.len() * 2);
    let mut chars = pattern.chars().peekable();
    let mut in_char_class = false;
    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                if in_char_class {
                    regex_str.push('*');
                } else {
                    regex_str.push_str("[^/]*");
                }
            }
            '?' => {
                if in_char_class {
                    regex_str.push('?');
                } else {
                    regex_str.push('.');
                }
            }
            '[' => {
                in_char_class = true;
                regex_str.push('[');
                if chars.peek() == Some(&'!') {
                    chars.next();
                    regex_str.push('^');
                }
            }
            ']' => {
                in_char_class = false;
                regex_str.push(']');
            }
            '.' | '+' | '(' | ')' | '{' | '}' | '\\' | '^' | '$' | '|' => {
                regex_str.push('\\');
                regex_str.push(ch);
            }
            _ => regex_str.push(ch),
        }
    }
    regex_str
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_to_regex() {
        assert_eq!(glob_to_regex("*.rs"), "[^/]*\\.rs");
        assert_eq!(glob_to_regex("file?.txt"), "file.\\.txt");
        assert_eq!(glob_to_regex("test"), "test");
    }

    #[test]
    fn test_glob_matcher_include() {
        let matcher = GlobMatcher::new(
            &["*.rs".to_string(), "*.toml".to_string()],
            &[],
            false,
            true,
        )
        .unwrap();
        assert!(matcher.matches("main.rs"));
        assert!(matcher.matches("Cargo.toml"));
        assert!(!matcher.matches("README.md"));
    }

    #[test]
    fn test_glob_matcher_exclude() {
        let matcher = GlobMatcher::new(
            &[],
            &["*.tmp".to_string(), "*.log".to_string()],
            false,
            true,
        )
        .unwrap();
        assert!(matcher.matches("main.rs"));
        assert!(!matcher.matches("debug.log"));
        assert!(!matcher.matches("temp.tmp"));
    }

    #[test]
    fn test_glob_matcher_case_insensitive() {
        let matcher = GlobMatcher::new(
            &["*.RS".to_string()],
            &[],
            false,
            false,
        )
        .unwrap();
        assert!(matcher.matches("main.rs"));
        assert!(matcher.matches("MAIN.RS"));
    }

    #[test]
    fn test_glob_matcher_regex() {
        let matcher = GlobMatcher::new(
            &[r"\w+\.rs".to_string()],
            &[],
            true,
            true,
        )
        .unwrap();
        assert!(matcher.matches("main.rs"));
        assert!(!matcher.matches("main.rs.bak"));
    }

    #[test]
    fn test_glob_matcher_empty_include() {
        let matcher = GlobMatcher::new(&[], &[], false, true).unwrap();
        assert!(matcher.matches("anything.txt"));
        assert!(matcher.matches("foo.bar"));
    }

    #[test]
    fn test_glob_matcher_include_and_exclude() {
        let matcher = GlobMatcher::new(
            &["*.rs".to_string()],
            &["*_test.rs".to_string()],
            false,
            true,
        )
        .unwrap();
        assert!(matcher.matches("main.rs"));
        assert!(!matcher.matches("lib_test.rs"));
    }
}
