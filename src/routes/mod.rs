pub mod generator;
pub mod matcher;
pub mod optimizer;
pub mod schema;
pub mod validator;

pub use generator::{find_static_dir, generate_routes_from_dir, write_routes_json};
pub use matcher::{RouteMatchResult, matches_pattern, pattern_subsumes, simulate_route};
pub use optimizer::{CLOUDFLARE_MAX_RULES, optimize_routes};
pub use schema::RoutesConfig;
pub use validator::{
    RouteDiagnostic, RouteValidationReport, Severity, validate_routes, validate_routes_file,
};
