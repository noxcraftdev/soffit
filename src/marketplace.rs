use std::fs;

use anyhow::{bail, Result};

use crate::paths;

pub const DEFAULT_SOURCE_NAME: &str = "default";
pub const DEFAULT_SOURCE_REPO: &str = "noxcraftdev/soffit-widgets";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketplaceSource {
    pub name: String,
    pub repo: String,
}

pub struct MarketplaceSources(Vec<MarketplaceSource>);

impl MarketplaceSources {
    pub fn load() -> Result<Self> {
        let path = paths::marketplace_config();
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Ok(Self::default_sources()),
        };
        if raw.trim().is_empty() {
            return Ok(Self::default_sources());
        }
        Self::parse(&raw)
    }

    fn parse(raw: &str) -> Result<Self> {
        let table: toml::Table = toml::from_str(raw)?;
        let sources = match table.get("sources").and_then(|v| v.as_array()) {
            Some(arr) => arr
                .iter()
                .filter_map(|entry| {
                    let t = entry.as_table()?;
                    let name = t.get("name")?.as_str()?.to_string();
                    let repo = t.get("repo")?.as_str()?.to_string();
                    Some(MarketplaceSource { name, repo })
                })
                .collect(),
            None => vec![],
        };
        Ok(MarketplaceSources(sources))
    }

    fn default_sources() -> Self {
        MarketplaceSources(vec![MarketplaceSource {
            name: DEFAULT_SOURCE_NAME.to_string(),
            repo: DEFAULT_SOURCE_REPO.to_string(),
        }])
    }

    pub fn save(&self) -> Result<()> {
        let path = paths::marketplace_config();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut table = toml::Table::new();
        let sources_arr: Vec<toml::Value> = self
            .0
            .iter()
            .map(|s| {
                let mut entry = toml::Table::new();
                entry.insert("name".to_string(), toml::Value::String(s.name.clone()));
                entry.insert("repo".to_string(), toml::Value::String(s.repo.clone()));
                toml::Value::Table(entry)
            })
            .collect();
        table.insert("sources".to_string(), toml::Value::Array(sources_arr));

        let out = toml::to_string_pretty(&table)?;

        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, &out)?;
        fs::rename(&tmp, &path)?;

        Ok(())
    }

    pub fn add(&mut self, name: &str, repo: &str) -> Result<()> {
        if self.0.iter().any(|s| s.name == name) {
            bail!("source '{}' already exists", name);
        }
        validate_repo_format(repo)?;
        self.0.push(MarketplaceSource {
            name: name.to_string(),
            repo: repo.to_string(),
        });
        Ok(())
    }

    pub fn remove(&mut self, name: &str, allow_remove_default: bool) -> Result<()> {
        let pos = self
            .0
            .iter()
            .position(|s| s.name == name)
            .ok_or_else(|| anyhow::anyhow!("source '{}' not found", name))?;
        if name == DEFAULT_SOURCE_NAME && !allow_remove_default {
            bail!("cannot remove default source without --force");
        }
        self.0.remove(pos);
        Ok(())
    }

    pub fn list(&self) -> &[MarketplaceSource] {
        &self.0
    }

    pub fn get_by_name(&self, name: &str) -> Option<&MarketplaceSource> {
        self.0.iter().find(|s| s.name == name)
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub file: String,
}

#[derive(serde::Deserialize)]
struct RegistryRoot {
    #[serde(alias = "plugins")]
    widgets: Vec<RegistryEntry>,
    #[serde(default)]
    #[allow(dead_code)]
    defaults: Vec<String>,
}

pub(crate) fn fetch_registry(repo: &str) -> anyhow::Result<Vec<RegistryEntry>> {
    let (owner, repo_name) = split_owner_repo(repo)?;

    let cache_path = crate::paths::marketplace_registry_cache(owner, repo_name);

    if !crate::cache::needs_refresh(&cache_path, 3600.0) {
        if let Some(cached) = crate::cache::read_stale(&cache_path) {
            let root: RegistryRoot = serde_json::from_str(&cached)?;
            return Ok(root.widgets);
        }
    }

    let url = crate::install::raw_url(owner, repo_name, "registry.json");
    let bytes = crate::http::curl_fetch(&url)?;
    let root: RegistryRoot = serde_json::from_slice(&bytes)?;
    crate::cache::write_cache(&cache_path, &String::from_utf8_lossy(&bytes));
    Ok(root.widgets)
}

