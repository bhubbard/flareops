use crate::env::parser::DevVars;
use crate::wrangler::WranglerBindings;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PullResult {
    pub target_file: PathBuf,
    pub created: bool,
    pub added_keys: Vec<String>,
    pub preserved_keys: Vec<String>,
}

pub fn pull_dev_vars(
    bindings: &WranglerBindings,
    project_dir: &Path,
    as_example: bool,
    force: bool,
) -> Result<PullResult> {
    let filename = if as_example {
        ".dev.vars.example"
    } else {
        ".dev.vars"
    };
    let target_file = project_dir.join(filename);

    let existing_vars = if target_file.exists() && !force {
        let content = fs::read_to_string(&target_file)
            .with_context(|| format!("Failed to read {}", target_file.display()))?;
        DevVars::parse(&content)
    } else {
        DevVars::default()
    };

    let mut final_vars: BTreeMap<String, String> = BTreeMap::new();
    let mut added_keys = Vec::new();
    let mut preserved_keys = Vec::new();

    // 1. Collect vars from wrangler
    for b in bindings.get_vars() {
        if let Some(existing_val) = existing_vars.get(&b.name) {
            final_vars.insert(b.name.clone(), existing_val.to_string());
            preserved_keys.push(b.name.clone());
        } else {
            // Extract default value from comment if possible
            let default_val = "";
            final_vars.insert(b.name.clone(), default_val.to_string());
            added_keys.push(b.name.clone());
        }
    }

    // 2. Collect secrets from wrangler
    for b in bindings.get_secrets() {
        if let Some(existing_val) = existing_vars.get(&b.name) {
            final_vars.insert(b.name.clone(), existing_val.to_string());
            preserved_keys.push(b.name.clone());
        } else {
            let placeholder = if as_example { "your_secret_here" } else { "" };
            final_vars.insert(b.name.clone(), placeholder.to_string());
            added_keys.push(b.name.clone());
        }
    }

    // 3. Preserve any additional existing keys in .dev.vars that weren't in wrangler
    for (k, v) in &existing_vars.entries {
        if !final_vars.contains_key(k) {
            final_vars.insert(k.clone(), v.clone());
            preserved_keys.push(k.clone());
        }
    }

    let mut output = String::new();
    output.push_str("# Auto-generated/synced by flareops\n");
    output.push_str("# Cloudflare Workers & Pages local development variables (.dev.vars)\n\n");

    for (k, v) in &final_vars {
        if v.contains(' ') || v.contains('#') || v.is_empty() {
            output.push_str(&format!("{k}=\"{v}\"\n"));
        } else {
            output.push_str(&format!("{k}={v}\n"));
        }
    }

    let created = !target_file.exists();
    fs::write(&target_file, output)
        .with_context(|| format!("Failed to write {}", target_file.display()))?;

    Ok(PullResult {
        target_file,
        created,
        added_keys,
        preserved_keys,
    })
}
