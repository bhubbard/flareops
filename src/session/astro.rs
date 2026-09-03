use crate::session::types::AstroConfigInfo;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

pub const ASTRO_CONFIG_CANDIDATES: &[&str] = &[
    "astro.config.mjs",
    "astro.config.ts",
    "astro.config.js",
    "astro.config.cjs",
    "astro.config.mts",
];

pub fn find_astro_config(root_dir: &Path) -> Option<PathBuf> {
    for candidate in ASTRO_CONFIG_CANDIDATES {
        let path = root_dir.join(candidate);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

pub fn extract_object_block(content: &str, key: &str) -> Option<String> {
    let key_pattern = format!(r#"{}\s*:\s*\{{"#, key);
    let re = Regex::new(&key_pattern).ok()?;
    let mat = re.find(content)?;
    let start_idx = mat.end() - 1; // start at '{'

    let mut depth = 0;
    let chars: Vec<(usize, char)> = content[start_idx..].char_indices().collect();
    let mut end_idx = 0;
    let mut in_string = false;
    let mut string_char = '"';
    let mut escape = false;

    for (idx, ch) in chars {
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == string_char {
                in_string = false;
            }
            continue;
        }

        if ch == '"' || ch == '\'' || ch == '`' {
            in_string = true;
            string_char = ch;
            continue;
        }

        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                end_idx = start_idx + idx + 1;
                break;
            }
        }
    }

    if depth == 0 && end_idx > start_idx {
        Some(content[start_idx + 1..end_idx - 1].to_string())
    } else {
        None
    }
}

pub fn parse_astro_config(file_path: &Path) -> Result<AstroConfigInfo, std::io::Error> {
    let content = fs::read_to_string(file_path)?;
    Ok(parse_astro_config_content(
        &content,
        Some(file_path.to_path_buf()),
    ))
}

pub fn parse_astro_config_content(content: &str, file_path: Option<PathBuf>) -> AstroConfigInfo {
    let mut info = AstroConfigInfo {
        file_path,
        raw_content: Some(content.to_string()),
        ..Default::default()
    };

    // 1. Detect session block
    if let Some(session_block) = extract_object_block(content, "session") {
        info.has_session_config = true;

        // Extract driver
        let driver_regex = Regex::new(r#"driver\s*:\s*['"]([^'"]+)['"]"#).unwrap();
        if let Some(driver_cap) = driver_regex.captures(&session_block) {
            info.session_driver = Some(driver_cap[1].to_string());
        }

        // Extract binding / kvBinding
        let binding_regex =
            Regex::new(r#"(?:binding|kvBinding|kv_binding)\s*:\s*['"]([^'"]+)['"]"#).unwrap();
        if let Some(binding_cap) = binding_regex.captures(&session_block) {
            info.session_binding_name = Some(binding_cap[1].to_string());
        }

        // Extract cookie name
        let cookie_name_regex = Regex::new(r#"name\s*:\s*['"]([^'"]+)['"]"#).unwrap();
        if let Some(cookie_name_cap) = cookie_name_regex.captures(&session_block) {
            info.cookie_name = Some(cookie_name_cap[1].to_string());
        }

        // Extract cookie sameSite
        let same_site_regex = Regex::new(r#"sameSite\s*:\s*['"]([^'"]+)['"]"#).unwrap();
        if let Some(same_site_cap) = same_site_regex.captures(&session_block) {
            info.cookie_same_site = Some(same_site_cap[1].to_string());
        }

        // Extract cookie secure
        let secure_regex = Regex::new(r#"secure\s*:\s*(true|false)"#).unwrap();
        if let Some(sec_cap) = secure_regex.captures(&session_block) {
            info.cookie_secure = Some(&sec_cap[1] == "true");
        }

        // Extract ttl
        let ttl_regex = Regex::new(r#"ttl\s*:\s*(\d+)"#).unwrap();
        if let Some(ttl_cap) = ttl_regex.captures(&session_block)
            && let Ok(val) = ttl_cap[1].parse::<u64>()
        {
            info.cookie_ttl = Some(val);
        }
    } else {
        let session_driver_direct = Regex::new(r#"session\s*:\s*['"]([^'"]+)['"]"#).unwrap();
        if let Some(caps) = session_driver_direct.captures(content) {
            info.has_session_config = true;
            info.session_driver = Some(caps[1].to_string());
        }
    }

    if info.session_driver.is_some() && info.session_binding_name.is_none() {
        info.session_binding_name = Some("SESSION".to_string());
    }

    // 2. Detect adapter
    if content.contains("cloudflare") || content.contains("@astrojs/cloudflare") {
        info.adapter = Some("cloudflare".to_string());
    } else if content.contains("node") || content.contains("@astrojs/node") {
        info.adapter = Some("node".to_string());
    } else if content.contains("vercel") || content.contains("@astrojs/vercel") {
        info.adapter = Some("vercel".to_string());
    } else if content.contains("netlify") || content.contains("@astrojs/netlify") {
        info.adapter = Some("netlify".to_string());
    }

    // 3. Detect output mode
    let output_regex = Regex::new(r#"output\s*:\s*['"]([^'"]+)['"]"#).unwrap();
    if let Some(output_cap) = output_regex.captures(content) {
        info.output = Some(output_cap[1].to_string());
    }

    info
}
