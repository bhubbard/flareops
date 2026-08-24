use flareops::session::{
    AstroConfigInfo, SessionCode, SessionUsage, init_session, parse_astro_config_content,
    scan_file_content, validate_session,
};
use flareops::wrangler::parse_wrangler_content;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn test_parse_astro_config_session_block() {
    let content = r#"
        import { defineConfig } from 'astro/config';
        import cloudflare from '@astrojs/cloudflare';

        export default defineConfig({
            output: 'server',
            adapter: cloudflare(),
            session: {
                driver: 'cloudflare',
                binding: 'SESSION_KV',
                cookie: {
                    name: 'custom_session_id',
                    sameSite: 'lax',
                    secure: true
                },
                ttl: 86400
            }
        });
    "#;

    let config = parse_astro_config_content(content, Some(PathBuf::from("astro.config.mjs")));
    assert!(config.has_session_config);
    assert_eq!(config.session_driver.as_deref(), Some("cloudflare"));
    assert_eq!(config.session_binding_name.as_deref(), Some("SESSION_KV"));
    assert_eq!(config.cookie_name.as_deref(), Some("custom_session_id"));
    assert_eq!(config.adapter.as_deref(), Some("cloudflare"));
    assert_eq!(config.output.as_deref(), Some("server"));
}

#[test]
fn test_scan_astro_session_calls() {
    let content = r#"---
const user = await Astro.session.get('user');
---
<div>User: {user}</div>
"#;

    let usages = scan_file_content(content, Path::new("src/pages/dashboard.astro"));
    assert_eq!(usages.len(), 1);
    assert_eq!(usages[0].expression, "Astro.session.get");
    assert_eq!(usages[0].method.as_deref(), Some("get"));
    assert!(!usages[0].is_prerender);
}

#[test]
fn test_validate_session_matching_kv() {
    let astro = AstroConfigInfo {
        file_path: Some(PathBuf::from("astro.config.mjs")),
        has_session_config: true,
        session_driver: Some("cloudflare".to_string()),
        session_binding_name: Some("SESSION".to_string()),
        adapter: Some("cloudflare".to_string()),
        output: Some("server".to_string()),
        ..Default::default()
    };

    let wrangler_raw = r#"{
        "name": "astro-app",
        "kv_namespaces": [
            {
                "binding": "SESSION",
                "id": "e28d49fbcfcf4b31a3b37996c56b0000",
                "preview_id": "a1b2c3d4e5f600001122334455667788"
            }
        ]
    }"#;

    let bindings = parse_wrangler_content(wrangler_raw, false, None).unwrap();
    let usages = vec![SessionUsage {
        file_path: PathBuf::from("src/pages/index.astro"),
        line_number: 2,
        column_number: 14,
        line_content: "const s = Astro.session.get('x');".to_string(),
        expression: "Astro.session.get".to_string(),
        method: Some("get".to_string()),
        is_prerender: false,
    }];

    let report = validate_session(
        Path::new("."),
        &astro,
        Some(&bindings),
        &usages,
        None,
        false,
    );

    assert!(report.passed);
    assert_eq!(report.error_count, 0);
}

#[test]
fn test_validate_session_missing_kv() {
    let astro = AstroConfigInfo {
        file_path: Some(PathBuf::from("astro.config.mjs")),
        has_session_config: true,
        session_driver: Some("cloudflare".to_string()),
        session_binding_name: Some("SESSION".to_string()),
        adapter: Some("cloudflare".to_string()),
        output: Some("server".to_string()),
        ..Default::default()
    };

    let wrangler_raw = r#"{
        "name": "astro-app",
        "kv_namespaces": []
    }"#;

    let bindings = parse_wrangler_content(wrangler_raw, false, None).unwrap();
    let usages = vec![];

    let report = validate_session(
        Path::new("."),
        &astro,
        Some(&bindings),
        &usages,
        None,
        false,
    );

    assert!(!report.passed);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == SessionCode::KvBindingMissing)
    );
}

#[test]
fn test_init_session_scaffolds_project() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let res = init_session(root, "SESSION").unwrap();
    assert_eq!(res.files_created.len(), 2);
    assert!(root.join("wrangler.jsonc").exists());
    assert!(root.join("astro.config.mjs").exists());

    let wrangler_content = fs::read_to_string(root.join("wrangler.jsonc")).unwrap();
    assert!(wrangler_content.contains(r#""binding": "SESSION""#));

    let astro_content = fs::read_to_string(root.join("astro.config.mjs")).unwrap();
    assert!(astro_content.contains("driver: 'cloudflare'"));
}
