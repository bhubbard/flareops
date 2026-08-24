use crate::session::types::SessionUsage;
use regex::Regex;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub const SCANNED_EXTENSIONS: &[&str] = &[
    "astro", "ts", "js", "mjs", "mts", "jsx", "tsx", "cjs", "cts",
];

pub fn scan_directory_for_session(src_dir: &Path) -> Result<Vec<SessionUsage>, std::io::Error> {
    let mut usages = Vec::new();

    if !src_dir.exists() {
        return Ok(usages);
    }

    for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension().and_then(|e| e.to_str())
            && SCANNED_EXTENSIONS.contains(&ext)
            && let Ok(file_usages) = scan_file_for_session(path)
        {
            usages.extend(file_usages);
        }
    }

    Ok(usages)
}

pub fn scan_file_for_session(file_path: &Path) -> Result<Vec<SessionUsage>, std::io::Error> {
    let content = fs::read_to_string(file_path)?;
    Ok(scan_file_content(&content, file_path))
}

pub fn scan_file_content(content: &str, file_path: &Path) -> Vec<SessionUsage> {
    let mut usages = Vec::new();

    let prerender_regex = Regex::new(r"export\s+const\s+prerender\s*=\s*true").unwrap();
    let is_prerender = prerender_regex.is_match(content);

    let session_call_regex = Regex::new(
        r"(?:\bAstro\.session|\bcontext\.session|\bctx\.session|\blocals\.session)(?:\.([a-zA-Z0-9_$]+))?",
    )
    .unwrap();

    let import_session_regex = Regex::new(r#"['"]astro:session['"]"#).unwrap();

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
            continue;
        }

        if let Some(mat) = import_session_regex.find(line) {
            usages.push(SessionUsage {
                file_path: file_path.to_path_buf(),
                line_number: line_num,
                column_number: mat.start() + 1,
                line_content: line.trim().to_string(),
                expression: "import 'astro:session'".to_string(),
                method: None,
                is_prerender,
            });
        }

        for cap in session_call_regex.captures_iter(line) {
            if let Some(whole_match) = cap.get(0) {
                let method = cap.get(1).map(|m| m.as_str().to_string());
                usages.push(SessionUsage {
                    file_path: file_path.to_path_buf(),
                    line_number: line_num,
                    column_number: whole_match.start() + 1,
                    line_content: line.trim().to_string(),
                    expression: whole_match.as_str().to_string(),
                    method,
                    is_prerender,
                });
            }
        }
    }

    usages
}
