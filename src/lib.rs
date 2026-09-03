pub mod check;
pub mod cli;
pub mod env;
pub mod headers;
pub mod routes;
pub mod session;
pub mod sync;
pub mod wrangler;

pub use check::{CheckItem, FullCheckReport, run_full_check};
pub use cli::Cli;
pub use env::{
    DevVars, EnvDiagnostic, EnvDiagnosticSeverity, EnvValidationReport, MigrationResult,
    MigrationSummary, PullResult, pull_dev_vars, scan_and_migrate, transform_source_code,
    validate_env,
};
pub use headers::{
    HeaderDiagnostic, HeaderRule, HeaderSeverity, HeaderValidationReport, HeadersFile,
    find_headers_file, generate_optimal_headers, resolve_headers_target, validate_headers,
    write_headers_file,
};
pub use routes::{
    CLOUDFLARE_MAX_RULES, RouteDiagnostic, RouteMatchResult, RouteValidationReport, RoutesConfig,
    Severity, find_static_dir, generate_routes_from_dir, matches_pattern, optimize_routes,
    pattern_subsumes, simulate_route, validate_routes, validate_routes_file, write_routes_json,
};
pub use session::{
    ASTRO_CONFIG_CANDIDATES, AstroConfigInfo, SessionCode, SessionDiagnostic, SessionInitResult,
    SessionSeverity, SessionUsage, SessionValidationReport, extract_object_block,
    find_astro_config, init_session, is_placeholder_id, parse_astro_config,
    parse_astro_config_content, scan_directory_for_session, scan_file_content,
    scan_file_for_session, validate_session,
};
pub use sync::{
    END_MARKER, GeneratorOptions, START_MARKER, SyncMode, SyncResult, TsFileAnalysis,
    analyze_env_dts, generate_complete_env_dts, generate_managed_block, sync_env_file,
};
pub use wrangler::{
    Binding, BindingKind, WranglerBindings, find_wrangler_config, parse_wrangler_content,
    parse_wrangler_file, sanitize_jsonc,
};
