use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct MigrationResult {
    pub file_path: PathBuf,
    pub replacements_count: usize,
    pub original_content: String,
    pub new_content: String,
}

#[derive(Debug, Default)]
pub struct MigrationSummary {
    pub files_scanned: usize,
    pub files_migrated: usize,
    pub total_replacements: usize,
    pub results: Vec<MigrationResult>,
}

pub fn transform_source_code(source: &str) -> (String, usize) {
    let mut modified = source.to_string();
    let mut total_count = 0;

    let patterns = [
        (
            Regex::new(r"Astro\.locals\.runtime\.env").unwrap(),
            "Astro.locals",
        ),
        (
            Regex::new(r"context\.locals\.runtime\.env").unwrap(),
            "context.locals",
        ),
        (Regex::new(r"locals\.runtime\.env").unwrap(), "locals"),
        (
            Regex::new(
                r"APIContext\[['\x22]locals['\x22]\]\[['\x22]runtime['\x22]\]\[['\x22]env['\x22]\]",
            )
            .unwrap(),
            "APIContext['locals']",
        ),
        (
            Regex::new(r"App\.Locals\[['\x22]runtime['\x22]\]\[['\x22]env['\x22]\]").unwrap(),
            "App.Locals",
        ),
    ];

    for (re, replacement) in &patterns {
        let count = re.find_iter(&modified).count();
        if count > 0 {
            total_count += count;
            modified = re.replace_all(&modified, *replacement).to_string();
        }
    }

    (modified, total_count)
}

pub fn scan_and_migrate(
    target_path: &Path,
    dry_run: bool,
    custom_extensions: Option<Vec<String>>,
) -> MigrationSummary {
    let mut allowed_exts: HashSet<String> = [
        "astro", "ts", "js", "tsx", "jsx", "mjs", "cjs", "mts", "cts",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    if let Some(exts) = custom_extensions {
        allowed_exts = exts.into_iter().collect();
    }

    let mut summary = MigrationSummary::default();

    for entry in WalkDir::new(target_path)
        .into_iter()
        .filter_entry(|e| !is_ignored(e.path()))
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && is_matching_extension(path, &allowed_exts) {
            summary.files_scanned += 1;
            if let Ok(content) = fs::read_to_string(path) {
                let (transformed, count) = transform_source_code(&content);
                if count > 0 {
                    summary.files_migrated += 1;
                    summary.total_replacements += count;

                    if !dry_run {
                        let _ = fs::write(path, &transformed);
                    }

                    summary.results.push(MigrationResult {
                        file_path: path.to_path_buf(),
                        replacements_count: count,
                        original_content: content,
                        new_content: transformed,
                    });
                }
            }
        }
    }

    summary
}

fn is_ignored(path: &Path) -> bool {
    let ignored_dirs = [
        "node_modules",
        ".git",
        "dist",
        ".astro",
        ".wrangler",
        "target",
        ".output",
    ];
    for comp in path.components() {
        if let std::path::Component::Normal(c) = comp
            && let Some(s) = c.to_str()
            && ignored_dirs.contains(&s)
        {
            return true;
        }
    }
    false
}

fn is_matching_extension(path: &Path, allowed: &HashSet<String>) -> bool {
    let name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
    if name.ends_with(".d.ts") {
        return false;
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        allowed.contains(ext)
    } else {
        false
    }
}
