use crate::headers::parser::HeadersFile;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderDiagnostic {
    pub rule: String,
    pub severity: HeaderSeverity,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HeaderValidationReport {
    pub diagnostics: Vec<HeaderDiagnostic>,
    pub astro_assets_found: usize,
    pub font_assets_found: usize,
    pub image_assets_found: usize,
    pub rules_count: usize,
}

impl HeaderValidationReport {
    pub fn is_clean(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|d| d.severity == HeaderSeverity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == HeaderSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == HeaderSeverity::Warning)
            .count()
    }
}

pub fn validate_headers(headers: &HeadersFile, dist_dir: Option<&Path>) -> HeaderValidationReport {
    let mut diagnostics = Vec::new();
    let mut astro_assets_found = 0;
    let mut font_assets_found = 0;
    let mut image_assets_found = 0;

    // 1. Inspect static asset directory if provided
    if let Some(dist) = dist_dir
        && dist.is_dir()
    {
        let astro_path = dist.join("_astro");
        if astro_path.is_dir()
            && let Ok(entries) = std::fs::read_dir(astro_path)
        {
            astro_assets_found = entries.filter_map(Result::ok).count();
        }

        // Scan for font and image assets
        for entry in walkdir::WalkDir::new(dist)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
            {
                let ext_lower = ext.to_lowercase();
                if matches!(ext_lower.as_str(), "woff2" | "woff" | "ttf" | "otf" | "eot") {
                    font_assets_found += 1;
                } else if matches!(
                    ext_lower.as_str(),
                    "png" | "jpg" | "jpeg" | "webp" | "avif" | "svg" | "ico"
                ) {
                    image_assets_found += 1;
                }
            }
        }
    }

    // 2. Syntax & General Rule Validation
    for rule in &headers.rules {
        if !rule.path_pattern.starts_with('/') && !rule.path_pattern.starts_with('*') {
            diagnostics.push(HeaderDiagnostic {
                rule: "invalid-path-pattern".to_string(),
                severity: HeaderSeverity::Error,
                message: format!(
                    "Path pattern `{}` should start with `/` or `*`.",
                    rule.path_pattern
                ),
                suggestion: Some(format!(
                    "Change to `/{}",
                    rule.path_pattern.trim_start_matches('/')
                )),
            });
        }

        for (k, v) in &rule.headers {
            if k.trim().is_empty() || v.trim().is_empty() {
                diagnostics.push(HeaderDiagnostic {
                    rule: "empty-header-entry".to_string(),
                    severity: HeaderSeverity::Error,
                    message: format!(
                        "Header rule for `{}` contains empty key or value (`{}: {}`).",
                        rule.path_pattern, k, v
                    ),
                    suggestion: Some("Specify both header name and header value".to_string()),
                });
            }

            // Check for conflicting max-age values in Cache-Control
            if k.eq_ignore_ascii_case("cache-control") {
                let lower_v = v.to_lowercase();
                let has_max_age_0 = lower_v.contains("max-age=0")
                    || lower_v.contains("no-cache")
                    || lower_v.contains("no-store");
                let has_immutable =
                    lower_v.contains("immutable") || lower_v.contains("max-age=31536000");

                if has_max_age_0 && has_immutable {
                    diagnostics.push(HeaderDiagnostic {
                        rule: "conflicting-cache-control".to_string(),
                        severity: HeaderSeverity::Error,
                        message: format!(
                            "Conflicting Cache-Control directives in rule `{}`: `{}` mixes zero max-age/no-cache with immutable/long cache.",
                            rule.path_pattern, v
                        ),
                        suggestion: Some("Separate dynamic route caching from immutable static caching".to_string()),
                    });
                }
            }
        }
    }

    // 3. Rule for `/_astro/*` immutable cache
    let astro_rule = headers.find_rule("/_astro/*");
    match astro_rule {
        None => {
            if astro_assets_found > 0 || dist_dir.is_none() {
                diagnostics.push(HeaderDiagnostic {
                    rule: "missing-astro-immutable-cache".to_string(),
                    severity: HeaderSeverity::Error,
                    message: "Missing `/_astro/*` rule in _headers. Hashed Astro bundles will not be cached immutably by Cloudflare edge.".to_string(),
                    suggestion: Some("Add `/_astro/*` with `Cache-Control: public, max-age=31536000, immutable`".to_string()),
                });
            }
        }
        Some(rule) => {
            let cache_control = rule
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("cache-control"))
                .map(|(_, v)| v.as_str());

            match cache_control {
                None => {
                    diagnostics.push(HeaderDiagnostic {
                        rule: "missing-astro-cache-control".to_string(),
                        severity: HeaderSeverity::Error,
                        message: "Rule `/_astro/*` is missing `Cache-Control` header.".to_string(),
                        suggestion: Some(
                            "Set `Cache-Control: public, max-age=31536000, immutable`".to_string(),
                        ),
                    });
                }
                Some(cc) => {
                    let cc_lower = cc.to_lowercase();
                    if !cc_lower.contains("immutable") {
                        diagnostics.push(HeaderDiagnostic {
                            rule: "astro-cache-not-immutable".to_string(),
                            severity: HeaderSeverity::Warning,
                            message: format!(
                                "`/_astro/*` Cache-Control does not include `immutable` directive: `{cc}`"
                            ),
                            suggestion: Some(
                                "Append `, immutable` to Cache-Control for fingerprinted assets"
                                    .to_string(),
                            ),
                        });
                    }
                    if !cc_lower.contains("max-age=31536000")
                        && !cc_lower.contains("max-age=315360000")
                    {
                        diagnostics.push(HeaderDiagnostic {
                            rule: "astro-cache-short-duration".to_string(),
                            severity: HeaderSeverity::Warning,
                            message: format!(
                                "`/_astro/*` Cache-Control max-age is shorter than 1 year (31536000s): `{cc}`"
                            ),
                            suggestion: Some(
                                "Set `max-age=31536000` for permanent hashed assets".to_string(),
                            ),
                        });
                    }
                }
            }
        }
    }

    // 4. Rule for `/*` global catch-all
    let global_rule = headers.find_rule("/*");
    if let Some(rule) = global_rule {
        if let Some((_, cc)) = rule
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("cache-control"))
        {
            let cc_lower = cc.to_lowercase();
            if cc_lower.contains("immutable") || cc_lower.contains("max-age=31536000") {
                diagnostics.push(HeaderDiagnostic {
                    rule: "dangerous-global-immutable-cache".to_string(),
                    severity: HeaderSeverity::Error,
                    message: format!(
                        "Catch-all `/*` rule sets aggressive Cache-Control (`{cc}`). This will cache dynamic SSR pages, authentication cookies, and HTML permanently."
                    ),
                    suggestion: Some(
                        "Use `Cache-Control: public, max-age=0, must-revalidate` for `/*`"
                            .to_string(),
                    ),
                });
            }
        }

        // Check for baseline security headers
        let has_content_type = rule
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("x-content-type-options"));
        let has_frame_options = rule
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("x-frame-options"));
        let has_referrer_policy = rule
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("referrer-policy"));

        if !has_content_type {
            diagnostics.push(HeaderDiagnostic {
                rule: "missing-x-content-type-options".to_string(),
                severity: HeaderSeverity::Warning,
                message: "Catch-all `/*` is missing `X-Content-Type-Options: nosniff` header."
                    .to_string(),
                suggestion: Some("Add `X-Content-Type-Options: nosniff`".to_string()),
            });
        }
        if !has_frame_options {
            diagnostics.push(HeaderDiagnostic {
                rule: "missing-x-frame-options".to_string(),
                severity: HeaderSeverity::Warning,
                message: "Catch-all `/*` is missing `X-Frame-Options: DENY` (or SAMEORIGIN)."
                    .to_string(),
                suggestion: Some("Add `X-Frame-Options: DENY`".to_string()),
            });
        }
        if !has_referrer_policy {
            diagnostics.push(HeaderDiagnostic {
                rule: "missing-referrer-policy".to_string(),
                severity: HeaderSeverity::Warning,
                message: "Catch-all `/*` is missing `Referrer-Policy` header.".to_string(),
                suggestion: Some(
                    "Add `Referrer-Policy: strict-origin-when-cross-origin`".to_string(),
                ),
            });
        }
    } else {
        diagnostics.push(HeaderDiagnostic {
            rule: "missing-global-security-headers".to_string(),
            severity: HeaderSeverity::Info,
            message: "No global `/*` rule found. Recommended to define baseline security headers (X-Content-Type-Options, X-Frame-Options, Referrer-Policy).".to_string(),
            suggestion: Some("Add `/*` with standard security headers".to_string()),
        });
    }

    HeaderValidationReport {
        diagnostics,
        astro_assets_found,
        font_assets_found,
        image_assets_found,
        rules_count: headers.rules.len(),
    }
}
