use crate::env::parser::DevVars;
use crate::wrangler::WranglerBindings;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvDiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct EnvDiagnostic {
    pub key: String,
    pub severity: EnvDiagnosticSeverity,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Default)]
pub struct EnvValidationReport {
    pub dev_vars_exists: bool,
    pub is_gitignored: bool,
    pub missing_secrets: Vec<String>,
    pub missing_vars: Vec<String>,
    pub empty_values: Vec<String>,
    pub unmanaged_keys: Vec<String>,
    pub diagnostics: Vec<EnvDiagnostic>,
}

impl EnvValidationReport {
    pub fn is_clean(&self) -> bool {
        self.missing_secrets.is_empty()
            && self.missing_vars.is_empty()
            && self.is_gitignored
            && !self
                .diagnostics
                .iter()
                .any(|d| d.severity == EnvDiagnosticSeverity::Error)
    }
}

pub fn validate_env(bindings: &WranglerBindings, project_dir: &Path) -> EnvValidationReport {
    let mut report = EnvValidationReport::default();
    let dev_vars_path = project_dir.join(".dev.vars");

    // Check .gitignore
    let gitignore_path = project_dir.join(".gitignore");
    if gitignore_path.exists() {
        if let Ok(gitignore_content) = fs::read_to_string(&gitignore_path) {
            let is_ignored = gitignore_content.lines().any(|l| {
                let t = l.trim();
                t == ".dev.vars" || t == "*.dev.vars" || t == ".dev.vars*" || t == ".env"
            });
            report.is_gitignored = is_ignored;
            if !is_ignored {
                report.diagnostics.push(EnvDiagnostic {
                    key: ".dev.vars".to_string(),
                    severity: EnvDiagnosticSeverity::Error,
                    message: "`.dev.vars` is NOT listed in `.gitignore`! Risk of exposing secrets to source control.".to_string(),
                    suggestion: Some("Add `.dev.vars` to `.gitignore` immediately.".to_string()),
                });
            }
        }
    } else {
        report.diagnostics.push(EnvDiagnostic {
            key: ".gitignore".to_string(),
            severity: EnvDiagnosticSeverity::Warning,
            message: "No `.gitignore` found in project directory.".to_string(),
            suggestion: Some("Create `.gitignore` and add `.dev.vars`.".to_string()),
        });
    }

    if !dev_vars_path.exists() {
        report.dev_vars_exists = false;
        let secrets = bindings.get_secrets();
        if !secrets.is_empty() {
            report.diagnostics.push(EnvDiagnostic {
                key: ".dev.vars".to_string(),
                severity: EnvDiagnosticSeverity::Warning,
                message: format!(
                    "No `.dev.vars` file found, but wrangler configuration defines {} secret(s).",
                    secrets.len()
                ),
                suggestion: Some("Run `flareops env pull` to scaffold `.dev.vars`.".to_string()),
            });
        }
        return report;
    }

    report.dev_vars_exists = true;
    let content = match fs::read_to_string(&dev_vars_path) {
        Ok(c) => c,
        Err(_) => return report,
    };

    let dev_vars = DevVars::parse(&content);

    // Check required secrets
    for s in bindings.get_secrets() {
        if !dev_vars.contains_key(&s.name) {
            report.missing_secrets.push(s.name.clone());
            report.diagnostics.push(EnvDiagnostic {
                key: s.name.clone(),
                severity: EnvDiagnosticSeverity::Error,
                message: format!(
                    "Secret `{}` defined in wrangler config is missing from `.dev.vars`.",
                    s.name
                ),
                suggestion: Some(format!("Add `{}=\"...\"` to `.dev.vars`.", s.name)),
            });
        } else if let Some(val) = dev_vars.get(&s.name)
            && val.is_empty()
        {
            report.empty_values.push(s.name.clone());
            report.diagnostics.push(EnvDiagnostic {
                key: s.name.clone(),
                severity: EnvDiagnosticSeverity::Warning,
                message: format!("Secret `{}` in `.dev.vars` has an empty value.", s.name),
                suggestion: Some(format!(
                    "Provide a local development value for `{}`.",
                    s.name
                )),
            });
        }
    }

    // Check vars
    for v in bindings.get_vars() {
        if !dev_vars.contains_key(&v.name) {
            report.missing_vars.push(v.name.clone());
            report.diagnostics.push(EnvDiagnostic {
                key: v.name.clone(),
                severity: EnvDiagnosticSeverity::Info,
                message: format!("Environment variable `{}` defined in wrangler config is not explicitly overridden in `.dev.vars`.", v.name),
                suggestion: None,
            });
        }
    }

    // Check unmanaged keys
    for k in dev_vars.entries.keys() {
        let is_wrangler_var = bindings.get_vars().iter().any(|b| &b.name == k);
        let is_wrangler_secret = bindings.get_secrets().iter().any(|b| &b.name == k);
        if !is_wrangler_var && !is_wrangler_secret {
            report.unmanaged_keys.push(k.clone());
        }
    }

    report
}
