use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::theme::{BarStyle, IconsConfig, ThemeConfig};
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
    pub theme: ThemeConfig,
    pub icons: IconsConfig,
    pub bar_style: BarStyle,
    pub use_unicode_text: bool,
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
            theme: ThemeConfig::default(),
            icons: IconsConfig::default(),
            bar_style: BarStyle::default(),
            use_unicode_text: true,
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

        let theme = extract_theme_config(&table);
        let icons = extract_icons_config(&table);
        let bar_style = table
            .get("bar_style")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();

        let use_unicode_text = table
            .get("use_unicode_text")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        Ok(Self {
            line1,
            line2,
            line3,
            separator,
            widgets,
            autocompact_pct,
            cost_target_weekly,
            theme,
            icons,
            bar_style,
            use_unicode_text,
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

        table.insert(
            "autocompact_pct".to_string(),
            toml::Value::Integer(i64::from(self.autocompact_pct)),
        );
        table.insert(
            "cost_target_weekly".to_string(),
            toml::Value::Float(self.cost_target_weekly),
        );

        let mut theme_table = toml::Table::new();
        for (key, val) in theme_color_entries(&self.theme) {
            if let Some(v) = val {
                theme_table.insert(key.to_string(), toml::Value::Integer(i64::from(v)));
            }
        }
        if !theme_table.is_empty() {
            table.insert(
                "statusline_theme".to_string(),
                toml::Value::Table(theme_table),
            );
        } else {
            table.remove("statusline_theme");
        }

        let mut icons_table = toml::Table::new();
        for (key, val) in icons_string_entries(&self.icons) {
            if let Some(v) = val {
                icons_table.insert(key.to_string(), toml::Value::String(v));
            }
        }
        if !icons_table.is_empty() {
            table.insert(
                "statusline_icons".to_string(),
                toml::Value::Table(icons_table),
            );
        } else {
            table.remove("statusline_icons");
        }

        if self.bar_style != BarStyle::default() {
            table.insert(
                "bar_style".to_string(),
                toml::Value::String(self.bar_style.to_string()),
            );
        } else {
            table.remove("bar_style");
        }

        if !self.use_unicode_text {
            table.insert("use_unicode_text".to_string(), toml::Value::Boolean(false));
        } else {
            table.remove("use_unicode_text");
        }

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

fn extract_theme_config(table: &toml::Table) -> ThemeConfig {
    let t = match table.get("statusline_theme").and_then(|v| v.as_table()) {
        Some(t) => t,
        None => return ThemeConfig::default(),
    };
    let c = |key: &str| -> Option<u8> { t.get(key).and_then(|v| v.as_integer()).map(|v| v as u8) };
    ThemeConfig {
        green: c("green"),
        orange: c("orange"),
        red: c("red"),
        dim: c("dim"),
        lgray: c("lgray"),
        cyan: c("cyan"),
        purple: c("purple"),
        yellow: c("yellow"),
        dim_green: c("dim_green"),
        dim_yellow: c("dim_yellow"),
        dim_orange: c("dim_orange"),
        dim_red: c("dim_red"),
        dim_cyan: c("dim_cyan"),
        dim_pink: c("dim_pink"),
    }
}

fn extract_icons_config(table: &toml::Table) -> IconsConfig {
    let t = match table.get("statusline_icons").and_then(|v| v.as_table()) {
        Some(t) => t,
        None => return IconsConfig::default(),
    };
    let s = |key: &str| -> Option<String> { t.get(key).and_then(|v| v.as_str()).map(String::from) };
    let ch = |key: &str| -> Option<char> {
        t.get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| s.chars().next())
    };
    IconsConfig {
        duration: s("duration"),
        cost: s("cost"),
        git_branch: s("git_branch"),
        git_staged: s("git_staged"),
        agent: s("agent"),
        update: s("update"),
        bar_fill: ch("bar_fill"),
        bar_empty: ch("bar_empty"),
        bar_half: ch("bar_half"),
        quota_fill: ch("quota_fill"),
        quota_empty: ch("quota_empty"),
        quota_pace: ch("quota_pace"),
    }
}

fn theme_color_entries(theme: &ThemeConfig) -> [(&'static str, Option<u8>); 14] {
    [
        ("green", theme.green),
        ("orange", theme.orange),
        ("red", theme.red),
        ("dim", theme.dim),
        ("lgray", theme.lgray),
        ("cyan", theme.cyan),
        ("purple", theme.purple),
        ("yellow", theme.yellow),
        ("dim_green", theme.dim_green),
        ("dim_yellow", theme.dim_yellow),
        ("dim_orange", theme.dim_orange),
        ("dim_red", theme.dim_red),
        ("dim_cyan", theme.dim_cyan),
        ("dim_pink", theme.dim_pink),
    ]
}

fn icons_string_entries(icons: &IconsConfig) -> [(&'static str, Option<String>); 12] {
    [
        ("duration", icons.duration.clone()),
        ("cost", icons.cost.clone()),
        ("git_branch", icons.git_branch.clone()),
        ("git_staged", icons.git_staged.clone()),
        ("agent", icons.agent.clone()),
        ("update", icons.update.clone()),
        ("bar_fill", icons.bar_fill.map(|c| c.to_string())),
        ("bar_empty", icons.bar_empty.map(|c| c.to_string())),
        ("bar_half", icons.bar_half.map(|c| c.to_string())),
        ("quota_fill", icons.quota_fill.map(|c| c.to_string())),
        ("quota_empty", icons.quota_empty.map(|c| c.to_string())),
        ("quota_pace", icons.quota_pace.map(|c| c.to_string())),
    ]
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

    #[test]
    fn theme_round_trip() -> Result<()> {
        let mut f = NamedTempFile::new()?;
        writeln!(f, "[statusline_theme]\ngreen = 82")?;
        let config = load_from_path(f.path())?;
        assert_eq!(config.theme.green, Some(82));
        assert_eq!(config.theme.orange, None);
        save_to_path(&config, f.path())?;
        let reloaded = load_from_path(f.path())?;
        assert_eq!(reloaded.theme.green, Some(82));
        assert_eq!(reloaded.theme.orange, None);
        Ok(())
    }

    #[test]
    fn icons_round_trip() -> Result<()> {
        let mut f = NamedTempFile::new()?;
        writeln!(f, "[statusline_icons]\ncost = \"$\"")?;
        let config = load_from_path(f.path())?;
        assert_eq!(config.icons.cost.as_deref(), Some("$"));
        assert_eq!(config.icons.duration, None);
        save_to_path(&config, f.path())?;
        let reloaded = load_from_path(f.path())?;
        assert_eq!(reloaded.icons.cost.as_deref(), Some("$"));
        assert_eq!(reloaded.icons.duration, None);
        Ok(())
    }

    #[test]
    fn cost_target_weekly_persists() -> Result<()> {
        let mut f = NamedTempFile::new()?;
        writeln!(f, "cost_target_weekly = 150.0")?;
        let config = load_from_path(f.path())?;
        assert!((config.cost_target_weekly - 150.0).abs() < f64::EPSILON);
        save_to_path(&config, f.path())?;
        let reloaded = load_from_path(f.path())?;
        assert!((reloaded.cost_target_weekly - 150.0).abs() < f64::EPSILON);
        Ok(())
    }

    #[test]
    fn bar_style_round_trip() -> Result<()> {
        let mut f = NamedTempFile::new()?;
        writeln!(f, "bar_style = \"dot\"")?;
        let config = load_from_path(f.path())?;
        assert_eq!(config.bar_style, BarStyle::Dot);
        save_to_path(&config, f.path())?;
        let reloaded = load_from_path(f.path())?;
        assert_eq!(reloaded.bar_style, BarStyle::Dot);
        Ok(())
    }

    #[test]
    fn default_bar_style_not_written() -> Result<()> {
        let f = NamedTempFile::new()?;
        let config = StatuslineConfig::default();
        save_to_path(&config, f.path())?;
        let raw = fs::read_to_string(f.path())?;
        assert!(
            !raw.contains("bar_style"),
            "Default bar_style should not be written: {raw}"
        );
        assert!(
            !raw.contains("statusline_theme"),
            "Empty theme should not be written: {raw}"
        );
        assert!(
            !raw.contains("statusline_icons"),
            "Empty icons should not be written: {raw}"
        );
        assert!(
            !raw.contains("use_unicode_text"),
            "Default use_unicode_text should not be written: {raw}"
        );
        Ok(())
    }

    #[test]
    fn use_unicode_text_round_trip() -> Result<()> {
        let mut f = NamedTempFile::new()?;
        writeln!(f, "use_unicode_text = false")?;
        let config = load_from_path(f.path())?;
        assert!(!config.use_unicode_text);
        save_to_path(&config, f.path())?;
        let reloaded = load_from_path(f.path())?;
        assert!(!reloaded.use_unicode_text);

        // Default (true) should not be written to file
        let mut config2 = StatuslineConfig::default();
        config2.use_unicode_text = true;
        save_to_path(&config2, f.path())?;
        let raw = fs::read_to_string(f.path())?;
        assert!(
            !raw.contains("use_unicode_text"),
            "Default use_unicode_text=true should not be written: {raw}"
        );
        Ok(())
    }
}
