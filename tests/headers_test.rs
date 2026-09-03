use flareops::headers::{
    HeadersFile, generate_optimal_headers, validate_headers, write_headers_file,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_parse_and_serialize_headers() {
    let raw = r#"
# Global rules
/*
  X-Frame-Options: DENY
  X-Content-Type-Options: nosniff
  Cache-Control: public, max-age=0, must-revalidate

# Static assets
/_astro/*
  Cache-Control: public, max-age=31536000, immutable
  X-Content-Type-Options: nosniff
"#;

    let parsed = HeadersFile::parse(raw);
    assert_eq!(parsed.rules.len(), 2);

    let global = parsed.find_rule("/*").expect("global rule exists");
    assert_eq!(
        global.headers.get("X-Frame-Options").map(|s| s.as_str()),
        Some("DENY")
    );

    let astro = parsed.find_rule("/_astro/*").expect("astro rule exists");
    assert_eq!(
        astro.headers.get("Cache-Control").map(|s| s.as_str()),
        Some("public, max-age=31536000, immutable")
    );

    let serialized = parsed.to_headers_string();
    assert!(serialized.contains("/*\n"));
    assert!(serialized.contains("/_astro/*\n"));
    assert!(serialized.contains("Cache-Control: public, max-age=31536000, immutable"));
}

#[test]
fn test_validate_clean_headers() {
    let raw = r#"
/_astro/*
  Cache-Control: public, max-age=31536000, immutable
  X-Content-Type-Options: nosniff

/*
  Cache-Control: public, max-age=0, must-revalidate
  X-Content-Type-Options: nosniff
  X-Frame-Options: DENY
  Referrer-Policy: strict-origin-when-cross-origin
"#;

    let parsed = HeadersFile::parse(raw);
    let report = validate_headers(&parsed, None);
    assert!(report.is_clean());
    assert_eq!(report.error_count(), 0);
    assert_eq!(report.warning_count(), 0);
}

#[test]
fn test_validate_detects_dangerous_global_immutable_and_missing_astro() {
    let raw = r#"
/*
  Cache-Control: public, max-age=31536000, immutable
"#;

    let parsed = HeadersFile::parse(raw);
    let report = validate_headers(&parsed, None);
    assert!(!report.is_clean());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.rule == "dangerous-global-immutable-cache")
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.rule == "missing-astro-immutable-cache")
    );
}

#[test]
fn test_validate_detects_conflicting_cache_control() {
    let raw = r#"
/_astro/*
  Cache-Control: public, max-age=0, max-age=31536000, immutable

/*
  Cache-Control: public, max-age=0, must-revalidate
  X-Content-Type-Options: nosniff
  X-Frame-Options: DENY
  Referrer-Policy: strict-origin-when-cross-origin
"#;

    let parsed = HeadersFile::parse(raw);
    let report = validate_headers(&parsed, None);
    assert!(!report.is_clean());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.rule == "conflicting-cache-control")
    );
}

#[test]
fn test_generate_optimal_headers_and_write() {
    let dir = tempdir().unwrap();
    let dist_dir = dir.path().join("dist");
    fs::create_dir_all(dist_dir.join("_astro")).unwrap();
    fs::create_dir_all(dist_dir.join("fonts")).unwrap();
    fs::write(
        dist_dir.join("_astro/index.a1b2c3d4.js"),
        "console.log('astro');",
    )
    .unwrap();
    fs::write(dist_dir.join("fonts/inter.woff2"), "binaryfont").unwrap();

    let optimal = generate_optimal_headers(HeadersFile::default(), Some(&dist_dir));
    assert!(optimal.find_rule("/_astro/*").is_some());
    assert!(optimal.find_rule("/*").is_some());
    assert!(optimal.find_rule("/fonts/*").is_some());

    let out_file = dir.path().join("_headers");
    write_headers_file(&optimal, &out_file).unwrap();
    assert!(out_file.exists());

    let content = fs::read_to_string(&out_file).unwrap();
    assert!(content.contains("max-age=31536000, immutable"));
    assert!(content.contains("X-Frame-Options: DENY"));
}
