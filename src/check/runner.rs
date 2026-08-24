use crate::env::{EnvValidationReport, scan_and_migrate, validate_env};
use crate::routes::{RouteValidationReport, validate_routes_file};
use crate::sync::generator::{GeneratorOptions, SyncMode, generate_complete_env_dts};
use crate::wrangler::{WranglerBindings, find_wrangler_config, parse_wrangler_file};
use anyhow::Result;
use colored::Colorize;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, Row, Table};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct CheckItem {
    pub category: String,
    pub title: String,
    pub passed: bool,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Default)]
pub struct FullCheckReport {
    pub wrangler_path: Option<PathBuf>,
    pub items: Vec<CheckItem>,
    pub env_report: Option<EnvValidationReport>,
    pub routes_report: Option<RouteValidationReport>,
    pub bindings_count: usize,
}

impl FullCheckReport {
    pub fn is_clean(&self) -> bool {
        self.items.iter().all(|item| item.passed)
    }

    pub fn print_summary(&self) {
        println!();
        println!("{}", "⚡ FLAREOPS / UNIFIED VERIFICATION REPORT".bold());
        println!("{}", "═".repeat(60).dimmed());

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec!["Category", "Status", "Check", "Details"]);

        for item in &self.items {
            let status_cell = if item.passed {
                Cell::new("PASS").fg(Color::Green)
            } else {
                Cell::new("FAIL").fg(Color::Red)
            };

            let details = if let Some(ref sug) = item.suggestion {
                format!("{}\n→ {}", item.message, sug.dimmed())
            } else {
                item.message.clone()
            };

            table.add_row(Row::from(vec![
                Cell::new(&item.category),
                status_cell,
                Cell::new(&item.title),
                Cell::new(&details),
            ]));
        }

        println!("{table}");
        println!();

        if self.is_clean() {
            println!(
                "{}",
                "✔ All Cloudflare environment & route checks passed!"
                    .green()
                    .bold()
            );
        } else {
            let failed_count = self.items.iter().filter(|i| !i.passed).count();
            println!(
                "{}",
                format!("✖ {failed_count} issue(s) identified. Run suggested fix commands above.")
                    .red()
                    .bold()
            );
        }
    }
}

