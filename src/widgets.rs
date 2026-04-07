use std::collections::HashMap;
use std::time::UNIX_EPOCH;

use crate::config::StatuslineConfig;
use crate::paths;
use crate::theme::{ansi, BarStyle, RESET};
use crate::types::{SessionSnapshot, StdinData, WidgetConfig};

fn terminal_width() -> u16 {
    #[cfg(unix)]
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
            return ws.ws_col;
        }
    }
    120
}

pub struct WidgetContext {
    pub data: StdinData,
    pub pct: u32,
    pub input_tokens: u64,
    pub compact_size: Option<u64>,
    pub terminal_width: u16,
    pub bar_style: BarStyle,
    pub use_unicode_text: bool,
    pub palette: crate::theme::ThemePalette,
}

pub fn build_context(data: StdinData, config: &StatuslineConfig) -> WidgetContext {
    let autocompact_pct: u32 = std::env::var("CLAUDE_AUTOCOMPACT_PCT_OVERRIDE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100u32);

    let ctx_window = data.context_window.as_ref();
    let raw_pct = ctx_window.and_then(|c| c.used_percentage).unwrap_or(0.0) as u32;
    let ctx_size = ctx_window.and_then(|c| c.context_window_size);
    let usage = ctx_window.and_then(|c| c.current_usage.as_ref());

    let input_tokens = usage
        .map(|u| {
            u.input_tokens.unwrap_or(0)
                + u.cache_creation_input_tokens.unwrap_or(0)
                + u.cache_read_input_tokens.unwrap_or(0)
        })
        .unwrap_or(0);

    let compact_size = if autocompact_pct < 100 {
        ctx_size.map(|s| s * autocompact_pct as u64 / 100)
    } else {
        ctx_size
    };

    let pct = if autocompact_pct > 0 {
        (raw_pct * 100 / autocompact_pct).min(100)
    } else {
        raw_pct
    };

    let snapshot_model = data
        .model
        .as_ref()
        .map(|m| m.display_name.clone())
        .unwrap_or_default();
    let snapshot_cwd = data
        .workspace
        .as_ref()
        .and_then(|w| w.current_dir.clone())
        .unwrap_or_default();

    if let Some(sid) = data.session_id.as_deref().filter(|s| !s.is_empty()) {
        let _ = std::fs::write(paths::context_pct_file(sid), pct.to_string());
        let updated_at = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let snap = SessionSnapshot {
            session_id: sid.to_string(),
            model: snapshot_model,
            context_pct: pct,
            cwd: snapshot_cwd,
            updated_at,
        };
        if let Ok(bytes) = serde_json::to_vec(&snap) {
            let path_str = paths::session_snapshot(sid);
            let path = std::path::Path::new(&path_str);
            let tmp = format!("{path_str}.tmp");
            if std::fs::write(&tmp, &bytes).is_ok() {
                let _ = std::fs::rename(&tmp, path);
            }
        }
    }

    let terminal_width = terminal_width();

    WidgetContext {
        data,
        pct,
        input_tokens,
        compact_size,
        terminal_width,
        bar_style: config.bar_style.clone(),
        use_unicode_text: config.use_unicode_text,
        palette: config.palette.clone(),
    }
}

// --- Compositor ---

fn dispatch_widget(
    name: &str,
    ctx: &WidgetContext,
    widget_configs: &HashMap<String, WidgetConfig>,
) -> Option<String> {
    let cfg = widget_configs.get(name);
    let compact = cfg.map(|c| c.compact).unwrap_or(false);
    let empty: Vec<String> = vec![];
    let components: &Vec<String> = cfg.map(|c| &c.components).unwrap_or(&empty);

    // Resolve widget metadata from TOML sidecar
    let wmeta = crate::plugin::widget_meta(name);

    // Merge per-widget palette overrides
    let mut widget_palette = ctx.palette.clone();
    if let (Some(ref wmeta), Some(c)) = (&wmeta, cfg) {
        if let Some(ref theme_map) = c.theme {
            for slot in &wmeta.theme_slots {
                if let Some(tv) = theme_map.get(&slot.key) {
                    let idx = match tv {
                        crate::types::ThemeValue::Custom(n) => *n,
                        crate::types::ThemeValue::Role(r) => ctx.palette.resolve(*r),
                    };
                    if let Some(role) = slot.palette_role {
                        widget_palette.set_role(role, idx);
                    }
                }
            }
        }
    }

    // Resolve icons
    let widget_icons: serde_json::Value = {
        let user_icons = cfg.and_then(|c| c.icons.as_ref());
        let mut map = serde_json::Map::new();
        if let Some(ref wmeta) = wmeta {
            for slot in &wmeta.icon_slots {
                let val = user_icons
                    .and_then(|m| m.get(&slot.key))
                    .cloned()
                    .unwrap_or_else(|| slot.default_value.clone());
                map.insert(slot.key.clone(), serde_json::Value::String(val));
            }
        }
        serde_json::Value::Object(map)
    };

    // Resolve settings
    let widget_settings: serde_json::Value = {
        let user_settings = cfg.and_then(|c| c.settings.as_ref());
        let mut map = serde_json::Map::new();
        if let Some(ref wmeta) = wmeta {
            for slot in &wmeta.setting_slots {
                let val = user_settings
                    .and_then(|m| m.get(&slot.key))
                    .cloned()
                    .unwrap_or_else(|| slot.default_value.clone());
                map.insert(slot.key.clone(), val);
            }
        }
        if let Some(user) = user_settings {
            for (k, v) in user {
                if !map.contains_key(k) {
                    map.insert(k.clone(), v.clone());
                }
            }
        }
        serde_json::Value::Object(map)
    };

    let effective_bar_style = cfg
        .and_then(|c| c.bar_style.clone())
        .unwrap_or_else(|| ctx.bar_style.clone());

    let widget_input = serde_json::json!({
        "data": ctx.data,
        "_soffit": {
            "pct": ctx.pct,
            "input_tokens": ctx.input_tokens,
            "compact_size": ctx.compact_size,
            "use_unicode_text": ctx.use_unicode_text,
            "terminal_width": ctx.terminal_width,
        },
        "config": {
            "compact": compact,
            "components": components,
            "palette": {
                "primary": ansi(widget_palette.primary),
                "accent": ansi(widget_palette.accent),
                "success": ansi(widget_palette.success),
                "warning": ansi(widget_palette.warning),
                "danger": ansi(widget_palette.danger),
                "muted": ansi(widget_palette.muted),
                "subtle": ansi(widget_palette.subtle),
                "reset": RESET,
                "dim_success": "\x1b[38;5;65m",
                "dim_warning": "\x1b[38;5;130m",
                "dim_danger": "\x1b[38;5;131m",
                "dim_primary": "\x1b[38;5;67m",
            },
            "icons": widget_icons,
            "settings": widget_settings,
            "bar_style": effective_bar_style.to_string(),
        }
    });
    crate::plugin::run_widget(name, &widget_input.to_string())
}

pub fn render_line_parts(
    widget_names: &[String],
    ctx: &WidgetContext,
    widget_configs: &HashMap<String, WidgetConfig>,
) -> Vec<String> {
    std::thread::scope(|s| {
        let handles: Vec<_> = widget_names
            .iter()
            .map(|name| s.spawn(|| dispatch_widget(name, ctx, widget_configs)))
            .collect();
        handles
            .into_iter()
            .filter_map(|h| h.join().ok().flatten())
            .collect()
    })
}

pub fn render_line(
    widget_names: &[String],
    ctx: &WidgetContext,
    separator: &str,
    widget_configs: &HashMap<String, WidgetConfig>,
) -> String {
    render_line_parts(widget_names, ctx, widget_configs).join(separator)
}

pub fn render(name: &str) -> anyhow::Result<()> {
    use anyhow::bail;

    let custom_widgets = crate::plugin::list_custom_widgets();
    if name == "list" {
        for p in &custom_widgets {
            println!("{p}");
        }
        return Ok(());
    }
    if !custom_widgets.iter().any(|p| p == name) {
        bail!("unknown widget '{name}'");
    }
    let config = StatuslineConfig::load()?;
    let data: StdinData = serde_json::from_reader(std::io::stdin()).unwrap_or_default();
    let ctx = build_context(data, &config);
    let output = render_line(&[name.to_string()], &ctx, "", &config.widgets);
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn make_ctx(data: StdinData) -> WidgetContext {
        let config = StatuslineConfig::default();
        build_context(data, &config)
    }

    #[test]
    fn build_context_defaults_with_empty_stdin() {
        let config = StatuslineConfig::default();
        let ctx = build_context(StdinData::default(), &config);
        assert_eq!(ctx.pct, 0);
        assert_eq!(ctx.input_tokens, 0);
        assert!(ctx.compact_size.is_none());
    }

    #[test]
    fn render_line_filters_empty_widgets() {
        let ctx = make_ctx(StdinData::default());
        let names = vec!["vim".into(), "agent".into()];
        let result = render_line(&names, &ctx, " | ", &HashMap::new());
        assert!(result.is_empty());
    }
}
