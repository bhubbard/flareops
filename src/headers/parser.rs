use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderRule {
    pub path_pattern: String,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeadersFile {
    pub rules: Vec<HeaderRule>,
}

impl HeadersFile {
    pub fn parse(content: &str) -> Self {
        let mut rules = Vec::new();
        let mut current_pattern: Option<String> = None;
        let mut current_headers: BTreeMap<String, String> = BTreeMap::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let is_indented = line.starts_with(' ') || line.starts_with('\t');

            if !is_indented && (trimmed.starts_with('/') || trimmed.starts_with('*')) {
                if let Some(pattern) = current_pattern.take() {
                    rules.push(HeaderRule {
                        path_pattern: pattern,
                        headers: current_headers,
                    });
                    current_headers = BTreeMap::new();
                }
                current_pattern = Some(trimmed.to_string());
            } else if let Some((key, val)) = trimmed.split_once(':')
                && current_pattern.is_some()
            {
                current_headers.insert(key.trim().to_string(), val.trim().to_string());
            }
        }

        if let Some(pattern) = current_pattern {
            rules.push(HeaderRule {
                path_pattern: pattern,
                headers: current_headers,
            });
        }

        HeadersFile { rules }
    }

    pub fn to_headers_string(&self) -> String {
        let mut out = String::new();
        for (i, rule) in self.rules.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&rule.path_pattern);
            out.push('\n');
            for (key, val) in &rule.headers {
                out.push_str(&format!("  {}: {}\n", key, val));
            }
        }
        out
    }

    pub fn find_rule(&self, pattern: &str) -> Option<&HeaderRule> {
        self.rules.iter().find(|r| r.path_pattern == pattern)
    }

    pub fn find_rule_mut(&mut self, pattern: &str) -> Option<&mut HeaderRule> {
        self.rules.iter_mut().find(|r| r.path_pattern == pattern)
    }

    pub fn get_header_value(&self, pattern: &str, header_name: &str) -> Option<&str> {
        self.find_rule(pattern).and_then(|r| {
            r.headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(header_name))
                .map(|(_, v)| v.as_str())
        })
    }
}

pub fn find_headers_file(project_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        project_dir.join("_headers"),
        project_dir.join("public/_headers"),
        project_dir.join("dist/_headers"),
        project_dir.join("static/_headers"),
    ];

    for candidate in &candidates {
        if candidate.is_file() {
            return Some(candidate.clone());
        }
    }
    None
}