pub fn resolve_and_install(name: &str, force: bool) -> anyhow::Result<()> {
    let sources = MarketplaceSources::load()?;
    for source in sources.list() {
        let entries = match fetch_registry(&source.repo) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("warning: could not reach source '{}': {}", source.name, e);
                continue;
            }
        };
        if let Some(entry) = entries.iter().find(|e| e.name == name) {
            let repo = if entry.repo.is_empty() {
                &source.repo
            } else {
                &entry.repo
            };
            let (owner, repo_name) = split_owner_repo(repo).map_err(|_| {
                anyhow::anyhow!("invalid repo '{}' in registry entry for '{}'", repo, name)
            })?;
            let file = if entry.file.is_empty() {
                format!("{name}.sh")
            } else {
                if entry.file.contains("..") {
                    anyhow::bail!("invalid file path in registry entry for '{}'", name);
                }
                entry.file.clone()
            };
            let ext = if file.ends_with(".py") { "py" } else { "sh" };
            let script_url = crate::install::raw_url(owner, repo_name, &file);
            let script_bytes = crate::http::curl_fetch(&script_url)?;
            let toml_file = {
                let stem = file.trim_end_matches(".sh").trim_end_matches(".py");
                format!("{stem}.toml")
            };
            let toml_url = crate::install::raw_url(owner, repo_name, &toml_file);
            let toml_opt = crate::http::curl_fetch(&toml_url).ok();
            let install_dir = crate::paths::widgets_dir();
            crate::install::install_one_in(
                &install_dir,
                name,
                ext,
                &script_bytes,
                toml_opt,
                force,
            )?;
            println!("installed {name}");
            return Ok(());
        }
    }
    anyhow::bail!(
        "widget '{name}' not found in any registered source. Use 'soffit marketplace list' to see sources or 'soffit install owner/repo/name' to install directly."
    )
}

/// Fetch the default source's registry and install any widgets listed in `defaults`
/// that are not already installed.
pub fn install_defaults() -> anyhow::Result<()> {
    let widgets_dir = crate::paths::widgets_dir();
    let cache_path = {
        let (owner, repo_name) = split_owner_repo(DEFAULT_SOURCE_REPO)?;
        crate::paths::marketplace_registry_cache(owner, repo_name)
    };

    let entries = fetch_registry(DEFAULT_SOURCE_REPO)?;

    let defaults: Vec<String> = crate::cache::read_stale(&cache_path)
        .and_then(|cached| {
            let root: RegistryRoot = serde_json::from_str(&cached).ok()?;
            Some(root.defaults)
        })
        .unwrap_or_default();

    for name in &defaults {
        if crate::widget::widget_source_path(name).is_some() {
            continue;
        }
        let Some(entry) = entries.iter().find(|e| &e.name == name) else {
            continue;
        };
        let repo = if entry.repo.is_empty() {
            DEFAULT_SOURCE_REPO
        } else {
            &entry.repo
        };
        let Ok((owner, repo_name)) = split_owner_repo(repo) else {
            continue;
        };
        let file = if entry.file.is_empty() {
            format!("{name}.sh")
        } else {
            entry.file.clone()
        };
        let ext = if file.ends_with(".py") { "py" } else { "sh" };
        let Ok(script_bytes) =
            crate::http::curl_fetch(&crate::install::raw_url(owner, repo_name, &file))
        else {
            continue;
        };
        let toml_file = {
            let stem = file.trim_end_matches(".sh").trim_end_matches(".py");
            format!("{stem}.toml")
        };
        let toml_url = crate::install::raw_url(owner, repo_name, &toml_file);
        let toml_opt = crate::http::curl_fetch(&toml_url).ok();
        let _ =
            crate::install::install_one_in(&widgets_dir, name, ext, &script_bytes, toml_opt, false);
    }
    Ok(())
}

#[derive(clap::Subcommand)]
pub enum MarketplaceCmd {
    /// Add a named widget source (owner/repo)
    Add { name: String, repo: String },
    /// List all registered widget sources
    List {
        /// Show widget counts (fetches or uses cached registry)
        #[arg(short = 'v', long)]
        verbose: bool,
    },
    /// Remove a named widget source
    Remove {
        name: String,
        #[arg(long)]
        force: bool,
    },
    /// Refresh the registry cache for one or all sources
    Update {
        #[arg(long)]
        source: Option<String>,
    },
}