pub fn run_full_check(project_dir: &Path, env: Option<&str>) -> Result<FullCheckReport> {
    let mut report = FullCheckReport::default();

    // 1. Check Wrangler Config
    let wrangler_path = find_wrangler_config(project_dir);
    let bindings = if let Some(ref wpath) = wrangler_path {
        report.wrangler_path = Some(wpath.clone());
        match parse_wrangler_file(wpath, env) {
            Ok(b) => {
                report.bindings_count = b.len();
                report.items.push(CheckItem {
                    category: "Config".to_string(),
                    title: "Wrangler Configuration".to_string(),
                    passed: true,
                    message: format!(
                        "Parsed {} (found {} bindings/vars)",
                        wpath.file_name().unwrap_or_default().to_string_lossy(),
                        b.len()
                    ),
                    suggestion: None,
                });
                b
            }
            Err(e) => {
                report.items.push(CheckItem {
                    category: "Config".to_string(),
                    title: "Wrangler Configuration".to_string(),
                    passed: false,
                    message: format!("Failed to parse wrangler config: {e}"),
                    suggestion: Some("Check wrangler.jsonc / wrangler.toml syntax".to_string()),
                });
                WranglerBindings::default()
            }
        }
    } else {
        report.items.push(CheckItem {
            category: "Config".to_string(),
            title: "Wrangler Configuration".to_string(),
            passed: false,
            message: "No wrangler.jsonc or wrangler.toml found".to_string(),
            suggestion: Some("Create wrangler.jsonc in your project root".to_string()),
        });
        WranglerBindings::default()
    };

    // 2. Check TypeScript Types Sync
    let env_dts_candidates = [
        project_dir.join("cloudflare-env.d.ts"),
        project_dir.join("src/env.d.ts"),
        project_dir.join("env.d.ts"),
    ];

    let found_env_dts = env_dts_candidates.iter().find(|p| p.exists());
    if let Some(env_dts) = found_env_dts {
        if let Ok(content) = fs::read_to_string(env_dts) {
            let options = GeneratorOptions {
                mode: SyncMode::Astro,
                include_comments: true,
            };
            let expected = generate_complete_env_dts(&bindings, &options, None);
            // Check if all binding names are present in env.d.ts
            let mut missing_bindings = Vec::new();
            for b in &bindings.bindings {
                if !content.contains(&b.name) {
                    missing_bindings.push(b.name.clone());
                }
            }

            if missing_bindings.is_empty() {
                report.items.push(CheckItem {
                    category: "Types".to_string(),
                    title: "TypeScript Declarations".to_string(),
                    passed: true,
                    message: format!(
                        "{} is in sync with wrangler config",
                        env_dts.file_name().unwrap_or_default().to_string_lossy()
                    ),
                    suggestion: None,
                });
            } else {
                report.items.push(CheckItem {
                    category: "Types".to_string(),
                    title: "TypeScript Declarations".to_string(),
                    passed: false,
                    message: format!(
                        "Missing {} binding(s) in {}: {}",
                        missing_bindings.len(),
                        env_dts.display(),
                        missing_bindings.join(", ")
                    ),
                    suggestion: Some("Run `flareops sync` to regenerate types".to_string()),
                });
            }
            let _ = expected; // suppress unused
        }
    } else {
        report.items.push(CheckItem {
            category: "Types".to_string(),
            title: "TypeScript Declarations".to_string(),
            passed: false,
            message: "cloudflare-env.d.ts not found".to_string(),
            suggestion: Some("Run `flareops sync` to generate types".to_string()),
        });
    }

    // 3. Check Environment / Secrets (.dev.vars)
    let env_report = validate_env(&bindings, project_dir);
    if env_report.is_clean() {
        report.items.push(CheckItem {
            category: "Environment".to_string(),
            title: "Secrets & .dev.vars".to_string(),
            passed: true,
            message: "Local .dev.vars is valid and gitignored".to_string(),
            suggestion: None,
        });
    } else {
        let mut issues = Vec::new();
        if !env_report.dev_vars_exists && !bindings.get_secrets().is_empty() {
            issues.push("`.dev.vars` missing".to_string());
        }
        if !env_report.is_gitignored {
            issues.push("`.dev.vars` NOT in `.gitignore`".to_string());
        }
        if !env_report.missing_secrets.is_empty() {
            issues.push(format!(
                "Missing secrets: {}",
                env_report.missing_secrets.join(", ")
            ));
        }

        report.items.push(CheckItem {
            category: "Environment".to_string(),
            title: "Secrets & .dev.vars".to_string(),
            passed: false,
            message: if issues.is_empty() {
                "Environment issues detected".to_string()
            } else {
                issues.join("; ")
            },
            suggestion: Some("Run `flareops env pull` or update `.gitignore`".to_string()),
        });
    }
    report.env_report = Some(env_report);

    // 4. Check _routes.json (if present or in dist)
    let routes_candidates = [
        project_dir.join("_routes.json"),
        project_dir.join("public/_routes.json"),
        project_dir.join("dist/_routes.json"),
    ];

    if let Some(routes_path) = routes_candidates.iter().find(|p| p.exists()) {
        match validate_routes_file(routes_path) {
            Ok(rreport) => {
                let is_clean = rreport.is_clean();
                let msg = format!(
                    "{} total rules in {}",
                    rreport.total_rules,
                    routes_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                );
                report.items.push(CheckItem {
                    category: "Routes".to_string(),
                    title: "Pages _routes.json".to_string(),
                    passed: is_clean,
                    message: msg,
                    suggestion: if is_clean {
                        None
                    } else {
                        Some("Run `flareops routes optimize`".to_string())
                    },
                });
                report.routes_report = Some(rreport);
            }
            Err(e) => {
                report.items.push(CheckItem {
                    category: "Routes".to_string(),
                    title: "Pages _routes.json".to_string(),
                    passed: false,
                    message: format!("Invalid routes file: {e}"),
                    suggestion: Some("Run `flareops routes generate`".to_string()),
                });
            }
        }
    }

    // 5. Check Astro Legacy Runtime Patterns
    let migration_summary = scan_and_migrate(project_dir, true, None);
    if migration_summary.total_replacements == 0 {
        report.items.push(CheckItem {
            category: "Astro".to_string(),
            title: "Runtime Environment Calls".to_string(),
            passed: true,
            message: "No deprecated `Astro.locals.runtime.env` calls found".to_string(),
            suggestion: None,
        });
    } else {
        report.items.push(CheckItem {
            category: "Astro".to_string(),
            title: "Runtime Environment Calls".to_string(),
            passed: false,
            message: format!(
                "Found {} deprecated `locals.runtime.env` call(s) across {} file(s)",
                migration_summary.total_replacements, migration_summary.files_migrated
            ),
            suggestion: Some(
                "Run `flareops migrate` to codemod to modern `Astro.locals.*`".to_string(),
            ),
        });
    }

    Ok(report)
}
