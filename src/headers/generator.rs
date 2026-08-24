use crate::headers::parser::{HeaderRule, HeadersFile};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn generate_optimal_headers(mut existing: HeadersFile, dist_dir: Option<&Path>) -> HeadersFile {
    let mut has_fonts = false;
    let mut has_images = false;

    if let Some(dist) = dist_dir
        && dist.is_dir()
    {
        for entry in walkdir::WalkDir::new(dist).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
            {
                let ext_lower = ext.to_lowercase();
                if matches!(ext_lower.as_str(), "woff2" | "woff" | "ttf" | "otf") {
                    has_fonts = true;
                } else if matches!(
                    ext_lower.as_str(),
                    "png" | "jpg" | "jpeg" | "webp" | "avif" | "svg" | "ico"
                ) {
                    has_images = true;
                }
            }
        }
    }

    // 1. Ensure /_astro/* has immutable caching
    if let Some(rule) = existing.find_rule_mut("/_astro/*") {
        rule.headers.insert(
            "Cache-Control".to_string(),
            "public, max-age=31536000, immutable".to_string(),
        );
        rule.headers
            .entry("X-Content-Type-Options".to_string())
            .or_insert_with(|| "nosniff".to_string());
    } else {
        let mut headers = BTreeMap::new();
        headers.insert(
            "Cache-Control".to_string(),
            "public, max-age=31536000, immutable".to_string(),
        );
        headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        existing.rules.insert(
            0,
            HeaderRule {
                path_pattern: "/_astro/*".to_string(),
                headers,
            },
        );
    }

    // 2. Add /fonts/* if font directory/files exist and rule doesn't exist
    if has_fonts && existing.find_rule("/fonts/*").is_none() {
        let mut headers = BTreeMap::new();
        headers.insert(
            "Cache-Control".to_string(),
            "public, max-age=31536000, immutable".to_string(),
        );
        headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        existing.rules.push(HeaderRule {
            path_pattern: "/fonts/*".to_string(),
            headers,
        });
    }

    // 3. Add /images/* if image directory exists and rule doesn't exist
    if has_images && existing.find_rule("/images/*").is_none() {
        let mut headers = BTreeMap::new();
        headers.insert(
            "Cache-Control".to_string(),
            "public, max-age=604800, stale-while-revalidate=86400".to_string(),
        );
        existing.rules.push(HeaderRule {
            path_pattern: "/images/*".to_string(),
            headers,
        });
    }

    // 4. Ensure /* has safe SSR/HTML caching and security headers
    if let Some(rule) = existing.find_rule_mut("/*") {
        if let Some(cc) = rule.headers.get("Cache-Control") {
            if cc.contains("immutable") || cc.contains("max-age=31536000") {
                rule.headers.insert(
                    "Cache-Control".to_string(),
                    "public, max-age=0, must-revalidate".to_string(),
                );
            }
        } else {
            rule.headers.insert(
                "Cache-Control".to_string(),
                "public, max-age=0, must-revalidate".to_string(),
            );
        }
        rule.headers
            .entry("X-Content-Type-Options".to_string())
            .or_insert_with(|| "nosniff".to_string());
        rule.headers
            .entry("X-Frame-Options".to_string())
            .or_insert_with(|| "DENY".to_string());
        rule.headers
            .entry("Referrer-Policy".to_string())
            .or_insert_with(|| "strict-origin-when-cross-origin".to_string());
    } else {
        let mut headers = BTreeMap::new();
        headers.insert(
            "Cache-Control".to_string(),
            "public, max-age=0, must-revalidate".to_string(),
        );
        headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
        headers.insert(
            "Referrer-Policy".to_string(),
            "strict-origin-when-cross-origin".to_string(),
        );
        existing.rules.push(HeaderRule {
            path_pattern: "/*".to_string(),
            headers,
        });
    }

    existing
}

pub fn write_headers_file(headers: &HeadersFile, out_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = out_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, headers.to_headers_string())
}

pub fn resolve_headers_target(project_dir: &Path, dist_dir: Option<&Path>) -> PathBuf {
    if let Some(dist) = dist_dir {
        let dist_headers = dist.join("_headers");
        if dist_headers.exists() {
            return dist_headers;
        }
    }

    let public_headers = project_dir.join("public/_headers");
    if public_headers.exists() {
        return public_headers;
    }

    let root_headers = project_dir.join("_headers");
    if root_headers.exists() {
        return root_headers;
    }

    if let Some(dist) = dist_dir {
        dist.join("_headers")
    } else if project_dir.join("public").exists() {
        public_headers
    } else {
        root_headers
    }
}
