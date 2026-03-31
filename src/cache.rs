use std::time::SystemTime;

pub fn cache_age_secs(path: &str) -> Option<f64> {
    let mtime = std::fs::metadata(path).ok()?.modified().ok()?;
    let age = SystemTime::now().duration_since(mtime).ok()?;
    Some(age.as_secs_f64())
}

pub fn read_stale(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

pub fn write_cache(path: &str, value: &str) {
    let _ = std::fs::write(path, value);
}

pub fn needs_refresh(path: &str, ttl_secs: f64) -> bool {
    cache_age_secs(path)
        .map(|age| age >= ttl_secs)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_stale_missing_file_returns_none() {
        assert!(read_stale("/tmp/claude-sl-test-nonexistent-9f8e7d").is_none());
    }

    #[test]
    fn write_and_read_round_trip() {
        let path = "/tmp/claude-sl-test-cache-roundtrip";
        write_cache(path, "hello");
        assert_eq!(read_stale(path).unwrap(), "hello");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn cache_age_secs_fresh_file() {
        let path = "/tmp/claude-sl-test-cache-age";
        write_cache(path, "x");
        let age = cache_age_secs(path).unwrap();
        assert!(age < 2.0, "just-written file should be <2s old, got {age}");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn cache_age_secs_missing_file() {
        assert!(cache_age_secs("/tmp/claude-sl-test-nonexistent-age-abc").is_none());
    }

    #[test]
    fn needs_refresh_missing_file() {
        assert!(needs_refresh(
            "/tmp/claude-sl-test-nonexistent-refresh",
            60.0
        ));
    }

    #[test]
    fn needs_refresh_fresh_file() {
        let path = "/tmp/claude-sl-test-needs-refresh-fresh";
        write_cache(path, "x");
        assert!(!needs_refresh(path, 60.0));
        let _ = std::fs::remove_file(path);
    }
}