pub fn run(cmd: MarketplaceCmd) -> anyhow::Result<()> {
    match cmd {
        MarketplaceCmd::Add { name, repo } => {
            let mut sources = MarketplaceSources::load()?;
            sources.add(&name, &repo)?;
            sources.save()?;
            println!("added source '{name}' -> {repo}");
            Ok(())
        }
        MarketplaceCmd::List { verbose } => {
            let sources = MarketplaceSources::load()?;
            if !verbose {
                for s in sources.list() {
                    println!("{}  {}", s.name, s.repo);
                }
                return Ok(());
            }
            for source in sources.list() {
                let (owner, repo_name) = match split_owner_repo(&source.repo) {
                    Ok(pair) => pair,
                    Err(_) => {
                        eprintln!(
                            "warning: skipping source '{}': invalid repo format",
                            source.name
                        );
                        continue;
                    }
                };
                let cache_path = crate::paths::marketplace_registry_cache(owner, repo_name);
                let is_cached = !crate::cache::needs_refresh(&cache_path, 3600.0);
                let result = fetch_registry(&source.repo);
                let (count, status) = match result {
                    Ok(entries) => (entries.len(), if is_cached { "cached" } else { "live" }),
                    Err(_) => (0usize, "unavailable"),
                };
                println!(
                    "{:<12} {:<40} {} widgets  [{}]",
                    source.name, source.repo, count, status
                );
            }
            Ok(())
        }
        MarketplaceCmd::Remove { name, force } => {
            let mut sources = MarketplaceSources::load()?;
            sources.remove(&name, force)?;
            sources.save()?;
            println!("removed source '{name}'");
            Ok(())
        }
        MarketplaceCmd::Update { source } => {
            let sources = MarketplaceSources::load()?;
            let targets: Vec<&MarketplaceSource> = match &source {
                Some(name) => {
                    let s = sources
                        .get_by_name(name)
                        .ok_or_else(|| anyhow::anyhow!("source '{name}' not found"))?;
                    vec![s]
                }
                None => sources.list().iter().collect(),
            };

            for s in targets {
                // Force-fresh: remove the cache file so needs_refresh returns true.
                if let Ok((owner, repo_name)) = split_owner_repo(&s.repo) {
                    let cache_path = crate::paths::marketplace_registry_cache(owner, repo_name);
                    let _ = std::fs::remove_file(&cache_path);
                }

                match fetch_registry(&s.repo) {
                    Ok(entries) => {
                        println!("updated {} ({} widgets)", s.name, entries.len());
                    }
                    Err(e) => {
                        eprintln!("warning: could not refresh '{}': {e}", s.name);
                    }
                }
            }
            Ok(())
        }
    }
}

pub fn split_owner_repo(repo: &str) -> anyhow::Result<(&str, &str)> {
    let mut parts = repo.splitn(2, '/');
    match (parts.next(), parts.next()) {
        (Some(owner), Some(repo_name)) if !owner.is_empty() && !repo_name.is_empty() => {
            Ok((owner, repo_name))
        }
        _ => anyhow::bail!("invalid repo '{repo}': expected owner/repo"),
    }
}

