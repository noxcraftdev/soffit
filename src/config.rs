use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::types::WidgetConfig;

fn config_path() -> PathBuf {
    crate::paths::config_dir().join("config.toml")
}

#[derive(Debug, Clone)]
pub struct StatuslineConfig {
    pub line1: Vec<String>,
    pub line2: Vec<String>,
    pub line3: Vec<String>,
    pub separator: String,
    pub widgets: HashMap<String, WidgetConfig>,
    pub autocompact_pct: u32,
    pub cost_target_weekly: f64,
}

impl Default for StatuslineConfig {
    fn default() -> Self {
        Self {
            line1: vec![
                "vim".into(),
                "agent".into(),
                "version".into(),
                "context_bar".into(),
                "quota".into(),
                "duration".into(),
                "cost".into(),
            ],
            line2: vec!["git".into()],
            line3: vec![],
            separator: format!(" {}|{} ", "\x1b[38;5;242m", "\x1b[0m"),
            widgets: HashMap::new(),
            autocompact_pct: 100,
            cost_target_weekly: 300.0,
        }
    }
}

impl StatuslineConfig {
    pub fn load() -> Result<Self> {
        let path = config_path();
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Ok(Self::default()),
        };
        Self::load_from_str(&raw)
    }

    fn load_from_str(raw: &str) -> Result<Self> {
        let table: toml::Table = toml::from_str(raw)?;
        let defaults = Self::default();

        let line1 = extract_string_vec(&table, "statusline_line1").unwrap_or(defaults.line1);
        let line2 = extract_string_vec(&table, "statusline_line2").unwrap_or(defaults.line2);
        let line3 = extract_string_vec(&table, "statusline_line3").unwrap_or(defaults.line3);
        let separator = table
            .get("statusline_separator")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or(defaults.separator);
        let widgets = extract_widget_configs(&table).unwrap_or_default();

        let autocompact_pct = table
            .get("autocompact_pct")
            .and_then(|v| v.as_integer())
            .map(|v| v as u32)
            .unwrap_or(defaults.autocompact_pct);
        let cost_target_weekly = table
            .get("cost_target_weekly")
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
            .unwrap_or(defaults.cost_target_weekly);

        Ok(Self {
            line1,
            line2,
            line3,
            separator,
            widgets,
            autocompact_pct,
            cost_target_weekly,
        })
    }

    #[allow(dead_code)] // Used by editor (Phase 5) and tests
    pub fn save(&self) -> Result<()> {
        let path = config_path();
        let raw = fs::read_to_string(&path).unwrap_or_default();
        let mut table: toml::Table = toml::from_str(&raw).unwrap_or_default();

        self.apply_to_table(&mut table)?;

        let out = toml::to_string_pretty(&table)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Atomic write: write to temp file then rename
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, &out)?;
        fs::rename(&tmp, &path)?;

        Ok(())
    }

    #[allow(dead_code)] // Used by save() and tests
    fn apply_to_table(&self, table: &mut toml::Table) -> Result<()> {
        table.insert(
            "statusline_line1".to_string(),
            toml::Value::try_from(self.line1.clone())?,
        );
        table.insert(
            "statusline_line2".to_string(),
            toml::Value::try_from(self.line2.clone())?,
        );
        table.insert(
            "statusline_line3".to_string(),
            toml::Value::try_from(self.line3.clone())?,
        );
        table.insert(
            "statusline_separator".to_string(),
            toml::Value::String(self.separator.clone()),
        );

        let mut widgets_table = toml::Table::new();
        for (name, wc) in &self.widgets {
            let mut wc_table = toml::Table::new();
            wc_table.insert("compact".to_string(), toml::Value::Boolean(wc.compact));
            wc_table.insert(
                "components".to_string(),
                toml::Value::try_from(wc.components.clone())?,
            );
            widgets_table.insert(name.to_string(), toml::Value::Table(wc_table));
        }
        table.insert(
            "statusline_widgets".to_string(),
            toml::Value::Table(widgets_table),
        );

        Ok(())
    }
}

fn extract_string_vec(table: &toml::Table, key: &str) -> Option<Vec<String>> {
    table.get(key).and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    })
}

fn extract_widget_configs(table: &toml::Table) -> Option<HashMap<String, WidgetConfig>> {
    let widgets_table = table.get("statusline_widgets")?.as_table()?;
    let mut result = HashMap::new();
    for (name, val) in widgets_table {
        if let Ok(wc) = val.clone().try_into::<WidgetConfig>() {
            result.insert(name.clone(), wc);
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn load_from_path(path: &std::path::Path) -> Result<StatuslineConfig> {
        match fs::read_to_string(path) {
            Ok(raw) => StatuslineConfig::load_from_str(&raw),
            Err(_) => Ok(StatuslineConfig::default()),
        }
    }

    fn save_to_path(config: &StatuslineConfig, path: &std::path::Path) -> Result<()> {
        let raw = fs::read_to_string(path).unwrap_or_default();
        let mut table: toml::Table = toml::from_str(&raw).unwrap_or_default();
        config.apply_to_table(&mut table)?;
        fs::write(path, toml::to_string_pretty(&table)?)?;
        Ok(())
    }

    #[test]
    fn round_trip() -> Result<()> {
        let mut f = NamedTempFile::new()?;
        writeln!(f, "statusline_line1 = [\"version\"]")?;
        let mut config = load_from_path(f.path())?;
        assert_eq!(config.line1, vec!["version"]);
        config.line1 = vec!["git".to_string()];
        save_to_path(&config, f.path())?;
        let reloaded = load_from_path(f.path())?;
        assert_eq!(reloaded.line1, vec!["git"]);
        Ok(())
    }

    #[test]
    fn non_statusline_keys_preserved() -> Result<()> {
        let mut f = NamedTempFile::new()?;
        writeln!(f, "lookback_days = 42\nstatusline_line1 = [\"version\"]")?;
        let config = load_from_path(f.path())?;
        save_to_path(&config, f.path())?;
        let raw = fs::read_to_string(f.path())?;
        assert!(
            raw.contains("lookback_days = 42"),
            "Non-statusline key was lost: {}",
            raw
        );
        Ok(())
    }

    #[test]
    fn empty_file_uses_defaults() -> Result<()> {
        let config = load_from_path(std::path::Path::new("/nonexistent/path/config.toml"))?;
        assert_eq!(
            config.line1,
            vec![
                "vim",
                "agent",
                "version",
                "context_bar",
                "quota",
                "duration",
                "cost"
            ]
        );
        assert_eq!(config.line2, vec!["git"]);
        assert!(config.line3.is_empty());
        Ok(())
    }
}
