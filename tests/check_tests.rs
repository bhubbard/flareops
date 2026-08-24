use flareops::check::run_full_check;
use flareops::sync::{GeneratorOptions, SyncMode, sync_env_file};
use flareops::wrangler::parse_wrangler_content;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_check_passes_on_synced_project() {
    let dir = tempdir().unwrap();

    let wrangler_json = r#"{
        "name": "full-test-app",
        "kv_namespaces": [{ "binding": "MY_KV", "id": "kv-1" }],
        "secrets": ["MY_SECRET"]
    }"#;
    fs::write(dir.path().join("wrangler.jsonc"), wrangler_json).unwrap();

    let bindings = parse_wrangler_content(wrangler_json, false, None).unwrap();
    let env_dts = dir.path().join("src/env.d.ts");
    sync_env_file(
        &bindings,
        &env_dts,
        &GeneratorOptions {
            mode: SyncMode::Astro,
            include_comments: true,
        },
        false,
    )
    .unwrap();

    fs::write(dir.path().join(".gitignore"), ".dev.vars\n").unwrap();
    fs::write(dir.path().join(".dev.vars"), "MY_SECRET=\"supersecret\"\n").unwrap();

    let report = run_full_check(dir.path(), None).unwrap();
    assert!(report.is_clean());
}

#[test]
fn test_check_fails_on_out_of_sync_types_or_missing_secrets() {
    let dir = tempdir().unwrap();

    let wrangler_json = r#"{
        "name": "failing-app",
        "kv_namespaces": [{ "binding": "NEW_KV", "id": "kv-1" }],
        "secrets": ["UNCONFIGURED_SECRET"]
    }"#;
    fs::write(dir.path().join("wrangler.jsonc"), wrangler_json).unwrap();

    // Do NOT generate env.d.ts and do NOT create .dev.vars
    let report = run_full_check(dir.path(), None).unwrap();
    assert!(!report.is_clean());
}
