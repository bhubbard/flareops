use flareops::sync::{
    END_MARKER, GeneratorOptions, START_MARKER, SyncMode, generate_complete_env_dts, sync_env_file,
};
use flareops::wrangler::{BindingKind, parse_wrangler_content};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_sync_jsonc_all_bindings() {
    let jsonc = r#"
    {
        // Cloudflare Workers config with comments & trailing comma
        "name": "enterprise-worker",
        "kv_namespaces": [{ "binding": "KV", "id": "kv-id-1" }],
        "d1_databases": [{ "binding": "DB", "database_name": "mydb", "database_id": "db-id-1" }],
        "r2_buckets": [{ "binding": "BUCKET", "bucket_name": "my-bucket" }],
        "vectorize": [{ "binding": "VECTORS", "index_name": "vector-index" }],
        "queues": {
            "producers": [{ "binding": "QUEUE", "queue": "my-queue" }],
        },
        "hyperdrive": [{ "binding": "HYPERDRIVE", "id": "hd-id" }],
        "ai": { "binding": "AI" },
        "services": [{ "binding": "AUTH_SERVICE", "service": "auth-api" }],
        "vars": {
            "API_URL": "https://api.example.com",
            "MAX_RETRIES": 5,
            "ENABLE_DEBUG": true,
        },
        "secrets": ["JWT_SECRET", "STRIPE_KEY"],
    }
    "#;

    let bindings = parse_wrangler_content(jsonc, false, None).expect("Should parse valid JSONC");
    assert_eq!(bindings.len(), 13);

    let kv = bindings.bindings.iter().find(|b| b.name == "KV").unwrap();
    assert_eq!(kv.kind, BindingKind::Kv);
    assert_eq!(kv.ts_type(), "KVNamespace");

    let db = bindings.bindings.iter().find(|b| b.name == "DB").unwrap();
    assert_eq!(db.kind, BindingKind::D1);
    assert_eq!(db.ts_type(), "D1Database");

    let jwt = bindings
        .bindings
        .iter()
        .find(|b| b.name == "JWT_SECRET")
        .unwrap();
    assert_eq!(jwt.kind, BindingKind::Secret);
    assert_eq!(jwt.ts_type(), "string");
}

#[test]
fn test_sync_toml_generation() {
    let toml_str = r#"
    name = "toml-app"

    [[kv_namespaces]]
    binding = "CACHE"
    id = "cache-id"

    [[d1_databases]]
    binding = "SQL"
    database_name = "sql-db"

    [ai]
    binding = "AI_MODEL"

    [vars]
    APP_NAME = "My Astro App"
    "#;

    let bindings = parse_wrangler_content(toml_str, true, None).expect("Should parse TOML");
    let options = GeneratorOptions {
        mode: SyncMode::Astro,
        include_comments: true,
    };

    let dts = generate_complete_env_dts(&bindings, &options, None);
    assert!(dts.contains(START_MARKER));
    assert!(dts.contains(END_MARKER));
    assert!(dts.contains("CACHE: KVNamespace;"));
    assert!(dts.contains("SQL: D1Database;"));
    assert!(dts.contains("AI_MODEL: Ai;"));
    assert!(dts.contains("APP_NAME: string;"));
    assert!(dts.contains("interface Locals extends CloudflareEnv"));
}

#[test]
fn test_sync_preserves_custom_locals_in_existing_file() {
    let dir = tempdir().unwrap();
    let env_file = dir.path().join("src/env.d.ts");
    fs::create_dir_all(env_file.parent().unwrap()).unwrap();

    let initial_content = format!(
        r#"/* eslint-disable */
{}
/// <reference types="@cloudflare/workers-types" />

import type {{ KVNamespace }} from "@cloudflare/workers-types";

export interface CloudflareEnv {{
	OLD_KV: KVNamespace;
}}
{}

declare namespace App {{
	interface Locals extends CloudflareEnv {{
		userSession?: {{ id: string; role: string }};
		customTheme: "dark" | "light";
	}}
}}

export type Env = CloudflareEnv;
"#,
        START_MARKER, END_MARKER
    );

    fs::write(&env_file, initial_content).unwrap();

    let jsonc = r#"{
        "kv_namespaces": [{ "binding": "NEW_KV", "id": "kv-2" }],
        "d1_databases": [{ "binding": "NEW_DB", "database_name": "db-2" }]
    }"#;

    let bindings = parse_wrangler_content(jsonc, false, None).unwrap();
    let options = GeneratorOptions {
        mode: SyncMode::Astro,
        include_comments: true,
    };

    let result = sync_env_file(&bindings, &env_file, &options, false).unwrap();
    assert!(result.changed);

    let updated = fs::read_to_string(&env_file).unwrap();
    assert!(updated.contains("NEW_KV: KVNamespace;"));
    assert!(updated.contains("NEW_DB: D1Database;"));
    assert!(!updated.contains("OLD_KV"));
    // Custom user locals preserved!
    assert!(updated.contains("userSession?: { id: string; role: string };"));
    assert!(updated.contains("customTheme: \"dark\" | \"light\";"));
}

#[test]
fn test_sync_worker_mode() {
    let jsonc = r#"{
        "kv_namespaces": [{ "binding": "SESSION_KV" }]
    }"#;
    let bindings = parse_wrangler_content(jsonc, false, None).unwrap();
    let options = GeneratorOptions {
        mode: SyncMode::Worker,
        include_comments: true,
    };

    let output = generate_complete_env_dts(&bindings, &options, None);
    assert!(output.contains("declare global {"));
    assert!(output.contains("type Env = CloudflareEnv;"));
}
