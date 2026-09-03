use crate::session::astro::find_astro_config;
use crate::wrangler::find_wrangler_config;
use crate::wrangler::sanitize::sanitize_jsonc;
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone)]
pub struct SessionInitResult {
    pub files_created: Vec<PathBuf>,
    pub files_modified: Vec<PathBuf>,
    pub messages: Vec<String>,
}

pub fn init_session(project_root: &Path, binding_name: &str) -> anyhow::Result<SessionInitResult> {
    let mut result = SessionInitResult::default();

    // 1. Scaffold Wrangler KV Binding
    let wrangler_found = find_wrangler_config(project_root);
    if let Some(wrangler_path) = wrangler_found {
        let content = fs::read_to_string(&wrangler_path)?;
        let is_toml = wrangler_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("toml"))
            .unwrap_or(false);

        if is_toml {
            if !content.contains(&format!("binding = \"{binding_name}\"")) {
                let snippet = format!(
                    "\n[[kv_namespaces]]\nbinding = \"{binding_name}\"\nid = \"<YOUR_KV_NAMESPACE_ID>\"\npreview_id = \"<YOUR_PREVIEW_KV_NAMESPACE_ID>\"\n"
                );
                let mut new_content = content;
                new_content.push_str(&snippet);
                fs::write(&wrangler_path, new_content)?;
                result.files_modified.push(wrangler_path.clone());
                result.messages.push(format!(
                    "Appended '{binding_name}' KV namespace binding to {}",
                    wrangler_path.display()
                ));
            }
        } else {
            let sanitized = sanitize_jsonc(&content);
            let mut val: Value =
                serde_json::from_str(&sanitized).unwrap_or_else(|_| serde_json::json!({}));

            let root_obj = val
                .as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("Wrangler JSON root is not an object"))?;

            let kv_array = root_obj
                .entry("kv_namespaces")
                .or_insert_with(|| Value::Array(Vec::new()))
                .as_array_mut()
                .ok_or_else(|| anyhow::anyhow!("kv_namespaces is not an array"))?;

            let binding_exists = kv_array
                .iter()
                .any(|item| item.get("binding").and_then(|b| b.as_str()) == Some(binding_name));

            if !binding_exists {
                let new_entry = serde_json::json!({
                    "binding": binding_name,
                    "id": "<YOUR_KV_NAMESPACE_ID>",
                    "preview_id": "<YOUR_PREVIEW_KV_NAMESPACE_ID>"
                });
                kv_array.push(new_entry);

                let formatted = serde_json::to_string_pretty(&val)?;
                fs::write(&wrangler_path, formatted + "\n")?;
                result.files_modified.push(wrangler_path.clone());
                result.messages.push(format!(
                    "Added '{binding_name}' KV namespace binding to {}",
                    wrangler_path.display()
                ));
            }
        }
    } else {
        // Create wrangler.jsonc
        let new_path = project_root.join("wrangler.jsonc");
        let project_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("astro-app");

        let content = format!(
            r#"{{
  "$schema": "node_modules/wrangler/config-schema.json",
  "name": "{}",
  "compatibility_date": "2024-09-23",
  "compatibility_flags": ["nodejs_compat"],
  "pages_build_output_dir": "./dist",
  "kv_namespaces": [
    {{
      "binding": "{}",
      "id": "<YOUR_KV_NAMESPACE_ID>",
      "preview_id": "<YOUR_PREVIEW_KV_NAMESPACE_ID>"
    }}
  ]
}}
"#,
            project_name, binding_name
        );
        fs::write(&new_path, content)?;
        result.files_created.push(new_path.clone());
        result.messages.push(format!(
            "Created {} with default '{binding_name}' KV namespace binding",
            new_path.display()
        ));
    }

    // 2. Configure Astro Config
    let astro_found = find_astro_config(project_root);
    if let Some(astro_path) = astro_found {
        let content = fs::read_to_string(&astro_path)?;
        if !content.contains("session:") {
            let define_config_regex = Regex::new(r"defineConfig\(\s*\{").unwrap();
            if let Some(mat) = define_config_regex.find(&content) {
                let insert_pos = mat.end();
                let session_insert = if binding_name == "SESSION" {
                    "\n  session: {\n    driver: 'cloudflare',\n  },"
                } else {
                    &format!(
                        "\n  session: {{\n    driver: 'cloudflare',\n    binding: '{binding_name}',\n  }},"
                    )
                };

                let mut new_content = String::new();
                new_content.push_str(&content[..insert_pos]);
                new_content.push_str(session_insert);
                new_content.push_str(&content[insert_pos..]);

                fs::write(&astro_path, new_content)?;
                result.files_modified.push(astro_path.clone());
                result.messages.push(format!(
                    "Injected session configuration into {}",
                    astro_path.display()
                ));
            }
        }
    } else {
        let new_astro = project_root.join("astro.config.mjs");
        let session_snippet = if binding_name == "SESSION" {
            "session: {\n    driver: 'cloudflare',\n  },"
        } else {
            &format!(
                "session: {{\n    driver: 'cloudflare',\n    binding: '{binding_name}',\n  }},"
            )
        };

        let content = format!(
            r#"import {{ defineConfig }} from 'astro/config';
import cloudflare from '@astrojs/cloudflare';

// https://astro.build/config
export default defineConfig({{
  output: 'server',
  adapter: cloudflare(),
  {}
}});
"#,
            session_snippet
        );
        fs::write(&new_astro, content)?;
        result.files_created.push(new_astro.clone());
        result.messages.push(format!(
            "Created {} with Cloudflare session configuration",
            new_astro.display()
        ));
    }

    Ok(result)
}
