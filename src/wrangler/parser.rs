use crate::wrangler::sanitize::sanitize_jsonc;
use crate::wrangler::schema::{
    Binding, BindingKind, RawAssetsConfig, RawWranglerConfig, WranglerBindings,
};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn find_wrangler_config(start_dir: &Path) -> Option<PathBuf> {
    let mut current = if start_dir.is_file() {
        start_dir.parent()?.to_path_buf()
    } else {
        start_dir.to_path_buf()
    };

    let candidates = ["wrangler.jsonc", "wrangler.json", "wrangler.toml"];

    loop {
        for candidate in &candidates {
            let path = current.join(candidate);
            if path.is_file() {
                return Some(path);
            }
        }

        if !current.pop() {
            break;
        }
    }

    None
}

pub fn parse_wrangler_file(path: &Path, env: Option<&str>) -> Result<WranglerBindings> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read wrangler file: {}", path.display()))?;

    let is_toml = path.extension().map(|ext| ext == "toml").unwrap_or(false);

    parse_wrangler_content(&content, is_toml, env)
}

pub fn parse_wrangler_content(
    content: &str,
    is_toml: bool,
    env: Option<&str>,
) -> Result<WranglerBindings> {
    let mut raw: RawWranglerConfig = if is_toml {
        toml::from_str(content).context("Failed to parse wrangler.toml")?
    } else {
        let sanitized = sanitize_jsonc(content);
        serde_json::from_str(&sanitized).context("Failed to parse wrangler JSON/JSONC")?
    };

    if let Some(target_env) = env
        && let Some(env_config) = raw.env.remove(target_env)
    {
        merge_env_config(&mut raw, env_config);
    }

    Ok(convert_raw_to_bindings(raw))
}

fn merge_env_config(base: &mut RawWranglerConfig, env_override: RawWranglerConfig) {
    if env_override.name.is_some() {
        base.name = env_override.name;
    }
    if env_override.compatibility_date.is_some() {
        base.compatibility_date = env_override.compatibility_date;
    }
    if !env_override.kv_namespaces.is_empty() {
        base.kv_namespaces = env_override.kv_namespaces;
    }
    if !env_override.d1_databases.is_empty() {
        base.d1_databases = env_override.d1_databases;
    }
    if !env_override.r2_buckets.is_empty() {
        base.r2_buckets = env_override.r2_buckets;
    }
    if !env_override.vectorize.is_empty() {
        base.vectorize = env_override.vectorize;
    }
    if !env_override.queues.producers.is_empty() || !env_override.queues.consumers.is_empty() {
        base.queues = env_override.queues;
    }
    if !env_override.hyperdrive.is_empty() {
        base.hyperdrive = env_override.hyperdrive;
    }
    if env_override.ai.is_some() {
        base.ai = env_override.ai;
    }
    if !env_override.services.is_empty() {
        base.services = env_override.services;
    }
    if !env_override.analytics_engine_datasets.is_empty() {
        base.analytics_engine_datasets = env_override.analytics_engine_datasets;
    }
    if env_override.browser.is_some() {
        base.browser = env_override.browser;
    }
    if !env_override.durable_objects.bindings.is_empty() {
        base.durable_objects = env_override.durable_objects;
    }
    if !env_override.workflows.is_empty() {
        base.workflows = env_override.workflows;
    }
    if !env_override.send_email.is_empty() {
        base.send_email = env_override.send_email;
    }
    if !env_override.ratelimits.is_empty() {
        base.ratelimits = env_override.ratelimits;
    }
    if env_override.ratelimit.is_some() {
        base.ratelimit = env_override.ratelimit;
    }
    if !env_override.dispatch_namespaces.is_empty() {
        base.dispatch_namespaces = env_override.dispatch_namespaces;
    }
    if !env_override.mtls_certificates.is_empty() {
        base.mtls_certificates = env_override.mtls_certificates;
    }
    if env_override.assets.is_some() {
        base.assets = env_override.assets;
    }
    for (k, v) in env_override.vars {
        base.vars.insert(k, v);
    }
    for secret in env_override.secrets {
        if !base.secrets.contains(&secret) {
            base.secrets.push(secret);
        }
    }
}

