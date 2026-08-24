use crate::routes::matcher::pattern_subsumes;
use crate::routes::schema::RoutesConfig;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDiagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Default)]
pub struct RouteValidationReport {
    pub total_rules: usize,
    pub diagnostics: Vec<RouteDiagnostic>,
}

impl RouteValidationReport {
    pub fn is_clean(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

pub const CLOUDFLARE_MAX_RULES: usize = 100;
pub const CLOUDFLARE_WARN_RULES: usize = 80;

pub fn validate_routes_file(path: &Path) -> Result<RouteValidationReport> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read routes file: {}", path.display()))?;
    let config: RoutesConfig = serde_json::from_str(&content)
        .with_context(|| format!("Invalid JSON format in routes file: {}", path.display()))?;
    Ok(validate_routes(&config))
}

pub fn validate_routes(config: &RoutesConfig) -> RouteValidationReport {
    let mut diagnostics = Vec::new();
    let total = config.total_rules();

    // 1. Version check
    if config.version != 1 {
        diagnostics.push(RouteDiagnostic {
            code: "invalid-version".to_string(),
            severity: Severity::Error,
            message: format!(
                "Invalid _routes.json version `{}`. Cloudflare Pages requires `version: 1`.",
                config.version
            ),
            suggestion: Some("Change version to 1".to_string()),
        });
    }

    // 2. Include rule requirement
    if config.include.is_empty() {
        diagnostics.push(RouteDiagnostic {
            code: "empty-include-rules".to_string(),
            severity: Severity::Error,
            message: "The `include` array is empty. At least one include rule (e.g. `/*`) is required to invoke Pages Functions.".to_string(),
            suggestion: Some("Add `/*` to `include` array".to_string()),
        });
    }

    // 3. Rule count limits
    if total > CLOUDFLARE_MAX_RULES {
        diagnostics.push(RouteDiagnostic {
            code: "quota-exceeded-100-rules".to_string(),
            severity: Severity::Error,
            message: format!(
                "Total rules count ({total}) exceeds Cloudflare Pages strict 100-rule limit ({} include, {} exclude). Deployments will fail.",
                config.include.len(),
                config.exclude.len()
            ),
            suggestion: Some("Run `flareops routes optimize` to consolidate exclusions into wildcards".to_string()),
        });
    } else if total >= CLOUDFLARE_WARN_RULES {
        diagnostics.push(RouteDiagnostic {
            code: "quota-approaching-limit".to_string(),
            severity: Severity::Warning,
            message: format!(
                "Total rules count ({total}) is approaching the 100-rule ceiling ({} left).",
                CLOUDFLARE_MAX_RULES - total
            ),
            suggestion: Some(
                "Consolidate path patterns to prevent future deployment blocks".to_string(),
            ),
        });
    }

    // 4. Syntax format validation
    for (idx, rule) in config
        .include
        .iter()
        .chain(config.exclude.iter())
        .enumerate()
    {
        if !rule.starts_with('/') {
            diagnostics.push(RouteDiagnostic {
                code: "invalid-path-prefix".to_string(),
                severity: Severity::Error,
                message: format!(
                    "Rule `{rule}` (rule #{}) does not start with a forward slash (`/`).",
                    idx + 1
                ),
                suggestion: Some(format!("Prefix rule with `/`: `/{rule}`")),
            });
        }
    }

    // 5. Redundant rules inside include
    if config.include.iter().any(|r| r == "/*") && config.include.len() > 1 {
        diagnostics.push(RouteDiagnostic {
            code: "redundant-include-shadowed".to_string(),
            severity: Severity::Warning,
            message: "Include array contains `/*` alongside more specific rules. `/*` already includes all routes, making specific include rules redundant.".to_string(),
            suggestion: Some("Remove specific include rules when `/*` is already present".to_string()),
        });
    }

    // 6. Direct Conflicts: Exclude rules matching include rules
    for inc in &config.include {
        for exc in &config.exclude {
            if inc == exc {
                diagnostics.push(RouteDiagnostic {
                    code: "direct-route-conflict".to_string(),
                    severity: Severity::Warning,
                    message: format!("Rule `{inc}` is present in BOTH `include` and `exclude`. Exclude takes absolute precedence, rendering the include rule dead."),
                    suggestion: Some(format!("Remove `{inc}` from `include` or adjust exclusion")),
                });
            }
        }
    }

    // 7. Redundant exclusions (child covered by parent exclusion)
    for (i, exc1) in config.exclude.iter().enumerate() {
        for (j, exc2) in config.exclude.iter().enumerate() {
            if i != j && pattern_subsumes(exc1, exc2) {
                diagnostics.push(RouteDiagnostic {
                    code: "redundant-exclusion".to_string(),
                    severity: Severity::Info,
                    message: format!("Exclusion `{exc2}` is redundant because parent pattern `{exc1}` already excludes it."),
                    suggestion: Some(format!("Remove redundant rule `{exc2}` to save quota")),
                });
            }
        }
    }

    RouteValidationReport {
        total_rules: total,
        diagnostics,
    }
}
