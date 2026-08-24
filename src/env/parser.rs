use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct DevVars {
    pub entries: BTreeMap<String, String>,
    pub raw_lines: Vec<String>,
}

impl DevVars {
    pub fn parse(content: &str) -> Self {
        let mut entries = BTreeMap::new();
        let mut raw_lines = Vec::new();

        for line in content.lines() {
            raw_lines.push(line.to_string());
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let stripped = trimmed.strip_prefix("export ").unwrap_or(trimmed);

            if let Some((key, val)) = stripped.split_once('=') {
                let key = key.trim().to_string();
                let val = val.trim();

                // Strip quotes if any
                let val = if (val.starts_with('"') && val.ends_with('"'))
                    || (val.starts_with('\'') && val.ends_with('\''))
                {
                    if val.len() >= 2 {
                        &val[1..val.len() - 1]
                    } else {
                        val
                    }
                } else {
                    val
                };

                entries.insert(key, val.to_string());
            }
        }

        Self { entries, raw_lines }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|s| s.as_str())
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }
}
