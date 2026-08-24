use flareops::routes::{
    RouteMatchResult, RoutesConfig, generate_routes_from_dir, matches_pattern, optimize_routes,
    pattern_subsumes, simulate_route, validate_routes,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_pattern_subsumes_and_matching() {
    assert!(pattern_subsumes("/*", "/api/*"));
    assert!(pattern_subsumes("/static/*", "/static/images/*"));
    assert!(pattern_subsumes("/static/*", "/static/style.css"));
    assert!(!pattern_subsumes("/static/images/*", "/static/*"));

    assert!(matches_pattern("/*", "/anything"));
    assert!(matches_pattern("/_astro/*", "/_astro/index.a1b2c3.js"));
    assert!(matches_pattern("/favicon.ico", "/favicon.ico"));
    assert!(!matches_pattern("/_astro/*", "/api/user"));
}

#[test]
fn test_routes_optimizer_deduplication_and_subsumption() {
    let unoptimized = RoutesConfig {
        version: 1,
        include: vec!["/*".to_string(), "/api/*".to_string()],
        exclude: vec![
            "/static/*".to_string(),
            "/static/images/*".to_string(),
            "/static/style.css".to_string(),
            "/_astro/*".to_string(),
            "/_astro/*".to_string(), // duplicate
        ],
    };

    let optimized = optimize_routes(&unoptimized);
    // /* in include should collapse include to just ["/*"]
    assert_eq!(optimized.include, vec!["/*"]);
    // /static/* should subsume /static/images/* and /static/style.css
    assert_eq!(optimized.exclude, vec!["/static/*", "/_astro/*"]);
}

#[test]
fn test_routes_generator_from_static_dir() {
    let dir = tempdir().unwrap();
    let dist = dir.path().join("dist");
    fs::create_dir_all(dist.join("_astro")).unwrap();
    fs::create_dir_all(dist.join("images")).unwrap();
    fs::create_dir_all(dist.join("api")).unwrap();

    fs::write(dist.join("_astro/bundle.123.js"), "console.log(1)").unwrap();
    fs::write(dist.join("images/logo.png"), "png").unwrap();
    fs::write(dist.join("favicon.ico"), "ico").unwrap();
    fs::write(dist.join("robots.txt"), "txt").unwrap();

    let routes = generate_routes_from_dir(&dist).unwrap();
    assert_eq!(routes.version, 1);
    assert_eq!(routes.include, vec!["/*"]);
    assert!(routes.exclude.contains(&"/_astro/*".to_string()));
    assert!(routes.exclude.contains(&"/images/*".to_string()));
    assert!(routes.exclude.contains(&"/favicon.ico".to_string()));
    assert!(routes.exclude.contains(&"/robots.txt".to_string()));
}

#[test]
fn test_routes_simulation() {
    let config = RoutesConfig {
        version: 1,
        include: vec!["/*".to_string()],
        exclude: vec!["/_astro/*".to_string(), "/favicon.ico".to_string()],
    };

    assert_eq!(
        simulate_route(&config, "/_astro/app.js"),
        RouteMatchResult::BypassesFunction {
            matched_exclude: "/_astro/*".to_string()
        }
    );

    assert_eq!(
        simulate_route(&config, "/api/checkout"),
        RouteMatchResult::InvokesFunction {
            matched_include: "/*".to_string()
        }
    );
}

#[test]
fn test_routes_validation_limits_and_diagnostics() {
    let bad_config = RoutesConfig {
        version: 2,      // invalid version
        include: vec![], // empty include
        exclude: vec!["no-leading-slash".to_string()],
    };

    let report = validate_routes(&bad_config);
    assert!(!report.is_clean());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "invalid-version")
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "empty-include-rules")
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "invalid-path-prefix")
    );
}
