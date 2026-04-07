use std::collections::HashMap;

use anyhow::Result;

use crate::config::StatuslineConfig;
use crate::theme::ThemePalette;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Preset {
    pub description: String,
    #[serde(default)]
    pub line1: Vec<String>,
    #[serde(default)]
    pub line2: Vec<String>,
    #[serde(default)]
    pub line3: Vec<String>,
    #[serde(default)]
    pub palette: Option<ThemePalette>,
}

#[derive(serde::Deserialize)]
struct RegistryWithPresets {
    #[serde(default)]
    presets: HashMap<String, Preset>,
}

pub fn list() -> Result<Vec<(String, Preset)>> {
    let cache_path = {
        let (owner, repo) =
            crate::marketplace::split_owner_repo(crate::marketplace::DEFAULT_SOURCE_REPO)?;
        crate::paths::marketplace_registry_cache(owner, repo)
    };

    // Try fetching fresh, fall back to cache
    let _ = crate::marketplace::fetch_registry(crate::marketplace::DEFAULT_SOURCE_REPO);

    let json = crate::cache::read_stale(&cache_path)
        .ok_or_else(|| anyhow::anyhow!("no registry cache available"))?;

    let root: RegistryWithPresets = serde_json::from_str(&json)?;
    let mut presets: Vec<(String, Preset)> = root.presets.into_iter().collect();
    presets.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(presets)
}

pub fn apply(name: &str) -> Result<()> {
    let presets = list()?;
    let (_, preset) = presets
        .iter()
        .find(|(n, _)| n == name)
        .ok_or_else(|| anyhow::anyhow!("preset '{name}' not found"))?;

    let mut config = StatuslineConfig::load()?;

    if !preset.line1.is_empty() {
        config.line1 = preset.line1.clone();
    }
    if !preset.line2.is_empty() {
        config.line2 = preset.line2.clone();
    }
    // line3: always apply (empty means clear it)
    config.line3 = preset.line3.clone();
    if let Some(ref palette) = preset.palette {
        config.palette = palette.clone();
    }

    // Install any widgets referenced by the preset that aren't installed yet
    let all_widgets: Vec<&String> = config
        .line1
        .iter()
        .chain(config.line2.iter())
        .chain(config.line3.iter())
        .collect();

    for widget_name in &all_widgets {
        if crate::plugin::widget_source_path(widget_name).is_none() {
            let _ = crate::marketplace::resolve_and_install(widget_name, false);
        }
    }

    config.save()?;
    println!("applied preset '{name}'");
    Ok(())
}

#[derive(clap::Subcommand)]
pub enum PresetCmd {
    /// List available presets
    List,
    /// Apply a preset (layout + palette)
    Apply {
        /// Preset name
        name: String,
    },
}

pub fn run(cmd: PresetCmd) -> Result<()> {
    match cmd {
        PresetCmd::List => {
            let presets = list()?;
            if presets.is_empty() {
                println!("no presets available");
                return Ok(());
            }
            for (name, preset) in &presets {
                println!("{:<16} {}", name, preset.description);
            }
            Ok(())
        }
        PresetCmd::Apply { name } => apply(&name),
    }
}
