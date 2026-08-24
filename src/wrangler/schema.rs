use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingKind {
    Kv,
    D1,
    R2,
    Queue,
    Vectorize,
    Hyperdrive,
    Ai,
    Service,
    AnalyticsEngine,
    Browser,
    DurableObject,
    Workflow,
    SendEmail,
    RateLimit,
    DispatchNamespace,
    MtlsCertificate,
    Assets,
    Var,
    Secret,
}

impl BindingKind {
    pub fn default_ts_type(&self) -> &'static str {
        match self {
            BindingKind::Kv => "KVNamespace",
            BindingKind::D1 => "D1Database",
            BindingKind::R2 => "R2Bucket",
            BindingKind::Queue => "Queue",
            BindingKind::Vectorize => "VectorizeIndex",
            BindingKind::Hyperdrive => "Hyperdrive",
            BindingKind::Ai => "Ai",
            BindingKind::Service => "Fetcher",
            BindingKind::AnalyticsEngine => "AnalyticsEngineDataset",
            BindingKind::Browser => "Fetcher",
            BindingKind::DurableObject => "DurableObjectNamespace",
            BindingKind::Workflow => "Workflow",
            BindingKind::SendEmail => "SendEmail",
            BindingKind::RateLimit => "RateLimit",
            BindingKind::DispatchNamespace => "DispatchNamespace",
            BindingKind::MtlsCertificate => "MtlsCertificate",
            BindingKind::Assets => "Fetcher",
            BindingKind::Var => "string",
            BindingKind::Secret => "string",
        }
    }

    pub fn workers_type_import(&self) -> Option<&'static str> {
        match self {
            BindingKind::Kv => Some("KVNamespace"),
            BindingKind::D1 => Some("D1Database"),
            BindingKind::R2 => Some("R2Bucket"),
            BindingKind::Queue => Some("Queue"),
            BindingKind::Vectorize => Some("VectorizeIndex"),
            BindingKind::Hyperdrive => Some("Hyperdrive"),
            BindingKind::Ai => Some("Ai"),
            BindingKind::Service => Some("Fetcher"),
            BindingKind::AnalyticsEngine => Some("AnalyticsEngineDataset"),
            BindingKind::Browser => Some("Fetcher"),
            BindingKind::DurableObject => Some("DurableObjectNamespace"),
            BindingKind::Workflow => Some("Workflow"),
            BindingKind::SendEmail => Some("SendEmail"),
            BindingKind::RateLimit => Some("RateLimit"),
            BindingKind::DispatchNamespace => Some("DispatchNamespace"),
            BindingKind::MtlsCertificate => Some("MtlsCertificate"),
            BindingKind::Assets => Some("Fetcher"),
            BindingKind::Var | BindingKind::Secret => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub name: String,
    pub kind: BindingKind,
    pub custom_type: Option<String>,
    pub comment: Option<String>,
}

impl Binding {
    pub fn new(name: impl Into<String>, kind: BindingKind) -> Self {
        Self {
            name: name.into(),
            kind,
            custom_type: None,
            comment: None,
        }
    }

