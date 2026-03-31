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

pub fn plugins_dir() -> PathBuf {
    config_dir().join("plugins")
}

pub fn version_cache() -> &'static str {
    "/tmp/soffit-version"
}

pub fn version_lock() -> &'static str {
    "/tmp/soffit-version-fetch.lock"
}

pub fn git_cache(cwd_hash: &str) -> String {
    format!("/tmp/soffit-git-{cwd_hash}")
}

pub fn sid_cache() -> &'static str {
    "/tmp/soffit-sids"
}

pub fn context_pct_file(sid: &str) -> String {
    format!("/tmp/claude-context-pct-{sid}")
}

pub fn session_snapshot(sid: &str) -> String {
    format!("/tmp/soffit-session-{sid}.json")
}

pub fn cost_lock() -> &'static str {
    "/tmp/soffit-cost-refresh.lock"
}

pub fn cost_daily() -> &'static str {
    "/tmp/soffit-cost-daily"
}

pub fn cost_session(sid: &str) -> String {
    format!("/tmp/soffit-cost-{sid}")
}

pub fn self_version_cache() -> &'static str {
    "/tmp/soffit-self-version"
}

pub fn self_version_lock() -> &'static str {
    "/tmp/soffit-self-version.lock"
}