fn validate_repo_format(repo: &str) -> Result<()> {
    // Must be exactly "owner/repo" — one slash, no empty segments, no spaces.
    if repo.contains(' ') {
        bail!("invalid repo format: expected 'owner/repo'");
    }
    let parts: Vec<&str> = repo.splitn(3, '/').collect();
    match parts.as_slice() {
        [owner, r] if !owner.is_empty() && !r.is_empty() && !r.contains('/') => Ok(()),
        _ => bail!("invalid repo format: expected 'owner/repo'"),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    // Redirect marketplace_config() by writing raw TOML and parsing directly.
    // For load/save round-trip tests we bypass paths:: and test the parse/save
    // logic directly using a temp path.

    fn save_to(sources: &MarketplaceSources, path: &std::path::Path) -> Result<()> {
        let mut table = toml::Table::new();
        let arr: Vec<toml::Value> = sources
            .0
            .iter()
            .map(|s| {
                let mut entry = toml::Table::new();
                entry.insert("name".to_string(), toml::Value::String(s.name.clone()));
                entry.insert("repo".to_string(), toml::Value::String(s.repo.clone()));
                toml::Value::Table(entry)
            })
            .collect();
        table.insert("sources".to_string(), toml::Value::Array(arr));
        let out = toml::to_string_pretty(&table)?;
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, &out)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    fn load_from(path: &std::path::Path) -> Result<MarketplaceSources> {
        match fs::read_to_string(path) {
            Ok(s) if !s.trim().is_empty() => MarketplaceSources::parse(&s),
            _ => Ok(MarketplaceSources::default_sources()),
        }
    }

    #[test]
    fn load_missing_file_yields_default() {
        let sources = load_from(std::path::Path::new("/nonexistent/marketplace.toml")).unwrap();
        assert_eq!(sources.list().len(), 1);
        let s = &sources.list()[0];
        assert_eq!(s.name, DEFAULT_SOURCE_NAME);
        assert_eq!(s.repo, DEFAULT_SOURCE_REPO);
    }

    #[test]
    fn save_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("marketplace.toml");

        let mut sources = MarketplaceSources::default_sources();
        sources.add("community", "acme/soffit-widgets").unwrap();
        save_to(&sources, &path).unwrap();

        let reloaded = load_from(&path).unwrap();
        assert_eq!(reloaded.list().len(), 2);
        assert_eq!(reloaded.list()[0].name, "default");
        assert_eq!(reloaded.list()[0].repo, DEFAULT_SOURCE_REPO);
        assert_eq!(reloaded.list()[1].name, "community");
        assert_eq!(reloaded.list()[1].repo, "acme/soffit-widgets");
    }

    #[test]
    fn add_duplicate_name_fails() {
        let mut sources = MarketplaceSources::default_sources();
        let err = sources.add(DEFAULT_SOURCE_NAME, "other/repo").unwrap_err();
        assert!(
            err.to_string().contains("already exists"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn add_bad_format_fails() {
        let mut sources = MarketplaceSources::default_sources();
        let err = sources.add("test", "notaslug").unwrap_err();
        assert!(
            err.to_string().contains("invalid"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn remove_missing_name_fails() {
        let mut sources = MarketplaceSources::default_sources();
        let err = sources.remove("nonexistent", false).unwrap_err();
        assert!(
            err.to_string().contains("not found"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn remove_default_without_force_fails() {
        let mut sources = MarketplaceSources::default_sources();
        let err = sources.remove(DEFAULT_SOURCE_NAME, false).unwrap_err();
        assert!(
            err.to_string().contains("cannot remove default source"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn remove_default_with_force_succeeds() {
        let mut sources = MarketplaceSources::default_sources();
        sources.remove(DEFAULT_SOURCE_NAME, true).unwrap();
        assert!(sources.list().is_empty());
    }

    #[test]
    fn validate_repo_rejects_no_slash() {
        let mut s = MarketplaceSources::default_sources();
        assert!(s.add("x", "noslash").is_err());
    }

    #[test]
    fn validate_repo_rejects_leading_slash() {
        let mut s = MarketplaceSources::default_sources();
        assert!(s.add("x", "/repo").is_err());
    }

    #[test]
    fn validate_repo_rejects_trailing_slash() {
        let mut s = MarketplaceSources::default_sources();
        assert!(s.add("x", "owner/").is_err());
    }

    #[test]
    fn validate_repo_rejects_three_segments() {
        let mut s = MarketplaceSources::default_sources();
        assert!(s.add("x", "a/b/c").is_err());
    }

    #[test]
    fn validate_repo_rejects_spaces() {
        let mut s = MarketplaceSources::default_sources();
        assert!(s.add("x", "owner /repo").is_err());
    }

    #[test]
    fn validate_repo_accepts_valid() {
        let mut s = MarketplaceSources::default_sources();
        assert!(s.add("extra", "alice/my-widgets").is_ok());
    }

    #[test]
    fn get_by_name_returns_correct() {
        let sources = MarketplaceSources::default_sources();
        let found = sources.get_by_name(DEFAULT_SOURCE_NAME).unwrap();
        assert_eq!(found.repo, DEFAULT_SOURCE_REPO);
        assert!(sources.get_by_name("missing").is_none());
    }
}