    pub fn with_custom_type(
        name: impl Into<String>,
        kind: BindingKind,
        ts_type: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            custom_type: Some(ts_type.into()),
            comment: None,
        }
    }

    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        let c = comment.into();
        if !c.is_empty() {
            self.comment = Some(c);
        }
        self
    }

    pub fn ts_type(&self) -> &str {
        if let Some(ref ct) = self.custom_type {
            ct.as_str()
        } else {
            self.kind.default_ts_type()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WranglerBindings {
    pub project_name: Option<String>,
    pub compatibility_date: Option<String>,
    pub bindings: Vec<Binding>,
}

impl WranglerBindings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, binding: Binding) {
        if let Some(pos) = self.bindings.iter().position(|b| b.name == binding.name) {
            self.bindings[pos] = binding;
        } else {
            self.bindings.push(binding);
        }
        self.bindings.sort_by(|a, b| a.name.cmp(&b.name));
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn required_workers_types(&self) -> Vec<String> {
        let mut set = HashSet::new();
        for b in &self.bindings {
            if let Some(import_type) = b.kind.workers_type_import() {
                set.insert(import_type.to_string());
            }
        }
        let mut sorted: Vec<String> = set.into_iter().collect();
        sorted.sort();
        sorted
    }

    pub fn get_vars(&self) -> Vec<&Binding> {
        self.bindings
            .iter()
            .filter(|b| b.kind == BindingKind::Var)
            .collect()
    }

    pub fn get_secrets(&self) -> Vec<&Binding> {
        self.bindings
            .iter()
            .filter(|b| b.kind == BindingKind::Secret)
            .collect()
    }
}

// Raw deserialization structures supporting both JSON and TOML
#[derive(Debug, Default, Deserialize)]
pub struct RawWranglerConfig {
    pub name: Option<String>,
    pub compatibility_date: Option<String>,

    #[serde(default)]
    pub kv_namespaces: Vec<RawKvBinding>,
    #[serde(default)]
    pub d1_databases: Vec<RawD1Binding>,
    #[serde(default)]
    pub r2_buckets: Vec<RawR2Binding>,
    #[serde(default)]
    pub vectorize: Vec<RawVectorizeBinding>,
    #[serde(default)]
    pub queues: RawQueuesConfig,
    #[serde(default)]
    pub hyperdrive: Vec<RawHyperdriveBinding>,
    #[serde(default)]
    pub ai: Option<RawAiBinding>,
    #[serde(default)]
    pub services: Vec<RawServiceBinding>,
    #[serde(default)]
    pub analytics_engine_datasets: Vec<RawAnalyticsBinding>,
    #[serde(default)]
    pub browser: Option<RawBrowserBinding>,
    #[serde(default)]
    pub durable_objects: RawDurableObjectsConfig,
    #[serde(default)]
    pub workflows: Vec<RawWorkflowBinding>,
    #[serde(default)]
    pub send_email: Vec<RawSendEmailBinding>,
    #[serde(default)]
    pub ratelimits: Vec<RawRateLimitBinding>,
    #[serde(default)]
    pub ratelimit: Option<RawRateLimitBinding>,
    #[serde(default)]
    pub dispatch_namespaces: Vec<RawDispatchNamespaceBinding>,
    #[serde(default)]
    pub mtls_certificates: Vec<RawMtlsBinding>,
    #[serde(default)]
    pub assets: Option<RawAssetsConfig>,
    #[serde(default)]
    pub vars: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub secrets: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, RawWranglerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct RawKvBinding {
    pub binding: String,
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawD1Binding {
    pub binding: String,
    pub database_name: Option<String>,
    pub database_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawR2Binding {
    pub binding: String,
    pub bucket_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawVectorizeBinding {
    pub binding: String,
    pub index_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawQueuesConfig {
    #[serde(default)]
    pub producers: Vec<RawQueueProducer>,
    #[serde(default)]
    pub consumers: Vec<RawQueueConsumer>,
}

#[derive(Debug, Deserialize)]
pub struct RawQueueProducer {
    pub binding: String,
    pub queue: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawQueueConsumer {
    pub queue: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawHyperdriveBinding {
    pub binding: String,
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawAiBinding {
    pub binding: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawServiceBinding {
    pub binding: String,
    pub service: Option<String>,
    pub environment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawAnalyticsBinding {
    pub binding: String,
    pub dataset: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawBrowserBinding {
    pub binding: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawDurableObjectsConfig {
    #[serde(default)]
    pub bindings: Vec<RawDurableObjectBinding>,
}

#[derive(Debug, Deserialize)]
pub struct RawDurableObjectBinding {
    pub name: String,
    pub class_name: Option<String>,
    pub script_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawWorkflowBinding {
    pub name: Option<String>,
    pub binding: Option<String>,
    pub class_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawSendEmailBinding {
    pub name: Option<String>,
    pub binding: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawRateLimitBinding {
    pub binding: String,
    pub namespace_id: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawDispatchNamespaceBinding {
    pub binding: String,
    pub namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawMtlsBinding {
    pub binding: String,
    pub certificate_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawAssetsConfig {
    Object {
        binding: Option<String>,
        directory: Option<String>,
    },
    String(String),
}