fn convert_raw_to_bindings(raw: RawWranglerConfig) -> WranglerBindings {
    let mut wb = WranglerBindings {
        project_name: raw.name,
        compatibility_date: raw.compatibility_date,
        bindings: Vec::new(),
    };

    // KV
    for kv in &raw.kv_namespaces {
        let comment = kv.id.as_ref().map(|id| format!("KV Namespace ID: {id}"));
        let mut b = Binding::new(&kv.binding, BindingKind::Kv);
        b.comment = comment;
        b.id = kv.id.clone();
        b.preview_id = kv.preview_id.clone();
        wb.add(b);
    }

    // D1
    for d1 in &raw.d1_databases {
        let comment = match (&d1.database_name, &d1.database_id) {
            (Some(name), Some(id)) => Some(format!("D1 Database: {name} (ID: {id})")),
            (Some(name), None) => Some(format!("D1 Database: {name}")),
            (None, Some(id)) => Some(format!("D1 Database ID: {id}")),
            (None, None) => None,
        };
        let mut b = Binding::new(&d1.binding, BindingKind::D1);
        b.comment = comment;
        wb.add(b);
    }

    // R2
    for r2 in &raw.r2_buckets {
        let comment = r2
            .bucket_name
            .as_ref()
            .map(|name| format!("R2 Bucket: {name}"));
        let mut b = Binding::new(&r2.binding, BindingKind::R2);
        b.comment = comment;
        wb.add(b);
    }

    // Vectorize
    for vec in &raw.vectorize {
        let comment = vec
            .index_name
            .as_ref()
            .map(|name| format!("Vectorize Index: {name}"));
        let mut b = Binding::new(&vec.binding, BindingKind::Vectorize);
        b.comment = comment;
        wb.add(b);
    }

    // Queues
    for q in &raw.queues.producers {
        let comment = q
            .queue
            .as_ref()
            .map(|name| format!("Queue Producer: {name}"));
        let mut b = Binding::new(&q.binding, BindingKind::Queue);
        b.comment = comment;
        wb.add(b);
    }

    // Hyperdrive
    for hd in &raw.hyperdrive {
        let comment = hd
            .id
            .as_ref()
            .map(|id| format!("Hyperdrive Config ID: {id}"));
        let mut b = Binding::new(&hd.binding, BindingKind::Hyperdrive);
        b.comment = comment;
        wb.add(b);
    }

    // AI
    if let Some(ai) = &raw.ai {
        let name = ai.binding.as_deref().unwrap_or("AI");
        wb.add(Binding::new(name, BindingKind::Ai).with_comment("Workers AI model binding"));
    }

    // Services
    for svc in &raw.services {
        let comment = match (&svc.service, &svc.environment) {
            (Some(s), Some(e)) => Some(format!("Service Binding: {s} ({e})")),
            (Some(s), None) => Some(format!("Service Binding: {s}")),
            _ => None,
        };
        let mut b = Binding::new(&svc.binding, BindingKind::Service);
        b.comment = comment;
        wb.add(b);
    }

    // Analytics Engine
    for ae in &raw.analytics_engine_datasets {
        let comment = ae
            .dataset
            .as_ref()
            .map(|d| format!("Analytics Engine Dataset: {d}"));
        let mut b = Binding::new(&ae.binding, BindingKind::AnalyticsEngine);
        b.comment = comment;
        wb.add(b);
    }

    // Browser Rendering
    if let Some(br) = &raw.browser {
        let name = br.binding.as_deref().unwrap_or("BROWSER");
        wb.add(Binding::new(name, BindingKind::Browser).with_comment("Browser Rendering binding"));
    }

    // Durable Objects
    for do_binding in &raw.durable_objects.bindings {
        let comment = do_binding
            .class_name
            .as_ref()
            .map(|c| format!("Durable Object: {c}"));
        let mut b = Binding::new(&do_binding.name, BindingKind::DurableObject);
        b.comment = comment;
        wb.add(b);
    }

    // Workflows
    for wf in &raw.workflows {
        let name = wf.binding.as_ref().or(wf.name.as_ref());
        if let Some(binding_name) = name {
            let comment = wf
                .class_name
                .as_ref()
                .map(|c| format!("Workflow Class: {c}"));
            let mut b = Binding::new(binding_name, BindingKind::Workflow);
            b.comment = comment;
            wb.add(b);
        }
    }

    // Send Email
    for em in &raw.send_email {
        let name = em.binding.as_ref().or(em.name.as_ref());
        if let Some(binding_name) = name {
            wb.add(
                Binding::new(binding_name, BindingKind::SendEmail)
                    .with_comment("Cloudflare Send Email binding"),
            );
        }
    }

    // Rate Limiting
    for rl in &raw.ratelimits {
        wb.add(
            Binding::new(&rl.binding, BindingKind::RateLimit).with_comment("Rate Limiter binding"),
        );
    }
    if let Some(rl) = &raw.ratelimit {
        wb.add(
            Binding::new(&rl.binding, BindingKind::RateLimit).with_comment("Rate Limiter binding"),
        );
    }

    // Dispatch Namespaces
    for dn in &raw.dispatch_namespaces {
        let comment = dn
            .namespace
            .as_ref()
            .map(|ns| format!("Dispatch Namespace: {ns}"));
        let mut b = Binding::new(&dn.binding, BindingKind::DispatchNamespace);
        b.comment = comment;
        wb.add(b);
    }

    // MTLS Certificates
    for mtls in &raw.mtls_certificates {
        let comment = mtls
            .certificate_id
            .as_ref()
            .map(|id| format!("mTLS Certificate ID: {id}"));
        let mut b = Binding::new(&mtls.binding, BindingKind::MtlsCertificate);
        b.comment = comment;
        wb.add(b);
    }

    // Assets
    if let Some(assets) = &raw.assets {
        match assets {
            RawAssetsConfig::Object {
                binding: Some(binding_name),
                ..
            } => {
                wb.add(
                    Binding::new(binding_name, BindingKind::Assets)
                        .with_comment("Cloudflare Assets fetcher binding"),
                );
            }
            RawAssetsConfig::Object { binding: None, .. } | RawAssetsConfig::String(_) => {}
        }
    }

    // Vars
    for (k, v) in &raw.vars {
        let (ts_type, comment) = match v {
            serde_json::Value::Bool(_) => (
                "boolean".to_string(),
                Some("Environment variable (boolean)".to_string()),
            ),
            serde_json::Value::Number(_) => (
                "number".to_string(),
                Some("Environment variable (number)".to_string()),
            ),
            serde_json::Value::String(s) => (
                "string".to_string(),
                Some(format!("Environment variable (value: {s:?})")),
            ),
            serde_json::Value::Null => (
                "string".to_string(),
                Some("Environment variable".to_string()),
            ),
            _ => (
                "unknown".to_string(),
                Some("Environment variable".to_string()),
            ),
        };
        let b = Binding::with_custom_type(k, BindingKind::Var, ts_type);
        wb.add(b.with_comment(comment.unwrap_or_default()));
    }

    // Secrets
    for secret in &raw.secrets {
        let b = Binding::with_custom_type(secret, BindingKind::Secret, "string")
            .with_comment("Wrangler secret");
        wb.add(b);
    }

    wb
}
