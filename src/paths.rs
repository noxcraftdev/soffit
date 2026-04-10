use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    let home = dirs::home_dir().expect("home dir");
    let legacy = home.join(".config/claude-statusline");
    let canonical = home.join(".config/soffit");
    if canonical.exists() {
        return canonical;
    }
    if legacy.exists() {
        return legacy;
    }
    canonical
}

pub fn widgets_dir() -> PathBuf {
    let canonical = config_dir().join("widgets");
    let legacy = config_dir().join("plugins");
    if canonical.exists() {
        return canonical;
    }
    if legacy.exists() {
        return legacy;
    }
    canonical
}

pub fn version_cache() -> &'static str {
    "/tmp/soffit-version"
}

pub fn version_lock() -> &'static str {
    "/tmp/soffit-version-fetch.lock"
}

pub fn context_pct_file(sid: &str) -> String {
    format!("/tmp/claude-context-pct-{sid}")
}

pub fn session_snapshot(sid: &str) -> String {
    format!("/tmp/soffit-session-{sid}.json")
}

pub fn self_version_cache() -> &'static str {
    "/tmp/soffit-self-version"
}

pub fn self_version_lock() -> &'static str {
    "/tmp/soffit-self-version.lock"
}

pub fn marketplace_config() -> std::path::PathBuf {
    config_dir().join("marketplace.toml")
}

pub fn marketplace_registry_cache(owner: &str, repo: &str) -> String {
    // Sanitize each segment (guards against hand-edited configs with path traversal chars)
    // and use %2F as separator to avoid collision between e.g. "foo-bar/baz" vs "foo/bar-baz"
    let safe_owner: String = owner
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
        .collect();
    let safe_repo: String = repo
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect();
    format!("/tmp/soffit-registry-{safe_owner}%2F{safe_repo}")
}
