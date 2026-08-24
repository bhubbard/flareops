use crate::sync::analyzer::analyze_env_dts;
use crate::sync::generator::{GeneratorOptions, generate_complete_env_dts, generate_managed_block};
use crate::wrangler::WranglerBindings;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub file_path: PathBuf,
    pub created: bool,
    pub changed: bool,
    pub bindings_count: usize,
    pub content: String,
}

pub fn sync_env_file(
    bindings: &WranglerBindings,
    out_path: &Path,
    options: &GeneratorOptions,
    dry_run: bool,
) -> Result<SyncResult> {
    let (new_content, created, changed) = if out_path.exists() {
        let existing = fs::read_to_string(out_path)
            .with_context(|| format!("Failed to read existing env file: {}", out_path.display()))?;
        let analysis = analyze_env_dts(&existing);

        let content = if let Some((start, end)) = analysis.managed_block_range {
            let managed_block = generate_managed_block(bindings, options);
            let mut updated = String::with_capacity(existing.len() + managed_block.len());
            updated.push_str(&existing[..start]);
            updated.push_str(&managed_block);
            updated.push_str(&existing[end..]);
            updated
        } else {
            // No managed block found, generate complete file preserving custom locals if any
            generate_complete_env_dts(bindings, options, analysis.custom_locals_body.as_deref())
        };

        let is_changed = existing != content;
        (content, false, is_changed)
    } else {
        let content = generate_complete_env_dts(bindings, options, None);
        (content, true, true)
    };

    if !dry_run && changed {
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        fs::write(out_path, &new_content)
            .with_context(|| format!("Failed to write env file: {}", out_path.display()))?;
    }

    Ok(SyncResult {
        file_path: out_path.to_path_buf(),
        created,
        changed,
        bindings_count: bindings.len(),
        content: new_content,
    })
}
