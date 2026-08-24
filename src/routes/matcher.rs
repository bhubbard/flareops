use crate::routes::schema::RoutesConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteMatchResult {
    InvokesFunction { matched_include: String },
    BypassesFunction { matched_exclude: String },
    NotHandled,
}

pub fn matches_pattern(pattern: &str, path: &str) -> bool {
    let p = pattern.trim();
    let target = path.trim();

    if p == "/*" {
        return true;
    }

    if let Some(prefix) = p.strip_suffix("/*") {
        if let Some(remainder) = target.strip_prefix(prefix) {
            return remainder.is_empty() || remainder.starts_with('/');
        }
        return false;
    }

    if let Some(prefix) = p.strip_suffix('*') {
        return target.starts_with(prefix);
    }

    p == target
}

pub fn pattern_subsumes(parent: &str, child: &str) -> bool {
    let p = parent.trim();
    let c = child.trim();

    if p == c {
        return true;
    }

    if p == "/*" {
        return true;
    }

    if let Some(parent_prefix) = p.strip_suffix("/*") {
        if let Some(child_prefix) = c.strip_suffix("/*") {
            return child_prefix.starts_with(parent_prefix)
                && (child_prefix.len() == parent_prefix.len()
                    || child_prefix.as_bytes().get(parent_prefix.len()) == Some(&b'/'));
        }
        return c.starts_with(parent_prefix)
            && (c.len() == parent_prefix.len()
                || c.as_bytes().get(parent_prefix.len()) == Some(&b'/'));
    }

    if let Some(parent_prefix) = p.strip_suffix('*') {
        return c.starts_with(parent_prefix);
    }

    false
}

pub fn simulate_route(config: &RoutesConfig, request_path: &str) -> RouteMatchResult {
    // Cloudflare Pages evaluation rule: Exclude rules take precedence over Include rules
    for exc in &config.exclude {
        if matches_pattern(exc, request_path) {
            return RouteMatchResult::BypassesFunction {
                matched_exclude: exc.clone(),
            };
        }
    }

    for inc in &config.include {
        if matches_pattern(inc, request_path) {
            return RouteMatchResult::InvokesFunction {
                matched_include: inc.clone(),
            };
        }
    }

    RouteMatchResult::NotHandled
}
