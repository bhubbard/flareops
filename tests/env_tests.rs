use flareops::env::{DevVars, pull_dev_vars, transform_source_code, validate_env};
use flareops::wrangler::parse_wrangler_content;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_dev_vars_parser() {
    let content = r#"
    # Comment line
    API_KEY=secret123
    PORT="8080"
    DEBUG='true'
    export DB_PASS="quoted pass # with hash"
    "#;

    let parsed = DevVars::parse(content);
    assert_eq!(parsed.get("API_KEY"), Some("secret123"));
    assert_eq!(parsed.get("PORT"), Some("8080"));
    assert_eq!(parsed.get("DEBUG"), Some("true"));
    assert_eq!(parsed.get("DB_PASS"), Some("quoted pass # with hash"));
}

#[test]
fn test_env_pull_preserves_existing_secrets() {
    let dir = tempdir().unwrap();
    let dev_vars_file = dir.path().join(".dev.vars");
    fs::write(
        &dev_vars_file,
        "SECRET_KEY=\"my_real_secret\"\nOLD_CUSTOM_VAR=\"persisted\"\n",
    )
    .unwrap();

    let jsonc = r#"{
        "vars": {
            "API_URL": "https://api.com"
        },
        "secrets": ["SECRET_KEY", "NEW_API_TOKEN"]
    }"#;

    let bindings = parse_wrangler_content(jsonc, false, None).unwrap();
    let res = pull_dev_vars(&bindings, dir.path(), false, false).unwrap();

    assert!(!res.created);
    let content = fs::read_to_string(&dev_vars_file).unwrap();
    let parsed = DevVars::parse(&content);

    // Existing secret preserved!
    assert_eq!(parsed.get("SECRET_KEY"), Some("my_real_secret"));
    // Existing untracked key preserved!
    assert_eq!(parsed.get("OLD_CUSTOM_VAR"), Some("persisted"));
    // New secret and var added
    assert!(parsed.contains_key("NEW_API_TOKEN"));
    assert!(parsed.contains_key("API_URL"));
}

#[test]
fn test_env_validation_security() {
    let dir = tempdir().unwrap();
    let jsonc = r#"{
        "secrets": ["STRIPE_KEY", "DATABASE_PASSWORD"]
    }"#;
    let bindings = parse_wrangler_content(jsonc, false, None).unwrap();

    // 1. Missing .dev.vars and .gitignore
    let report1 = validate_env(&bindings, dir.path());
    assert!(!report1.is_clean());
    assert!(!report1.dev_vars_exists);
    assert!(!report1.is_gitignored);

    // 2. Create .gitignore and .dev.vars with all secrets
    fs::write(dir.path().join(".gitignore"), ".dev.vars\nnode_modules\n").unwrap();
    fs::write(
        dir.path().join(".dev.vars"),
        "STRIPE_KEY=sk_test_123\nDATABASE_PASSWORD=strongpass\n",
    )
    .unwrap();

    let report2 = validate_env(&bindings, dir.path());
    assert!(report2.is_clean());
    assert!(report2.is_gitignored);
    assert!(report2.dev_vars_exists);
    assert!(report2.missing_secrets.is_empty());
}

#[test]
fn test_env_migrate_astro_v5() {
    let raw_source = r#"---
const db = Astro.locals.runtime.env.DB;
const auth = context.locals.runtime.env.AUTH;
---
<script>
  const x = locals.runtime.env.KEY;
</script>
"#;

    let (transformed, count) = transform_source_code(raw_source);
    assert_eq!(count, 3);
    assert!(transformed.contains("const db = Astro.locals.DB;"));
    assert!(transformed.contains("const auth = context.locals.AUTH;"));
    assert!(transformed.contains("const x = locals.KEY;"));
    assert!(!transformed.contains("runtime.env"));
}
