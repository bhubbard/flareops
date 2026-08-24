use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutesConfig {
    pub version: u32,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Default for RoutesConfig {
    fn default() -> Self {
        Self {
            version: 1,
            include: vec!["/*".to_string()],
            exclude: vec!["/_astro/*".to_string(), "/favicon.ico".to_string()],
        }
    }
}

impl RoutesConfig {
    pub fn total_rules(&self) -> usize {
        self.include.len() + self.exclude.len()
    }
}
