use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionSeverity {
    Info,
    Success,
    Warning,
    Error,
}

impl std::fmt::Display for SessionSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionSeverity::Info => write!(f, "INFO"),
            SessionSeverity::Success => write!(f, "SUCCESS"),
            SessionSeverity::Warning => write!(f, "WARNING"),
            SessionSeverity::Error => write!(f, "ERROR"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionCode {
    AstroConfigNotFound,
    SessionConfigMissing,
    SessionDriverUnsupported,
    EphemeralDriverOnCloudflare,
    AdapterNotCloudflare,
    WranglerConfigNotFound,
    KvBindingMissing,
    KvIdMissing,
    KvIdIsPlaceholder,
    KvPreviewIdMissing,
    KvPreviewIdMatchesId,
    DuplicateKvBinding,
    SessionUsedWithoutDriver,
    SessionUsedWithoutKvBinding,
    OutputModeNotServerOrHybrid,
    PrerenderPageUsesSession,
    ValidConfiguration,
}

impl SessionCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionCode::AstroConfigNotFound => "ASTRO_CONFIG_NOT_FOUND",
            SessionCode::SessionConfigMissing => "SESSION_CONFIG_MISSING",
            SessionCode::SessionDriverUnsupported => "SESSION_DRIVER_UNSUPPORTED",
            SessionCode::EphemeralDriverOnCloudflare => "EPHEMERAL_DRIVER_ON_CLOUDFLARE",
            SessionCode::AdapterNotCloudflare => "ADAPTER_NOT_CLOUDFLARE",
            SessionCode::WranglerConfigNotFound => "WRANGLER_CONFIG_NOT_FOUND",
            SessionCode::KvBindingMissing => "KV_BINDING_MISSING",
            SessionCode::KvIdMissing => "KV_ID_MISSING",
            SessionCode::KvIdIsPlaceholder => "KV_ID_IS_PLACEHOLDER",
            SessionCode::KvPreviewIdMissing => "KV_PREVIEW_ID_MISSING",
            SessionCode::KvPreviewIdMatchesId => "KV_PREVIEW_ID_MATCHES_ID",
            SessionCode::DuplicateKvBinding => "DUPLICATE_KV_BINDING",
            SessionCode::SessionUsedWithoutDriver => "SESSION_USED_WITHOUT_DRIVER",
            SessionCode::SessionUsedWithoutKvBinding => "SESSION_USED_WITHOUT_KV_BINDING",
            SessionCode::OutputModeNotServerOrHybrid => "OUTPUT_MODE_NOT_SERVER_OR_HYBRID",
            SessionCode::PrerenderPageUsesSession => "PRERENDER_PAGE_USES_SESSION",
            SessionCode::ValidConfiguration => "VALID_CONFIGURATION",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDiagnostic {
    pub severity: SessionSeverity,
    pub code: SessionCode,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub suggestion: Option<String>,
}

impl SessionDiagnostic {
    pub fn error(code: SessionCode, message: impl Into<String>) -> Self {
        Self {
            severity: SessionSeverity::Error,
            code,
            message: message.into(),
            file: None,
            line: None,
            column: None,
            suggestion: None,
        }
    }

    pub fn warning(code: SessionCode, message: impl Into<String>) -> Self {
        Self {
            severity: SessionSeverity::Warning,
            code,
            message: message.into(),
            file: None,
            line: None,
            column: None,
            suggestion: None,
        }
    }

    pub fn info(code: SessionCode, message: impl Into<String>) -> Self {
        Self {
            severity: SessionSeverity::Info,
            code,
            message: message.into(),
            file: None,
            line: None,
            column: None,
            suggestion: None,
        }
    }

    pub fn success(code: SessionCode, message: impl Into<String>) -> Self {
        Self {
            severity: SessionSeverity::Success,
            code,
            message: message.into(),
            file: None,
            line: None,
            column: None,
            suggestion: None,
        }
    }

    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionUsage {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub column_number: usize,
    pub line_content: String,
    pub expression: String,
    pub method: Option<String>,
    pub is_prerender: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AstroConfigInfo {
    pub file_path: Option<PathBuf>,
    pub has_session_config: bool,
    pub session_driver: Option<String>,
    pub session_binding_name: Option<String>,
    pub cookie_name: Option<String>,
    pub cookie_ttl: Option<u64>,
    pub cookie_same_site: Option<String>,
    pub cookie_secure: Option<bool>,
    pub adapter: Option<String>,
    pub output: Option<String>,
    pub raw_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionValidationReport {
    pub project_root: PathBuf,
    pub astro_config: AstroConfigInfo,
    pub session_usages: Vec<SessionUsage>,
    pub diagnostics: Vec<SessionDiagnostic>,
    pub error_count: usize,
    pub warning_count: usize,
    pub passed: bool,
}

impl SessionValidationReport {
    pub fn is_clean(&self) -> bool {
        self.error_count == 0
    }
}
