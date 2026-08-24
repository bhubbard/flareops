use crate::sync::generator::{END_MARKER, START_MARKER};
use regex::Regex;

#[derive(Debug, Default)]
pub struct TsFileAnalysis {
    pub has_managed_block: bool,
    pub managed_block_range: Option<(usize, usize)>,
    pub custom_locals_body: Option<String>,
}

pub fn analyze_env_dts(content: &str) -> TsFileAnalysis {
    let mut analysis = TsFileAnalysis::default();

    if let (Some(start_idx), Some(end_idx)) = (content.find(START_MARKER), content.find(END_MARKER))
        && start_idx < end_idx
    {
        analysis.has_managed_block = true;
        let actual_end = end_idx + END_MARKER.len();
        analysis.managed_block_range = Some((start_idx, actual_end));
    }

    // Extract any custom fields in interface Locals
    let locals_re = Regex::new(r"interface\s+Locals\s*(?:extends[^{]*)?\{([^}]*)\}").unwrap();
    if let Some(caps) = locals_re.captures(content)
        && let Some(body) = caps.get(1)
    {
        let body_str = body.as_str().trim();
        if !body_str.is_empty() {
            analysis.custom_locals_body = Some(body_str.to_string());
        }
    }

    analysis
}
