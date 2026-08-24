use crate::routes::optimizer::optimize_routes;
use crate::routes::schema::RoutesConfig;
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn find_static_dir(project_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        "dist",
        "build/client",
        ".svelte-kit/output/client",
        "public",
        "out",
        ".output/public",
    ];

    for candidate in &candidates {
        let dir = project_dir.join(candidate);
        if dir.is_dir() {
            return Some(dir);
        }
    }

    None
}

pub fn generate_routes_from_dir(static_dir: &Path) -> Result<RoutesConfig> {
    let mut raw_excludes = BTreeSet::new();

    for entry in WalkDir::new(static_dir)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file()
            && let Ok(rel) = path.strip_prefix(static_dir)
        {
            let route_path = format!("/{}", rel.to_string_lossy().replace('\\', "/"));

            // Exclude static assets
            if is_static_asset(&route_path) {
                if route_path.starts_with("/_astro/") {
                    raw_excludes.insert("/_astro/*".to_string());
                } else if route_path.starts_with("/assets/") {
                    raw_excludes.insert("/assets/*".to_string());
                } else if route_path.starts_with("/static/") {
                    raw_excludes.insert("/static/*".to_string());
                } else if route_path.starts_with("/images/") {
                    raw_excludes.insert("/images/*".to_string());
                } else if route_path.starts_with("/fonts/") {
                    raw_excludes.insert("/fonts/*".to_string());
                } else if route_path.starts_with("/media/") {
                    raw_excludes.insert("/media/*".to_string());
                } else {
                    raw_excludes.insert(route_path);
                }
            }
        }
    }

    let unoptimized = RoutesConfig {
        version: 1,
        include: vec!["/*".to_string()],
        exclude: raw_excludes.into_iter().collect(),
    };

    Ok(optimize_routes(&unoptimized))
}

fn is_static_asset(path: &str) -> bool {
    let static_extensions = [
        ".js", ".mjs", ".css", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".avif", ".ico",
        ".woff", ".woff2", ".ttf", ".eot", ".otf", ".wasm", ".json", ".xml", ".txt", ".map",
        ".pdf", ".mp4", ".webm", ".mp3",
    ];

    static_extensions.iter().any(|ext| path.ends_with(ext))
}

pub fn write_routes_json(config: &RoutesConfig, out_path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(config).context("Failed to serialize _routes.json")?;
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create parent directory for {}",
                out_path.display()
            )
        })?;
    }
    fs::write(out_path, format!("{json}\n"))
        .with_context(|| format!("Failed to write {}", out_path.display()))?;
    Ok(())
}
