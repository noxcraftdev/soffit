use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::theme::{BarStyle, ThemePalette};
use crate::types::WidgetConfig;

fn config_path() -> PathBuf {
    crate::paths::config_dir().join("config.toml")
}

#[derive(Debug, Clone)]
pub struct StatuslineConfig {
    pub line1: Vec<String>,
    pub line2: Vec<String>,
    pub line3: Vec<String>,
    pub widgets: HashMap<String, WidgetConfig>,
    pub bar_style: BarStyle,
    pub use_unicode_text: bool,
    pub palette: ThemePalette,
    pub editor_font: Option<String>,
    pub editor_width: Option<f64>,
    pub editor_height: Option<f64>,
    pub weekly_budget: Option<f64>,
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
            widgets: HashMap::new(),
            bar_style: BarStyle::default(),
            use_unicode_text: true,
            palette: ThemePalette::default(),
            editor_font: None,
            editor_width: None,
            editor_height: None,
            weekly_budget: None,
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
        let widgets = extract_widget_configs(&table).unwrap_or_default();

        let bar_style = table
            .get("bar_style")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();

        let use_unicode_text = table
            .get("use_unicode_text")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let palette = table
            .get("palette")
            .and_then(|v| v.clone().try_into::<ThemePalette>().ok())
            .unwrap_or_default();

        // Ensure every widget in the layout has at least a default config entry
        let mut widgets = widgets;
        for name in line1.iter().chain(line2.iter()).chain(line3.iter()) {
            widgets.entry(name.clone()).or_default();
        }

        Ok(Self {
            line1,
            line2,
            line3,
            widgets,
            bar_style,
            use_unicode_text,
            palette,
            editor_font: table
                .get("editor_font")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            editor_width: table.get("editor_width").and_then(|v| v.as_float()),
            editor_height: table.get("editor_height").and_then(|v| v.as_float()),
            weekly_budget: table.get("weekly_budget").and_then(|v| v.as_float()),
        })
    }

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

        let mut widgets_table = toml::Table::new();
        for (name, wc) in &self.widgets {
            let mut wc_table = toml::Table::new();
            wc_table.insert("compact".to_string(), toml::Value::Boolean(wc.compact));
            wc_table.insert(
                "components".to_string(),
                toml::Value::try_from(wc.components.clone())?,
            );
            if let Some(ref theme) = wc.theme {
                wc_table.insert("theme".to_string(), toml::Value::try_from(theme.clone())?);
            }
            if let Some(ref icons) = wc.icons {
                wc_table.insert("icons".to_string(), toml::Value::try_from(icons.clone())?);
            }
            if let Some(ref bar_style) = wc.bar_style {
                wc_table.insert(
                    "bar_style".to_string(),
                    toml::Value::try_from(bar_style.clone())?,
                );
            }
            widgets_table.insert(name.to_string(), toml::Value::Table(wc_table));
        }
        table.insert(
            "statusline_widgets".to_string(),
            toml::Value::Table(widgets_table),
        );

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

        if let Some(ref font) = self.editor_font {
            table.insert("editor_font".to_string(), toml::Value::String(font.clone()));
        } else {
            table.remove("editor_font");
        }

        if self.palette != ThemePalette::default() {
            table.insert(
                "palette".to_string(),
                toml::Value::try_from(self.palette.clone())?,
            );
        } else {
            table.remove("palette");
        }

        if let Some(w) = self.editor_width {
            table.insert("editor_width".to_string(), toml::Value::Float(w));
        } else {
            table.remove("editor_width");
        }

        if let Some(h) = self.editor_height {
            table.insert("editor_height".to_string(), toml::Value::Float(h));
        } else {
            table.remove("editor_height");
        }

        if let Some(b) = self.weekly_budget {
            table.insert("weekly_budget".to_string(), toml::Value::Float(b));
        } else {
            table.remove("weekly_budget");
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

    #[test]
    fn per_widget_colors_round_trip() -> Result<()> {
        let mut f = NamedTempFile::new()?;
        writeln!(
            f,
            "[statusline_widgets.cost]\ncompact = true\ncomponents = [\"session\"]\n\n[statusline_widgets.cost.theme]\nwithin_budget = 46\nover_budget = 196"
        )?;
        let config = load_from_path(f.path())?;
        let cost_cfg = config.widgets.get("cost").expect("cost widget config");
        assert!(cost_cfg.compact);
        use crate::types::ThemeValue;
        let colors = cost_cfg.theme.as_ref().expect("per-widget colors");
        assert_eq!(colors.get("within_budget"), Some(&ThemeValue::Custom(46)));
        assert_eq!(colors.get("over_budget"), Some(&ThemeValue::Custom(196)));
        assert_eq!(colors.get("approaching"), None);
        save_to_path(&config, f.path())?;
        let reloaded = load_from_path(f.path())?;
        let colors2 = reloaded
            .widgets
            .get("cost")
            .and_then(|c| c.theme.as_ref())
            .expect("per-widget colors after round-trip");
        assert_eq!(colors2.get("within_budget"), Some(&ThemeValue::Custom(46)));
        assert_eq!(colors2.get("over_budget"), Some(&ThemeValue::Custom(196)));
        Ok(())
    }

    #[test]
    fn per_widget_icons_round_trip() -> Result<()> {
        let mut f = NamedTempFile::new()?;
        writeln!(
            f,
            "[statusline_widgets.cost]\ncompact = false\n\n[statusline_widgets.cost.icons]\ncost = \"$$$\""
        )?;
        let config = load_from_path(f.path())?;
        let icons = config
            .widgets
            .get("cost")
            .and_then(|c| c.icons.as_ref())
            .expect("per-widget icons");
        assert_eq!(icons.get("cost").map(|s| s.as_str()), Some("$$$"));
        save_to_path(&config, f.path())?;
        let reloaded = load_from_path(f.path())?;
        let icons2 = reloaded
            .widgets
            .get("cost")
            .and_then(|c| c.icons.as_ref())
            .expect("per-widget icons");
        assert_eq!(icons2.get("cost").map(|s| s.as_str()), Some("$$$"));
        Ok(())
    }

    #[test]
    fn widget_without_overrides_has_none() -> Result<()> {
        let mut f = NamedTempFile::new()?;
        writeln!(
            f,
            "[statusline_widgets.git]\ncompact = true\ncomponents = [\"branch\"]"
        )?;
        let config = load_from_path(f.path())?;
        let git_cfg = config.widgets.get("git").expect("git widget config");
        assert!(git_cfg.theme.is_none());
        assert!(git_cfg.icons.is_none());
        assert!(git_cfg.bar_style.is_none());
        Ok(())
    }
}
