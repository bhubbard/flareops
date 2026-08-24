use crate::session::types::{
    AstroConfigInfo, SessionCode, SessionDiagnostic, SessionSeverity, SessionUsage,
    SessionValidationReport,
};
use crate::wrangler::WranglerBindings;
use std::path::Path;

pub fn is_placeholder_id(id: &str) -> bool {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return true;
    }

    let lower = trimmed.to_lowercase();
    lower.contains("todo")
        || lower.contains("placeholder")
        || lower.contains("your_")
        || lower.contains("xxxx")
        || lower.contains("fill_me")
        || lower == "1234567890abcdef1234567890abcdef"
        || lower == "00000000000000000000000000000000"
        || lower == "default"
}

pub fn validate_session(
    project_root: &Path,
    astro_config: &AstroConfigInfo,
    wrangler_bindings: Option<&WranglerBindings>,
    session_usages: &[SessionUsage],
    expected_binding: Option<&str>,
    strict: bool,
) -> SessionValidationReport {
    let mut diagnostics = Vec::new();

    let target_binding = expected_binding
        .map(|s| s.to_string())
        .or_else(|| astro_config.session_binding_name.clone())
        .unwrap_or_else(|| "SESSION".to_string());

    // 1. Astro Config Validation
    if let Some(ref astro_path) = astro_config.file_path {
        if !astro_config.has_session_config {
            if !session_usages.is_empty() {
                diagnostics.push(
                    SessionDiagnostic::error(
                        SessionCode::SessionConfigMissing,
                        "Astro session API is used in source code, but session is not configured in astro.config.",
                    )
                    .with_file(astro_path)
                    .with_suggestion(
                        "Add `session: { driver: 'cloudflare' }` to your defineConfig in astro.config.",
                    ),
                );
            }
        } else if let Some(driver) = &astro_config.session_driver {
            match driver.as_str() {
                "cloudflare" | "cloudflare-kv" => {
                    if let Some(adapter) = &astro_config.adapter
                        && adapter != "cloudflare"
                    {
                        diagnostics.push(
                            SessionDiagnostic::warning(
                                SessionCode::AdapterNotCloudflare,
                                format!(
                                    "Session driver is set to '{driver}', but adapter is '{adapter}' instead of Cloudflare."
                                ),
                            )
                            .with_file(astro_path)
                            .with_suggestion(
                                "Ensure adapter is set to cloudflare() from '@astrojs/cloudflare'.",
                            ),
                        );
                    }
                }
                "memory" => {
                    diagnostics.push(
                        SessionDiagnostic::warning(
                            SessionCode::EphemeralDriverOnCloudflare,
                            "Session driver is set to 'memory'. Memory sessions are ephemeral and reset across Cloudflare Worker isolate lifecycles.",
                        )
                        .with_file(astro_path)
                        .with_suggestion(
                            "Switch session driver to 'cloudflare' to persist sessions in Cloudflare KV.",
                        ),
                    );
                }
                "fs" => {
                    diagnostics.push(
                        SessionDiagnostic::error(
                            SessionCode::SessionDriverUnsupported,
                            "Session driver is set to 'fs'. Filesystem session driver is not supported on Cloudflare Workers / Pages.",
                        )
                        .with_file(astro_path)
                        .with_suggestion(
                            "Switch session driver to 'cloudflare' for Cloudflare KV storage.",
                        ),
                    );
                }
                other => {
                    diagnostics.push(
                        SessionDiagnostic::warning(
                            SessionCode::SessionDriverUnsupported,
                            format!("Custom or unknown session driver '{other}' detected."),
                        )
                        .with_file(astro_path),
                    );
                }
            }
        }

        if (astro_config.has_session_config || !session_usages.is_empty())
            && astro_config.output.as_deref() == Some("static")
        {
            diagnostics.push(
                SessionDiagnostic::warning(
                    SessionCode::OutputModeNotServerOrHybrid,
                    "Project output is set to 'static', but sessions require dynamic on-demand rendering.",
                )
                .with_file(astro_path)
                .with_suggestion("Change output to 'server' or 'hybrid' in astro.config."),
            );
        }
    } else if !session_usages.is_empty() {
        diagnostics.push(
            SessionDiagnostic::error(
                SessionCode::AstroConfigNotFound,
                "Astro configuration file not found, but session API usage was detected in source files.",
            )
            .with_suggestion("Create astro.config.mjs with session: { driver: 'cloudflare' }."),
        );
    } else {
        diagnostics.push(
            SessionDiagnostic::info(
                SessionCode::AstroConfigNotFound,
                "No astro.config.mjs / astro.config.ts found in project root.",
            )
            .with_suggestion("Run `flareops session init` to configure session bindings."),
        );
    }

    // 2. Wrangler Config Validation
    let is_cf_target = astro_config
        .session_driver
        .as_deref()
        .map(|d| d == "cloudflare" || d == "cloudflare-kv")
        .unwrap_or(false)
        || astro_config.adapter.as_deref() == Some("cloudflare")
        || !session_usages.is_empty();

    if let Some(bindings) = wrangler_bindings {
        let kv_bindings: Vec<_> = bindings
            .bindings
            .iter()
            .filter(|b| b.name == target_binding && b.kind == crate::wrangler::BindingKind::Kv)
            .collect();

        if kv_bindings.is_empty() {
            diagnostics.push(
                SessionDiagnostic::error(
                    SessionCode::KvBindingMissing,
                    format!("Missing KV namespace binding '{target_binding}' in Wrangler configuration."),
                )
                .with_suggestion(format!(
                    "Run `flareops session init` or add `kv_namespaces` with binding '{target_binding}' to wrangler.jsonc"
                )),
            );
        } else if kv_bindings.len() > 1 {
            diagnostics.push(
                SessionDiagnostic::error(
                    SessionCode::DuplicateKvBinding,
                    format!("Multiple KV namespace bindings found with the name '{target_binding}'."),
                )
                .with_suggestion("Remove duplicate KV namespace entries."),
            );
        } else {
            let b = kv_bindings[0];
            if let Some(ref id) = b.id {
                if is_placeholder_id(id) {
                    diagnostics.push(
                        SessionDiagnostic::warning(
                            SessionCode::KvIdIsPlaceholder,
                            format!(
                                "KV namespace binding '{target_binding}' has placeholder id '{id}'."
                            ),
                        )
                        .with_suggestion(
                            "Create a KV namespace with `wrangler kv namespace create SESSION`.",
                        ),
                    );
                }
            } else {
                diagnostics.push(
                    SessionDiagnostic::error(
                        SessionCode::KvIdMissing,
                        format!("KV namespace binding '{target_binding}' is missing the 'id' field."),
                    )
                    .with_suggestion("Specify a valid Cloudflare KV namespace ID for 'id'."),
                );
            }

            if let Some(ref prev_id) = b.preview_id {
                if let Some(ref id) = b.id
                    && prev_id == id
                {
                    diagnostics.push(
                        SessionDiagnostic::warning(
                            SessionCode::KvPreviewIdMatchesId,
                            format!(
                                "KV namespace binding '{target_binding}' has identical 'id' and 'preview_id'."
                            ),
                        )
                        .with_suggestion("Use a separate preview KV namespace ID for local development."),
                    );
                }
            } else {
                diagnostics.push(
                    SessionDiagnostic::warning(
                        SessionCode::KvPreviewIdMissing,
                        format!("KV namespace binding '{target_binding}' is missing 'preview_id'."),
                    )
                    .with_suggestion(
                        "Create a preview KV namespace with `wrangler kv namespace create SESSION --preview`.",
                    ),
                );
            }
        }
    } else if is_cf_target {
        diagnostics.push(
            SessionDiagnostic::error(
                SessionCode::WranglerConfigNotFound,
                "Wrangler configuration was not found.",
            )
            .with_suggestion("Run `flareops session init` to scaffold wrangler.jsonc with SESSION KV binding."),
        );
    }

    // 3. Source Code Usage Usages vs Configuration
    for usage in session_usages {
        if !astro_config.has_session_config {
            diagnostics.push(
                SessionDiagnostic::error(
                    SessionCode::SessionUsedWithoutDriver,
                    format!(
                        "Astro session API ({}) is used, but session driver is not configured in astro.config.",
                        usage.expression
                    ),
                )
                .with_file(&usage.file_path)
                .with_location(usage.line_number, usage.column_number)
                .with_suggestion("Configure `session: { driver: 'cloudflare' }` in astro.config."),
            );
        }

        if let Some(bindings) = wrangler_bindings {
            let has_target_binding = bindings
                .bindings
                .iter()
                .any(|b| b.name == target_binding && b.kind == crate::wrangler::BindingKind::Kv);
            if !has_target_binding {
                diagnostics.push(
                    SessionDiagnostic::error(
                        SessionCode::SessionUsedWithoutKvBinding,
                        format!(
                            "Session used in route ({}), but KV namespace binding '{target_binding}' is missing in Wrangler config.",
                            usage.expression
                        ),
                    )
                    .with_file(&usage.file_path)
                    .with_location(usage.line_number, usage.column_number)
                    .with_suggestion(format!(
                        "Add `{target_binding}` KV namespace binding to Wrangler config."
                    )),
                );
            }
        }

        if usage.is_prerender {
            diagnostics.push(
                SessionDiagnostic::warning(
                    SessionCode::PrerenderPageUsesSession,
                    format!(
                        "Session API ({}) is called in a prerendered route (`export const prerender = true`). Session data cannot be resolved at build time.",
                        usage.expression
                    ),
                )
                .with_file(&usage.file_path)
                .with_location(usage.line_number, usage.column_number)
                .with_suggestion(
                    "Remove `export const prerender = true` or move session logic to an SSR endpoint or middleware.",
                ),
            );
        }
    }

    let error_count = diagnostics
        .iter()
        .filter(|d| d.severity == SessionSeverity::Error)
        .count();
    let warning_count = diagnostics
        .iter()
        .filter(|d| d.severity == SessionSeverity::Warning)
        .count();

    let passed = if strict {
        error_count == 0 && warning_count == 0
    } else {
        error_count == 0
    };

    if passed && diagnostics.is_empty() {
        diagnostics.push(SessionDiagnostic::success(
            SessionCode::ValidConfiguration,
            format!(
                "Astro Cloudflare KV Session is correctly configured with binding '{target_binding}'."
            ),
        ));
    }

    SessionValidationReport {
        project_root: project_root.to_path_buf(),
        astro_config: astro_config.clone(),
        session_usages: session_usages.to_vec(),
        diagnostics,
        error_count,
        warning_count,
        passed,
    }
}
