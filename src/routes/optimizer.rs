use crate::routes::matcher::pattern_subsumes;
use crate::routes::schema::RoutesConfig;
use std::collections::HashSet;

pub const CLOUDFLARE_MAX_RULES: usize = 100;

pub fn optimize_routes(config: &RoutesConfig) -> RoutesConfig {
    let mut optimized_include: Vec<String> = Vec::new();
    let mut optimized_exclude: Vec<String> = Vec::new();

    // 1. Deduplicate and clean include rules
    let mut seen_includes = HashSet::new();
    for rule in &config.include {
        let trimmed = rule.trim().to_string();
        if !trimmed.is_empty() && seen_includes.insert(trimmed.clone()) {
            optimized_include.push(trimmed);
        }
    }

    if optimized_include.iter().any(|r| r == "/*") {
        optimized_include = vec!["/*".to_string()];
    } else {
        let mut pruned = Vec::new();
        for i in 0..optimized_include.len() {
            let current = &optimized_include[i];
            let is_subsumed = optimized_include
                .iter()
                .enumerate()
                .any(|(j, other)| i != j && pattern_subsumes(other, current));
            if !is_subsumed {
                pruned.push(current.clone());
            }
        }
        optimized_include = pruned;
    }

    // 2. Deduplicate and clean exclude rules
    let mut seen_excludes = HashSet::new();
    for rule in &config.exclude {
        let trimmed = rule.trim().to_string();
        if !trimmed.is_empty() && seen_excludes.insert(trimmed.clone()) {
            optimized_exclude.push(trimmed);
        }
    }

    // Prune redundant sub-rules in exclude (e.g. /static/* subsumes /static/images/*)
    let mut pruned_exclude = Vec::new();
    for i in 0..optimized_exclude.len() {
        let current = &optimized_exclude[i];
        let is_subsumed = optimized_exclude
            .iter()
            .enumerate()
            .any(|(j, other)| i != j && pattern_subsumes(other, current));
        if !is_subsumed {
            pruned_exclude.push(current.clone());
        }
    }
    optimized_exclude = pruned_exclude;

    // 3. If total rules still exceed limit, cluster by top-level directories
    if optimized_include.len() + optimized_exclude.len() > CLOUDFLARE_MAX_RULES {
        optimized_exclude = cluster_exclusions(
            optimized_exclude,
            CLOUDFLARE_MAX_RULES - optimized_include.len(),
        );
    }

    RoutesConfig {
        version: config.version,
        include: optimized_include,
        exclude: optimized_exclude,
    }
}

fn cluster_exclusions(rules: Vec<String>, max_allowed: usize) -> Vec<String> {
    if rules.len() <= max_allowed {
        return rules;
    }

    let mut dir_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for r in &rules {
        if let Some(first_dir) = r.strip_prefix('/').and_then(|s| s.split('/').next())
            && !first_dir.is_empty()
            && !first_dir.contains('*')
        {
            *dir_counts.entry(format!("/{first_dir}/*")).or_insert(0) += 1;
        }
    }

    // Sort dirs with most children
    let mut sorted_dirs: Vec<(String, usize)> = dir_counts.into_iter().collect();
    sorted_dirs.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut current_rules = rules;
    for (dir_pattern, count) in sorted_dirs {
        if current_rules.len() <= max_allowed {
            break;
        }
        if count > 1 {
            let mut next_rules = Vec::new();
            next_rules.push(dir_pattern.clone());
            for r in current_rules {
                if !pattern_subsumes(&dir_pattern, &r) {
                    next_rules.push(r);
                }
            }
            current_rules = next_rules;
        }
    }

    current_rules
}
