pub mod widget_reference;

use anyhow::Result;
#[cfg(feature = "desktop")]
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

use crate::config::StatuslineConfig;
use crate::plugin::{self, PluginMeta};
use crate::theme::{
    ansi_256_to_hex, BarStyle, PaletteRole, ThemePalette, CURATED_COLORS, PALETTE_ROLES,
    THEME_PRESETS,
};
use crate::types::ThemeValue;
use widget_reference::{component_desc, widget_ref, WIDGETS};

// ---- entry point -----------------------------------------------------------

const CUSTOM_HEAD_CSS: &str = r#"html, body { margin:0; padding:0; overflow:hidden; background:#1e1e2e; }
body.dnd-active, body.dnd-active * { cursor: grabbing !important; user-select: none !important; -webkit-user-select: none !important; }
body.dnd-active select, body.dnd-active input, body.dnd-active textarea, body.dnd-active button { pointer-events: none !important; }
.dnd-src { opacity: 0.3 !important; }
.dnd-target { border-left: 3px solid #89b4fa !important; }
.dnd-row-target { border-color: #89b4fa !important; background: #1e1e3e !important; }
.dnd-drop-zone { width:64px; height:28px; border-radius:4px; flex-shrink:0; }
body.dnd-active .dnd-drop-zone { background: rgba(137,180,250,0.1); border: 1px dashed rgba(137,180,250,0.3); }"#;

const CUSTOM_HEAD_JS: &str = r#"(function() {
  // ---- Line chip drag/drop ----
  var lineSrc = null;
  document.addEventListener('mousedown', function(e) {
    var chip = e.target.closest('[data-dnd]');
    if (!chip || e.target.closest('[data-no-drag]')) return;
    e.preventDefault();
    lineSrc = chip.getAttribute('data-dnd');
    chip.classList.add('dnd-src');
    document.body.classList.add('dnd-active');
  });
  document.addEventListener('mousemove', function(e) {
    if (!lineSrc) return;
    e.preventDefault();
    document.querySelectorAll('.dnd-target').forEach(function(el) { el.classList.remove('dnd-target'); });
    document.querySelectorAll('.dnd-row-target').forEach(function(el) { el.classList.remove('dnd-row-target'); });
    var chip = e.target.closest('[data-dnd]');
    if (chip && chip.getAttribute('data-dnd') !== lineSrc) {
      chip.classList.add('dnd-target');
    } else {
      var row = e.target.closest('[data-drop-line]');
      if (row) row.classList.add('dnd-row-target');
    }
  });
  document.addEventListener('mouseup', function(e) {
    if (!lineSrc) return;
    var target = null;
    var chip = e.target.closest('[data-dnd]');
    if (chip) {
      var chipId = chip.getAttribute('data-dnd');
      if (chipId !== lineSrc) target = chipId;
    } else {
      var row = e.target.closest('[data-drop-line]');
      if (row) target = 'append:' + row.getAttribute('data-drop-line');
    }
    document.querySelectorAll('.dnd-src,.dnd-target').forEach(function(el) { el.classList.remove('dnd-src', 'dnd-target'); });
    document.querySelectorAll('.dnd-row-target').forEach(function(el) { el.classList.remove('dnd-row-target'); });
    document.body.classList.remove('dnd-active');
    if (target) window.__dndResult = lineSrc + '>' + target;
    lineSrc = null;
  });

  // ---- Component chip drag/drop ----
  var compSrc = null;
  document.addEventListener('mousedown', function(e) {
    var chip = e.target.closest('[data-comp-dnd]');
    if (!chip || e.target.closest('[data-no-drag]')) return;
    e.preventDefault();
    e.stopPropagation();
    compSrc = chip.getAttribute('data-comp-dnd');
    chip.classList.add('dnd-src');
    document.body.classList.add('dnd-active');
  }, true);
  document.addEventListener('mousemove', function(e) {
    if (!compSrc) return;
    e.preventDefault();
    document.querySelectorAll('[data-comp-dnd].dnd-target').forEach(function(el) { el.classList.remove('dnd-target'); });
    document.querySelectorAll('.dnd-comp-row-target').forEach(function(el) { el.classList.remove('dnd-comp-row-target'); });
    var chip = e.target.closest('[data-comp-dnd]');
    if (chip && chip.getAttribute('data-comp-dnd') !== compSrc) {
      var srcWidget = compSrc.split(':')[0];
      var tgtWidget = chip.getAttribute('data-comp-dnd').split(':')[0];
      if (srcWidget === tgtWidget) chip.classList.add('dnd-target');
    } else {
      var zone = e.target.closest('[data-comp-drop]');
      if (zone) zone.classList.add('dnd-comp-row-target');
    }
  }, true);
  document.addEventListener('mouseup', function(e) {
    if (!compSrc) return;
    var target = null;
    var chip = e.target.closest('[data-comp-dnd]');
    if (chip && chip.getAttribute('data-comp-dnd') !== compSrc) {
      var srcWidget = compSrc.split(':')[0];
      var tgtWidget = chip.getAttribute('data-comp-dnd').split(':')[0];
      if (srcWidget === tgtWidget) target = chip.getAttribute('data-comp-dnd');
    } else {
      var zone = e.target.closest('[data-comp-drop]');
      if (zone) {
        var srcWidget = compSrc.split(':')[0];
        var zoneWidget = zone.getAttribute('data-comp-drop');
        if (srcWidget === zoneWidget) target = srcWidget + ':append';
      }
    }
    document.querySelectorAll('[data-comp-dnd].dnd-src,[data-comp-dnd].dnd-target').forEach(function(el) { el.classList.remove('dnd-src', 'dnd-target'); });
    document.querySelectorAll('.dnd-comp-row-target').forEach(function(el) { el.classList.remove('dnd-comp-row-target'); });
    document.body.classList.remove('dnd-active');
    if (target) window.__compDndResult = compSrc + '>' + target;
    compSrc = null;
  }, true);
})();"#;

#[cfg(feature = "desktop")]
fn load_icon() -> Option<dioxus::desktop::tao::window::Icon> {
    let bytes = include_bytes!("../../assets/icon.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    dioxus::desktop::tao::window::Icon::from_rgba(img.into_raw(), w, h).ok()
}

#[cfg(feature = "desktop")]
pub fn run() -> Result<()> {
    let cfg = StatuslineConfig::load().unwrap_or_default();
    let width = cfg.editor_width.unwrap_or(1100.0);
    let height = cfg.editor_height.unwrap_or(530.0);

    let mut window = WindowBuilder::new()
        .with_title("Soffit")
        .with_decorations(true)
        .with_resizable(true)
        .with_inner_size(LogicalSize::new(width, height));
    if let Some(icon) = load_icon() {
        window = window.with_window_icon(Some(icon));
    }

    dioxus::LaunchBuilder::new()
        .with_cfg(
            Config::default()
                .with_custom_head(format!(
                    "<style>{}</style><script>{}</script>",
                    CUSTOM_HEAD_CSS, CUSTOM_HEAD_JS
                ))
                .with_menu(None)
                .with_window(window),
        )
        .launch(App);
    Ok(())
}

// ---- types -----------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tab {
    Lines,
    Widgets,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DragState {
    src_line: usize,
    src_idx: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DropTarget {
    Before(usize, usize),
    Append(usize),
}

// ---- helpers ---------------------------------------------------------------

fn effective_components(config: &StatuslineConfig, name: &str) -> Vec<String> {
    if let Some(wc) = config.widgets.get(name) {
        if !wc.components.is_empty() {
            return wc.components.clone();
        }
    }
    widget_ref(name)
        .map(|w| w.default_components.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

fn autosave(config: &Signal<StatuslineConfig>) {
    if let Err(e) = config.read().save() {
        eprintln!("save failed: {e}");
    }
}

fn get_line(cfg: &StatuslineConfig, idx: usize) -> &Vec<String> {
    match idx {
        0 => &cfg.line1,
        1 => &cfg.line2,
        _ => &cfg.line3,
    }
}

fn get_line_mut(cfg: &mut StatuslineConfig, idx: usize) -> &mut Vec<String> {
    match idx {
        0 => &mut cfg.line1,
        1 => &mut cfg.line2,
        _ => &mut cfg.line3,
    }
}

fn perform_move(config: &mut StatuslineConfig, drag: DragState, target: DropTarget) {
    let src = get_line_mut(config, drag.src_line);
    if drag.src_idx >= src.len() {
        return;
    }
    let item = src.remove(drag.src_idx);

    match target {
        DropTarget::Append(line) => {
            get_line_mut(config, line).push(item);
        }
        DropTarget::Before(line, idx) => {
            let adjusted = if line == drag.src_line && drag.src_idx < idx {
                idx.saturating_sub(1)
            } else {
                idx
            };
            let dst = get_line_mut(config, line);
            let at = adjusted.min(dst.len());
            dst.insert(at, item);
        }
    }
}

fn parse_line_drop(s: &str) -> Option<(DragState, DropTarget)> {
    let parts: Vec<&str> = s.split('>').collect();
    if parts.len() != 2 {
        return None;
    }
    let src: Vec<usize> = parts[0].split(',').filter_map(|x| x.parse().ok()).collect();
    if src.len() != 2 {
        return None;
    }
    let drag = DragState {
        src_line: src[0],
        src_idx: src[1],
    };
    if let Some(line_str) = parts[1].strip_prefix("append:") {
        let line: usize = line_str.parse().ok()?;
        Some((drag, DropTarget::Append(line)))
    } else {
        let dst: Vec<usize> = parts[1].split(',').filter_map(|x| x.parse().ok()).collect();
        if dst.len() != 2 {
            return None;
        }
        Some((drag, DropTarget::Before(dst[0], dst[1])))
    }
}

enum CompDrop {
    Before(String, usize, usize),
    Append(String, usize),
}

fn parse_comp_drop(s: &str) -> Option<CompDrop> {
    // Format: "widget:srcIdx>widget:dstIdx" or "widget:srcIdx>widget:append"
    let parts: Vec<&str> = s.split('>').collect();
    if parts.len() != 2 {
        return None;
    }
    let src: Vec<&str> = parts[0].splitn(2, ':').collect();
    let dst: Vec<&str> = parts[1].splitn(2, ':').collect();
    if src.len() != 2 || dst.len() != 2 {
        return None;
    }
    if src[0] != dst[0] {
        return None;
    }
    let src_idx: usize = src[1].parse().ok()?;
    if dst[1] == "append" {
        Some(CompDrop::Append(src[0].to_string(), src_idx))
    } else {
        let dst_idx: usize = dst[1].parse().ok()?;
        Some(CompDrop::Before(src[0].to_string(), src_idx, dst_idx))
    }
}

// ---- preview ---------------------------------------------------------------

fn resolve_preview_icon<'a>(
    icons: &'a std::collections::HashMap<String, String>,
    key: &str,
    default: &'a str,
) -> &'a str {
    icons.get(key).map(|s| s.as_str()).unwrap_or(default)
}

fn bar_style_chars(bar_style: &crate::theme::BarStyle) -> (char, char) {
    match bar_style {
        crate::theme::BarStyle::Block => ('■', '□'),
        crate::theme::BarStyle::Dot => ('●', '○'),
        crate::theme::BarStyle::Ascii => ('#', '-'),
    }
}

fn widget_preview(
    name: &str,
    compact: bool,
    components: &[String],
    config: &StatuslineConfig,
) -> Element {
    use crate::theme::ansi_256_to_hex;

    // Build effective palette: start from global palette, apply per-widget color slot overrides.
    let mut pal = config.palette.clone();
    if let Some(wref) = widget_ref(name) {
        let per_widget_theme = config.widgets.get(name).and_then(|wc| wc.theme.as_ref());
        for slot in wref.color_slots {
            let idx = config.palette.resolve(slot.palette_role);
            pal.set_role(slot.palette_role, idx);
        }
        if let Some(theme_map) = per_widget_theme {
            for slot in wref.color_slots {
                if let Some(tv) = theme_map.get(slot.key) {
                    let idx = match tv {
                        ThemeValue::Custom(n) => *n,
                        ThemeValue::Role(r) => config.palette.resolve(*r),
                    };
                    pal.set_role(slot.palette_role, idx);
                }
            }
        }
    }

    let widget_icons: std::collections::HashMap<String, String> = config
        .widgets
        .get(name)
        .and_then(|wc| wc.icons.as_ref())
        .cloned()
        .unwrap_or_default();

    let widget_bar_style = config
        .widgets
        .get(name)
        .and_then(|wc| wc.bar_style.clone())
        .unwrap_or_else(|| config.bar_style.clone());

    let dim_s = ansi_256_to_hex(pal.muted);
    let blue_s = ansi_256_to_hex(pal.primary);
    let green_s = ansi_256_to_hex(pal.success);
    let orange_s = ansi_256_to_hex(pal.warning);
    let red_s = ansi_256_to_hex(pal.danger);
    let purple_s = ansi_256_to_hex(pal.accent);
    let yellow_s = ansi_256_to_hex(pal.subtle);
    let dim = dim_s.as_str();
    let blue = blue_s.as_str();
    let green = green_s.as_str();
    let orange = orange_s.as_str();
    let red = red_s.as_str();
    let purple = purple_s.as_str();
    let yellow = yellow_s.as_str();

    match name {
        "insights" => {
            let all: &[(&str, &str, &str, &str)] = &[
                ("strategies", purple, "🔭2", "🔭 2 strategies"),
                ("priorities", red, "🎯1", "🎯 1 priority"),
                ("insights", orange, "💡3", "💡 3 insights"),
                ("notes", green, "📋4", "📋 4 notes"),
                ("pending", yellow, "⏳2", "⏳ 2 pending"),
            ];
            let parts: Vec<(&str, &str, &str, &str)> = components
                .iter()
                .filter_map(|c| all.iter().find(|(k, _, _, _)| *k == c.as_str()).copied())
                .collect();
            if parts.is_empty() {
                return rsx! { span { style: "color:{dim}", "—" } };
            }
            rsx! {
                span {
                    for (i, (_, color, compact_txt, verbose_txt)) in parts.into_iter().enumerate() {
                        if i > 0 {
                            if compact { " " } else { span { style: "color:{dim}", " | " } }
                        }
                        span { style: "color:{color}",
                            if compact { "{compact_txt}" } else { "{verbose_txt}" }
                        }
                    }
                    if !compact { span { style: "color:{dim}", " /brief" } }
                }
            }
        }
        "cost" => {
            let cost_icon = resolve_preview_icon(&widget_icons, "cost", "💸 ");
            let all: &[(&str, &str, &str)] = &[
                ("session", dim, "$0.42"),
                ("today", green, "$1.80"),
                ("week", green, "$18.50"),
            ];
            let parts: Vec<(&str, &str)> = components
                .iter()
                .filter_map(|c| {
                    all.iter()
                        .find(|(k, _, _)| *k == c.as_str())
                        .map(|(_, col, val)| (*col, *val))
                })
                .collect();
            if parts.is_empty() {
                return rsx! { span { style: "color:{dim}", "—" } };
            }
            rsx! {
                span {
                    if !compact { "{cost_icon}" }
                    for (i, (color, val)) in parts.into_iter().enumerate() {
                        if i > 0 {
                            if compact { " " } else { span { style: "color:{dim}", " | " } }
                        }
                        span { style: "color:{color}", "{val}" }
                    }
                }
            }
        }
        "version" => {
            let update_icon = resolve_preview_icon(&widget_icons, "update", "↑ ").to_string();
            let use_uni = config.use_unicode_text;
            let ver_v = if use_uni {
                crate::fmt::superscript("1.2.16")
            } else {
                "1.2.16".into()
            };
            let model_v = if use_uni {
                crate::fmt::subscript("claude-sonnet-4-6")
            } else {
                "claude-sonnet-4-6".into()
            };
            let parts: Vec<(&str, String)> = components
                .iter()
                .filter_map(|c| match c.as_str() {
                    "update" => Some((orange, update_icon.clone())),
                    "version" => Some((
                        dim,
                        if compact {
                            "1.2.16".into()
                        } else {
                            ver_v.clone()
                        },
                    )),
                    "model" => Some((
                        purple,
                        if compact {
                            "sonnet".into()
                        } else {
                            model_v.clone()
                        },
                    )),
                    _ => None,
                })
                .filter(|(_, txt)| !txt.is_empty())
                .collect();
            rsx! {
                span {
                    for (i, (color, txt)) in parts.into_iter().enumerate() {
                        if i > 0 { " " }
                        span { style: "color:{color}", "{txt}" }
                    }
                }
            }
        }
        "context_bar" => {
            let (bar_fill, bar_empty) = bar_style_chars(&widget_bar_style);
            let bar_str = format!(
                "{}{}{}{}{}{}{}{}",
                bar_fill, bar_fill, bar_fill, bar_fill, bar_empty, bar_empty, bar_empty, bar_empty
            );
            let all: &[(&str, bool)] = &[("bar", false), ("pct", false), ("tokens", true)];
            let parts: Vec<(&str, String)> = components
                .iter()
                .filter_map(|c| {
                    all.iter()
                        .find(|(k, _)| *k == c.as_str())
                        .filter(|(_, compact_hide)| !compact || !compact_hide)
                        .map(|(k, _)| {
                            let (col, txt) = match *k {
                                "bar" => (green, bar_str.clone()),
                                "pct" => (green, "🯴🯲٪".to_string()),
                                "tokens" => (dim, "42k/100k".to_string()),
                                _ => unreachable!(),
                            };
                            (col, txt)
                        })
                })
                .collect();
            rsx! {
                span {
                    for (i, (color, txt)) in parts.into_iter().enumerate() {
                        if i > 0 { " " }
                        span { style: "color:{color}", "{txt}" }
                    }
                }
            }
        }
        "git" => {
            let branch_icon = resolve_preview_icon(&widget_icons, "branch", "⎇ ").to_string();
            let staged_icon = resolve_preview_icon(&widget_icons, "staged", "•").to_string();
            let branch_txt = format!("{branch_icon}main");
            let staged_txt = format!("{staged_icon}2");
            let all: &[(&str, bool)] = &[
                ("branch", false),
                ("staged", true),
                ("modified", true),
                ("repo", true),
            ];
            let parts: Vec<(&str, String)> = components
                .iter()
                .filter_map(|c| {
                    all.iter()
                        .find(|(k, _)| *k == c.as_str())
                        .filter(|(_, compact_hide)| !compact || !compact_hide)
                        .map(|(k, _)| {
                            let (col, txt) = match *k {
                                "branch" => (blue, branch_txt.clone()),
                                "staged" => (green, staged_txt.clone()),
                                "modified" => (orange, "~1".to_string()),
                                "repo" => (dim, "jarvis".to_string()),
                                _ => unreachable!(),
                            };
                            (col, txt)
                        })
                })
                .collect();
            rsx! {
                span {
                    for (i, (color, txt)) in parts.into_iter().enumerate() {
                        if i > 0 { " " }
                        span { style: "color:{color}", "{txt}" }
                    }
                }
            }
        }
        "quota" => {
            let (quota_fill, quota_empty) = bar_style_chars(&widget_bar_style);
            let five_hour_txt = format!(
                "5h:{}{}{}{} 60%",
                quota_fill, quota_fill, quota_empty, quota_empty
            );
            let seven_day_txt = format!(
                "7d:{}{}{}{} 75%",
                quota_fill, quota_fill, quota_fill, quota_empty
            );
            let all: &[(&str, &str)] = &[("five_hour", blue), ("seven_day", green)];
            let parts: Vec<(&str, String)> = components
                .iter()
                .filter_map(|c| {
                    all.iter().find(|(k, _)| *k == c.as_str()).map(|(k, col)| {
                        let txt = match *k {
                            "five_hour" => five_hour_txt.clone(),
                            "seven_day" => seven_day_txt.clone(),
                            _ => unreachable!(),
                        };
                        (*col, txt)
                    })
                })
                .collect();
            rsx! {
                span {
                    for (i, (color, txt)) in parts.into_iter().enumerate() {
                        if i > 0 { span { style: "color:{dim}", " | " } }
                        span { style: "color:{color}", "{txt}" }
                    }
                }
            }
        }
        "duration" => {
            let duration_icon = resolve_preview_icon(&widget_icons, "duration", "⏱ ").to_string();
            rsx! { span { if !compact { "{duration_icon}" } "1h23m" } }
        }
        "vim" => rsx! { span { style: "color:{purple}", if !compact { " " } "NORMAL" } },
        "agent" => {
            let agent_icon = resolve_preview_icon(&widget_icons, "agent", "❯ ").to_string();
            rsx! { span { style: "color:{orange}", if !compact { "{agent_icon}" } "worker-1" } }
        }
        "session" => rsx! { span { style: "color:{dim}", "a3f9" } },
        _ => {
            // Build palette-aware effective palette, then apply per-widget color overrides.
            let wmeta = crate::plugin::widget_meta(name);
            let mut plugin_palette = config.palette.clone();
            if let (Some(ref wmeta), Some(wc)) = (&wmeta, config.widgets.get(name)) {
                if let Some(ref theme_map) = wc.theme {
                    for slot in &wmeta.theme_slots {
                        if let Some(tv) = theme_map.get(&slot.key) {
                            let idx = match tv {
                                ThemeValue::Custom(n) => *n,
                                ThemeValue::Role(r) => config.palette.resolve(*r),
                            };
                            if let Some(role) = slot.palette_role {
                                plugin_palette.set_role(role, idx);
                            }
                        }
                    }
                }
            }
            let plugin_icons: serde_json::Value = {
                let user_icons = config.widgets.get(name).and_then(|wc| wc.icons.as_ref());
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
            let input = serde_json::json!({
                "data": {},
                "config": {
                    "compact": compact,
                    "components": components,
                    "palette": {
                        "primary": crate::theme::ansi(plugin_palette.primary),
                        "accent": crate::theme::ansi(plugin_palette.accent),
                        "success": crate::theme::ansi(plugin_palette.success),
                        "warning": crate::theme::ansi(plugin_palette.warning),
                        "danger": crate::theme::ansi(plugin_palette.danger),
                        "muted": crate::theme::ansi(plugin_palette.muted),
                        "subtle": crate::theme::ansi(plugin_palette.subtle),
                        "reset": crate::theme::RESET,
                    },
                    "icons": plugin_icons,
                    "bar_style": "block",
                }
            });
            match crate::plugin::run_plugin(name, &input.to_string()) {
                Some(text) => {
                    let html = crate::theme::ansi_to_html(&text).to_string();
                    rsx! { span { dangerous_inner_html: "{html}" } }
                }
                None => rsx! { span { style: "color:{orange}", "{name}" } },
            }
        }
    }
}

fn preview_line(widgets: &[String], config: &StatuslineConfig) -> Element {
    let dim_s = crate::theme::ansi_256_to_hex(242u8);
    let dim = dim_s.as_str();
    if widgets.is_empty() {
        return rsx! { span { style: "color:{dim}; font-style:italic;", "—" } };
    }
    rsx! {
        span {
            for (i, name) in widgets.iter().enumerate() {
                if i > 0 { span { style: "color:{dim}", " │ " } }
                {
                    let compact = config.widgets.get(name.as_str()).map(|c| c.compact).unwrap_or(false);
                    let comps = effective_components(config, name.as_str());
                    widget_preview(name.as_str(), compact, &comps, config)
                }
            }
        }
    }
}

// ---- root component --------------------------------------------------------

#[component]
pub fn App() -> Element {
    let config_init = StatuslineConfig::load().unwrap_or_default();
    let mut config = use_signal(|| config_init);
    let mut active_tab = use_signal(|| Tab::Lines);
    let plugin_metas = use_signal(plugin::list_plugin_metas);

    let tab = *active_tab.read();
    let cfg_snap = config.read().clone();
    let preview_font_family = match &cfg_snap.editor_font {
        Some(f) => format!("'{f}',monospace"),
        None => "'JetBrains Mono',Menlo,Consolas,monospace".to_string(),
    };

    // Poll JS for line-level drag/drop results
    use_coroutine::<(), _, _>(move |_rx| async move {
        loop {
            let eval = document::eval(
                r#"return new Promise(r => setTimeout(() => {
                    let v = window.__dndResult || '';
                    if (v) window.__dndResult = null;
                    r(v);
                }, 80));"#,
            );
            if let Ok(val) = eval.await {
                if let Some(s) = val.as_str() {
                    if let Some((drag, target)) = parse_line_drop(s) {
                        {
                            let mut cfg = config.write();
                            perform_move(&mut cfg, drag, target);
                        }
                        autosave(&config);
                    }
                }
            }
        }
    });

    // Poll JS for component-level drag/drop results
    use_coroutine::<(), _, _>(move |_rx| async move {
        loop {
            let eval = document::eval(
                r#"return new Promise(r => setTimeout(() => {
                    let v = window.__compDndResult || '';
                    if (v) window.__compDndResult = null;
                    r(v);
                }, 80));"#,
            );
            if let Ok(val) = eval.await {
                if let Some(s) = val.as_str() {
                    if let Some(drop) = parse_comp_drop(s) {
                        let (widget_name, src_idx) = match &drop {
                            CompDrop::Before(n, s, _) => (n.clone(), *s),
                            CompDrop::Append(n, s) => (n.clone(), *s),
                        };
                        {
                            let mut cfg = config.write();
                            let all_comps: Vec<String> = widget_ref(&widget_name)
                                .map(|w| {
                                    w.default_components.iter().map(|s| s.to_string()).collect()
                                })
                                .or_else(|| {
                                    plugin_metas
                                        .read()
                                        .iter()
                                        .find(|p| p.name == widget_name)
                                        .map(|p| p.components.clone())
                                })
                                .unwrap_or_default();
                            let wc = cfg.widgets.entry(widget_name).or_default();
                            if wc.components.is_empty() {
                                wc.components = all_comps;
                            }
                            if src_idx < wc.components.len() {
                                match drop {
                                    CompDrop::Before(_, _, dst_idx)
                                        if dst_idx < wc.components.len() =>
                                    {
                                        let item = wc.components.remove(src_idx);
                                        let at = if src_idx < dst_idx {
                                            dst_idx.saturating_sub(1)
                                        } else {
                                            dst_idx
                                        };
                                        wc.components.insert(at, item);
                                    }
                                    CompDrop::Append(_, _) => {
                                        let item = wc.components.remove(src_idx);
                                        wc.components.push(item);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        autosave(&config);
                    }
                }
            }
        }
    });

    rsx! {
        div {
            style: "display:flex; flex-direction:column; height:100vh; font-family:system-ui,sans-serif; font-size:14px; background:#1e1e2e; color:#cdd6f4; overflow:hidden;",

            // Preview
            div {
                style: "padding:12px 16px; flex-shrink:0; border-bottom:1px solid #313244;",
                div { style: "font-size:10px; color:#6c7086; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:6px;", "Preview" }
                div {
                    style: "background:#11111b; border:1px solid #313244; border-radius:6px; padding:10px 14px; font-family:{preview_font_family}; font-size:13px; line-height:1.6;",
                    div { {preview_line(&cfg_snap.line1, &cfg_snap)} }
                    if !cfg_snap.line2.is_empty() { div { {preview_line(&cfg_snap.line2, &cfg_snap)} } }
                    if !cfg_snap.line3.is_empty() { div { {preview_line(&cfg_snap.line3, &cfg_snap)} } }
                }
            }

            // Tabs
            div {
                style: "display:flex; border-bottom:1px solid #313244; background:#181825; flex-shrink:0;",
                TabButton { label: "Lines", active: tab == Tab::Lines, onclick: move |_| active_tab.set(Tab::Lines) }
                TabButton { label: "Widgets", active: tab == Tab::Widgets, onclick: move |_| active_tab.set(Tab::Widgets) }
                TabButton { label: "Settings", active: tab == Tab::Settings, onclick: move |_| active_tab.set(Tab::Settings) }
            }

            // Tab content (only scrolling area)
            div {
                style: "flex:1; min-height:0; overflow-y:auto; padding:16px;",
                match tab {
                    Tab::Lines => rsx! { LinesTab { config, plugin_metas } },
                    Tab::Widgets => rsx! { WidgetsTab { config, plugin_metas } },
                    Tab::Settings => rsx! { SettingsTab { config } },
                }
            }
        }
    }
}

#[component]
fn TabButton(label: &'static str, active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let style = if active {
        "padding:8px 16px; cursor:pointer; background:transparent; border:none; color:#89b4fa; border-bottom:2px solid #89b4fa; font-size:13px; font-weight:bold;"
    } else {
        "padding:8px 16px; cursor:pointer; background:transparent; border:none; color:#6c7086; border-bottom:2px solid transparent; font-size:13px;"
    };
    rsx! { button { style, onclick, "{label}" } }
}

// ---- settings tab ----------------------------------------------------------

#[component]
fn ColorInputRow(
    label: &'static str,
    value: Option<u8>,
    default_idx: u8,
    on_change: EventHandler<Option<u8>>,
) -> Element {
    let hex = crate::theme::ansi_256_to_hex(value.unwrap_or(default_idx));
    let display_val = value.map(|v| v.to_string()).unwrap_or_default();
    rsx! {
        div { style: "display:flex; align-items:center; gap:8px;",
            span { style: "color:#a6adc8; font-size:12px; min-width:80px;", "{label}" }
            input {
                r#type: "number",
                min: "0",
                max: "255",
                value: "{display_val}",
                style: "width:60px; background:#181825; color:#cdd6f4; border:1px solid #45475a; border-radius:3px; padding:2px 6px; font-size:12px;",
                oninput: move |evt| {
                    let raw = evt.value();
                    if raw.is_empty() {
                        on_change.call(None);
                    } else if let Ok(n) = raw.parse::<u8>() {
                        on_change.call(Some(n));
                    }
                }
            }
            span {
                style: "background-color:{hex}; width:16px; height:16px; border-radius:3px; display:inline-block; vertical-align:middle; flex-shrink:0; border:1px solid #45475a;",
            }
        }
    }
}

// ---- widget color picker ---------------------------------------------------

#[component]
fn WidgetColorPicker(
    label: String,
    value: Option<ThemeValue>,
    palette: ThemePalette,
    default_role: Option<PaletteRole>,
    on_change: EventHandler<Option<ThemeValue>>,
) -> Element {
    let mut custom_open = use_signal(|| false);

    // Resolve the effective ANSI index for the preview swatch.
    let effective_idx = match &value {
        Some(ThemeValue::Custom(n)) => *n,
        Some(ThemeValue::Role(r)) => palette.resolve(*r),
        None => match default_role {
            Some(r) => palette.resolve(r),
            None => 242,
        },
    };
    let preview_hex = ansi_256_to_hex(effective_idx);

    let has_override = value.is_some();

    rsx! {
        div { style: "margin-bottom:10px;",
            // Row 1: label + preview swatch + clear button
            div { style: "display:flex; align-items:center; gap:6px; margin-bottom:5px;",
                span { style: "color:#a6adc8; font-size:12px; min-width:90px;", "{label}" }
                div {
                    style: "width:16px; height:16px; border-radius:3px; background:{preview_hex}; border:1px solid #45475a; flex-shrink:0;",
                }
                if has_override {
                    button {
                        style: "background:none; border:none; color:#6c7086; font-size:12px; cursor:pointer; padding:0 2px; line-height:1;",
                        onclick: move |_| on_change.call(None),
                        "×"
                    }
                }
            }
            // Row 2: role swatches + custom toggle
            div { style: "display:flex; flex-wrap:wrap; gap:4px; align-items:flex-end;",
                for role in PALETTE_ROLES {
                    {
                        let role = *role;
                        let role_idx = palette.resolve(role);
                        let role_hex = ansi_256_to_hex(role_idx);
                        let is_active = matches!(&value, Some(ThemeValue::Role(r)) if *r == role);
                        let is_default_hint = value.is_none() && default_role == Some(role);
                        let border = if is_active {
                            "2px solid #ffffff".to_string()
                        } else if is_default_hint {
                            "2px dashed #6c7086".to_string()
                        } else {
                            "2px solid transparent".to_string()
                        };
                        rsx! {
                            div {
                                key: "{role.name()}",
                                style: "display:flex; flex-direction:column; align-items:center; gap:2px; cursor:pointer;",
                                onclick: move |_| on_change.call(Some(ThemeValue::Role(role))),
                                div {
                                    style: "width:20px; height:20px; border-radius:4px; background:{role_hex}; border:{border}; box-sizing:border-box;",
                                }
                                span { style: "font-size:8px; color:#6c7086;", "{role.label()}" }
                            }
                        }
                    }
                }
                // Custom toggle button
                {
                    let custom_idx_opt = match &value {
                        Some(ThemeValue::Custom(n)) => Some(*n),
                        _ => None,
                    };
                    let custom_hex = custom_idx_opt
                        .map(ansi_256_to_hex)
                        .unwrap_or_else(|| "#45475a".to_string());
                    let custom_border = if custom_idx_opt.is_some() {
                        "2px solid #ffffff"
                    } else {
                        "2px solid transparent"
                    };
                    rsx! {
                        div {
                            style: "display:flex; flex-direction:column; align-items:center; gap:2px; cursor:pointer;",
                            onclick: move |_| custom_open.set(!custom_open()),
                            div {
                                style: "width:20px; height:20px; border-radius:4px; background:{custom_hex}; border:{custom_border}; box-sizing:border-box; display:flex; align-items:center; justify-content:center;",
                                if custom_idx_opt.is_none() {
                                    span { style: "font-size:10px; color:#6c7086;", "+" }
                                }
                            }
                            span { style: "font-size:8px; color:#6c7086;", "Custom" }
                        }
                    }
                }
            }
            // Custom color grid (expanded inline)
            if custom_open() {
                div { style: "display:flex; flex-wrap:wrap; gap:3px; margin-top:6px;",
                    for &idx in CURATED_COLORS {
                        {
                            let hex = ansi_256_to_hex(idx);
                            let is_active = matches!(&value, Some(ThemeValue::Custom(n)) if *n == idx);
                            let border = if is_active {
                                "2px solid #ffffff"
                            } else {
                                "2px solid transparent"
                            };
                            rsx! {
                                div {
                                    key: "{idx}",
                                    style: "width:16px; height:16px; border-radius:2px; background:{hex}; cursor:pointer; border:{border}; box-sizing:border-box;",
                                    onclick: move |_| {
                                        on_change.call(Some(ThemeValue::Custom(idx)));
                                        custom_open.set(false);
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn IconInputRow(
    label: String,
    value: Option<String>,
    placeholder: String,
    on_change: EventHandler<Option<String>>,
) -> Element {
    let display_val = value.unwrap_or_default();
    rsx! {
        div { style: "display:flex; align-items:center; gap:8px;",
            span { style: "color:#a6adc8; font-size:12px; min-width:80px;", "{label}" }
            input {
                r#type: "text",
                value: "{display_val}",
                placeholder: "{placeholder}",
                style: "width:80px; background:#181825; color:#cdd6f4; border:1px solid #45475a; border-radius:3px; padding:2px 6px; font-size:12px;",
                oninput: move |evt| {
                    let raw = evt.value();
                    if raw.is_empty() {
                        on_change.call(None);
                    } else {
                        on_change.call(Some(raw));
                    }
                }
            }
        }
    }
}

#[component]
fn CharInputRow(
    label: String,
    value: Option<char>,
    placeholder: char,
    on_change: EventHandler<Option<char>>,
) -> Element {
    let display_val = value.map(|c| c.to_string()).unwrap_or_default();
    let placeholder_str = placeholder.to_string();
    rsx! {
        div { style: "display:flex; align-items:center; gap:8px;",
            span { style: "color:#a6adc8; font-size:12px; min-width:80px;", "{label}" }
            input {
                r#type: "text",
                value: "{display_val}",
                placeholder: "{placeholder_str}",
                style: "width:60px; background:#181825; color:#cdd6f4; border:1px solid #45475a; border-radius:3px; padding:2px 6px; font-size:12px;",
                oninput: move |evt| {
                    let raw = evt.value();
                    if raw.is_empty() {
                        on_change.call(None);
                    } else {
                        on_change.call(raw.chars().next());
                    }
                }
            }
        }
    }
}

#[component]
fn PaletteRoleRow(label: &'static str, current_idx: u8, on_change: EventHandler<u8>) -> Element {
    let mut expanded = use_signal(|| false);
    let current_hex = ansi_256_to_hex(current_idx);
    rsx! {
        div { style: "margin-bottom:10px;",
            // Label + current swatch (clicking swatch toggles grid)
            div { style: "display:flex; align-items:center; gap:8px;",
                span { style: "color:#cdd6f4; font-size:12px; min-width:72px;", "{label}" }
                div {
                    style: "width:24px; height:24px; border-radius:4px; background:{current_hex}; border:2px solid #6c7086; flex-shrink:0; cursor:pointer;",
                    title: "Click to change",
                    onclick: move |_| expanded.set(!expanded()),
                }
                button {
                    style: "background:none; border:none; color:#6c7086; font-size:11px; cursor:pointer; padding:0 2px; line-height:1;",
                    onclick: move |_| expanded.set(!expanded()),
                    if expanded() { "▲" } else { "▼" }
                }
            }
            // Curated color grid — only shown when expanded
            if expanded() {
                div { style: "display:flex; flex-wrap:wrap; gap:3px; margin-top:6px; padding:6px; background:#11111b; border:1px solid #313244; border-radius:4px;",
                    for &idx in CURATED_COLORS {
                        {
                            let hex = ansi_256_to_hex(idx);
                            let is_active = idx == current_idx;
                            let border = if is_active {
                                "2px solid #ffffff"
                            } else {
                                "2px solid transparent"
                            };
                            rsx! {
                                div {
                                    key: "{idx}",
                                    style: "width:16px; height:16px; border-radius:2px; background:{hex}; cursor:pointer; border:{border}; box-sizing:border-box;",
                                    onclick: move |_| {
                                        on_change.call(idx);
                                        expanded.set(false);
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SettingsTab(config: Signal<StatuslineConfig>) -> Element {
    let bar_style = config.read().bar_style.clone();
    let use_unicode = config.read().use_unicode_text;
    let palette = config.read().palette.clone();
    let editor_font = config.read().editor_font.clone();

    let btn = |active: bool| -> &'static str {
        if active {
            "background:#89b4fa; color:#1e1e2e; border:none; border-radius:4px; padding:4px 14px; font-size:12px; cursor:pointer; font-weight:bold;"
        } else {
            "background:#313244; color:#6c7086; border:1px solid #45475a; border-radius:4px; padding:4px 14px; font-size:12px; cursor:pointer;"
        }
    };

    rsx! {
        div {
            // Section 1: Theme Presets
            div { style: "margin-bottom:20px;",
                div { style: "color:#cdd6f4; font-size:14px; font-weight:bold; margin:0 0 8px 0;", "Theme Presets" }
                div { style: "display:flex; gap:8px; flex-wrap:wrap;",
                    for &(name, ref preset) in THEME_PRESETS {
                        {
                            let is_active = palette == *preset;
                            let preset_clone = preset.clone();
                            rsx! {
                                button {
                                    key: "{name}",
                                    style: btn(is_active),
                                    onclick: move |_| {
                                        config.write().palette = preset_clone.clone();
                                        autosave(&config);
                                    },
                                    "{name}"
                                }
                            }
                        }
                    }
                }
            }

            // Section 2: Palette Roles
            div { style: "margin-bottom:20px;",
                div { style: "color:#cdd6f4; font-size:14px; font-weight:bold; margin:0 0 10px 0;", "Palette Roles" }
                div { style: "display:grid; grid-template-columns:repeat(auto-fill, minmax(280px, 1fr)); gap:4px 24px;",
                for &role in PALETTE_ROLES {
                    {
                        let current_idx = palette.resolve(role);
                        rsx! {
                            PaletteRoleRow {
                                key: "{role.name()}",
                                label: role.label(),
                                current_idx,
                                on_change: move |idx: u8| {
                                    let mut cfg = config.write();
                                    match role {
                                        crate::theme::PaletteRole::Primary => cfg.palette.primary = idx,
                                        crate::theme::PaletteRole::Accent => cfg.palette.accent = idx,
                                        crate::theme::PaletteRole::Success => cfg.palette.success = idx,
                                        crate::theme::PaletteRole::Warning => cfg.palette.warning = idx,
                                        crate::theme::PaletteRole::Danger => cfg.palette.danger = idx,
                                        crate::theme::PaletteRole::Muted => cfg.palette.muted = idx,
                                        crate::theme::PaletteRole::Subtle => cfg.palette.subtle = idx,
                                    }
                                    drop(cfg);
                                    autosave(&config);
                                },
                            }
                        }
                    }
                }
                }
            }

            // Section 3: Bar Style
            div { style: "margin-bottom:20px;",
                div { style: "color:#cdd6f4; font-size:14px; font-weight:bold; margin:0 0 8px 0;", "Bar Style" }
                div { style: "display:flex; gap:8px;",
                    button {
                        style: btn(bar_style == BarStyle::Block),
                        onclick: move |_| {
                            config.write().bar_style = BarStyle::Block;
                            autosave(&config);
                        },
                        "Block"
                    }
                    button {
                        style: btn(bar_style == BarStyle::Dot),
                        onclick: move |_| {
                            config.write().bar_style = BarStyle::Dot;
                            autosave(&config);
                        },
                        "Dot"
                    }
                    button {
                        style: btn(bar_style == BarStyle::Ascii),
                        onclick: move |_| {
                            config.write().bar_style = BarStyle::Ascii;
                            autosave(&config);
                        },
                        "Ascii"
                    }
                }
            }

            // Section 4: Unicode Text
            div { style: "margin-bottom:20px;",
                div { style: "color:#cdd6f4; font-size:14px; font-weight:bold; margin:0 0 8px 0;", "Unicode Text" }
                button {
                    style: btn(use_unicode),
                    onclick: move |_| {
                        let v = config.read().use_unicode_text;
                        config.write().use_unicode_text = !v;
                        autosave(&config);
                    },
                    if use_unicode { "Enabled" } else { "Disabled" }
                }
            }

            // Section 5: Editor Font
            div { style: "margin-bottom:20px;",
                div { style: "color:#cdd6f4; font-size:14px; font-weight:bold; margin:0 0 8px 0;", "Editor Font" }
                div { style: "display:flex; gap:6px; flex-wrap:wrap; align-items:center;",
                    {
                        let font_presets: &[(&str, Option<&str>)] = &[
                            ("Default", None),
                            ("JetBrainsMono NF", Some("JetBrainsMono Nerd Font")),
                            ("Fira Code", Some("Fira Code")),
                            ("Ubuntu Mono", Some("Ubuntu Mono")),
                            ("DejaVu Sans Mono", Some("DejaVu Sans Mono")),
                            ("Consolas", Some("Consolas")),
                            ("Menlo", Some("Menlo")),
                        ];
                        let is_preset = font_presets.iter().any(|(_, v)| v.map(|s| s.to_string()).as_deref() == editor_font.as_deref());
                        let custom_value = if is_preset { String::new() } else { editor_font.clone().unwrap_or_default() };
                        rsx! {
                            for &(label, preset_val) in font_presets {
                                {
                                    let is_active = editor_font.as_deref() == preset_val;
                                    let preset_owned = preset_val.map(|s| s.to_string());
                                    rsx! {
                                        button {
                                            key: "{label}",
                                            style: btn(is_active),
                                            onclick: move |_| {
                                                config.write().editor_font = preset_owned.clone();
                                                autosave(&config);
                                            },
                                            "{label}"
                                        }
                                    }
                                }
                            }
                            input {
                                r#type: "text",
                                placeholder: "Font family name...",
                                value: "{custom_value}",
                                style: "background:#181825; color:#cdd6f4; border:1px solid #45475a; border-radius:4px; padding:4px 8px; font-size:12px; width:160px;",
                                oninput: move |evt| {
                                    let v = evt.value();
                                    config.write().editor_font = if v.trim().is_empty() { None } else { Some(v) };
                                    autosave(&config);
                                }
                            }
                        }
                    }
                }
            }

            // Section 6: Reset
            div { style: "margin-bottom:20px;",
                div { style: "color:#cdd6f4; font-size:14px; font-weight:bold; margin:0 0 8px 0;", "Reset to Defaults" }
                button {
                    style: "background:#f38ba8; color:#1e1e2e; border:none; border-radius:4px; padding:4px 14px; font-size:12px; cursor:pointer; font-weight:bold;",
                    onclick: move |_| {
                        {
                            let mut cfg = config.write();
                            cfg.palette = ThemePalette::default();
                            cfg.bar_style = BarStyle::default();
                            cfg.use_unicode_text = true;
                        }
                        autosave(&config);
                    },
                    "Reset"
                }
            }
        }
    }
}

// ---- lines tab -------------------------------------------------------------

#[component]
fn LinesTab(config: Signal<StatuslineConfig>, plugin_metas: Signal<Vec<PluginMeta>>) -> Element {
    rsx! {
        div {
            for line_idx in 0..3usize {
                LineRow { config, plugin_metas, line_idx }
            }
        }
    }
}

#[component]
fn LineRow(
    config: Signal<StatuslineConfig>,
    plugin_metas: Signal<Vec<PluginMeta>>,
    line_idx: usize,
) -> Element {
    let widgets: Vec<String> = get_line(&config.read(), line_idx).clone();
    let line_idx_str = line_idx.to_string();

    rsx! {
        div { style: "margin-bottom:16px;",
            div { style: "font-size:11px; color:#6c7086; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:6px;",
                "Line {line_idx + 1}"
            }
            div {
                "data-drop-line": "{line_idx_str}",
                style: "display:flex; flex-wrap:wrap; gap:6px; align-items:center; background:#181825; border:1px solid #313244; border-radius:6px; padding:8px; min-height:44px; transition: border-color 0.1s, background 0.1s;",

                if widgets.is_empty() {
                    span { style: "color:#45475a; font-size:12px; font-style:italic;", "empty" }
                } else {
                    for (widget_idx, name) in widgets.iter().enumerate() {
                        LineChip { config, line_idx, widget_idx, name: name.clone() }
                    }
                }
                // Drop zone at end of line for appending
                div { class: "dnd-drop-zone" }
                AddWidgetSelect { config, plugin_metas, line_idx, current_widgets: widgets.clone() }
            }
        }
    }
}

#[component]
fn LineChip(
    config: Signal<StatuslineConfig>,
    line_idx: usize,
    widget_idx: usize,
    name: String,
) -> Element {
    let dnd_id = format!("{line_idx},{widget_idx}");

    rsx! {
        span {
            "data-dnd": "{dnd_id}",
            style: "display:inline-flex; align-items:center; gap:4px; background:#313244; color:#cdd6f4; border-radius:4px; padding:3px 8px; cursor:grab; user-select:none; font-size:13px; font-weight:bold; border-left:3px solid transparent; transition: opacity 0.1s, border-left-color 0.1s;",
            "{name}"
            span {
                "data-no-drag": "1",
                style: "margin-left:2px; opacity:0.6; cursor:pointer; font-size:11px; font-weight:normal;",
                onclick: move |_| {
                    {
                        let mut cfg = config.write();
                        let line = get_line_mut(&mut cfg, line_idx);
                        if widget_idx < line.len() {
                            line.remove(widget_idx);
                        }
                    }
                    autosave(&config);
                },
                "×"
            }
        }
    }
}

#[component]
fn AddWidgetSelect(
    config: Signal<StatuslineConfig>,
    plugin_metas: Signal<Vec<PluginMeta>>,
    line_idx: usize,
    current_widgets: Vec<String>,
) -> Element {
    rsx! {
        select {
            "data-no-drag": "1",
            style: "background:#313244; color:#6c7086; border:1px solid #45475a; border-radius:4px; padding:3px 6px; font-size:12px; cursor:pointer;",
            onchange: move |evt: Event<FormData>| {
                let name = evt.value();
                if !name.is_empty() {
                    {
                        let mut cfg = config.write();
                        get_line_mut(&mut cfg, line_idx).push(name);
                    }
                    autosave(&config);
                }
            },
            option { value: "", disabled: true, selected: true, "＋ add widget" }
            for w in WIDGETS.iter() {
                if !current_widgets.iter().any(|s| s == w.name) {
                    option { value: "{w.name}", "{w.name} — {w.description}" }
                }
            }
            for p in plugin_metas.read().iter() {
                if !current_widgets.contains(&p.name) {
                    option { value: "{p.name}", "{p.name} — {p.description}" }
                }
            }
        }
    }
}

// ---- widgets tab -----------------------------------------------------------

#[component]
fn WidgetsTab(config: Signal<StatuslineConfig>, plugin_metas: Signal<Vec<PluginMeta>>) -> Element {
    let metas = plugin_metas.read().clone();
    let mut show_create_form = use_signal(|| false);
    let mut new_name = use_signal(String::new);
    let mut new_lang = use_signal(|| "bash".to_string());
    let mut create_error = use_signal(String::new);

    rsx! {
        div {
            p { style: "color:#6c7086; font-size:12px; margin:0 0 12px;",
                "Reorder or hide components per widget. Compact strips labels and separators."
            }
            for w in WIDGETS.iter().filter(|w| !w.default_components.is_empty() || w.has_compact || !w.color_slots.is_empty() || !w.icon_slots.is_empty()) {
                WidgetAccordion {
                    config,
                    widget_name: w.name.to_string(),
                    all_components: w.default_components.iter().map(|s| s.to_string()).collect(),
                    has_compact: w.has_compact,
                }
            }
            div { style: "margin-top:16px;",
                div { style: "display:flex; align-items:center; gap:8px; margin-bottom:8px;",
                    div { style: "font-size:11px; color:#6c7086; text-transform:uppercase; letter-spacing:0.05em;", "Custom Widgets" }
                    button {
                        style: "background:#313244; color:#cdd6f4; border:1px solid #45475a; border-radius:4px; padding:1px 8px; font-size:12px; cursor:pointer; line-height:1.6;",
                        onclick: move |_| {
                            let current = *show_create_form.read();
                            show_create_form.set(!current);
                            new_name.set(String::new());
                            new_lang.set("bash".to_string());
                            create_error.set(String::new());
                        },
                        "+"
                    }
                }
                if *show_create_form.read() {
                    div { style: "background:#181825; border:1px solid #313244; border-radius:6px; padding:12px; margin-bottom:8px;",
                        // New plugin from template
                        div { style: "display:flex; gap:8px; align-items:center; flex-wrap:wrap; margin-bottom:8px;",
                            input {
                                r#type: "text",
                                placeholder: "plugin-name",
                                value: "{new_name.read()}",
                                style: "background:#11111b; color:#cdd6f4; border:1px solid #313244; border-radius:4px; padding:4px 8px; font-size:13px; font-family:monospace;",
                                oninput: move |evt| new_name.set(evt.value()),
                            }
                            div { style: "display:flex; gap:4px;",
                                button {
                                    style: if *new_lang.read() == "bash" {
                                        "background:#89b4fa; color:#1e1e2e; border:none; border-radius:4px; padding:4px 10px; font-size:12px; cursor:pointer; font-weight:bold;"
                                    } else {
                                        "background:#313244; color:#cdd6f4; border:1px solid #45475a; border-radius:4px; padding:4px 10px; font-size:12px; cursor:pointer;"
                                    },
                                    onclick: move |_| new_lang.set("bash".to_string()),
                                    "bash"
                                }
                                button {
                                    style: if *new_lang.read() == "python" {
                                        "background:#89b4fa; color:#1e1e2e; border:none; border-radius:4px; padding:4px 10px; font-size:12px; cursor:pointer; font-weight:bold;"
                                    } else {
                                        "background:#313244; color:#cdd6f4; border:1px solid #45475a; border-radius:4px; padding:4px 10px; font-size:12px; cursor:pointer;"
                                    },
                                    onclick: move |_| new_lang.set("python".to_string()),
                                    "python"
                                }
                            }
                            button {
                                style: "background:#89b4fa; color:#1e1e2e; border:none; border-radius:4px; padding:4px 12px; font-size:13px; cursor:pointer; font-weight:bold;",
                                onclick: move |_| {
                                    let name = new_name.read().trim().to_string();
                                    if name.is_empty() {
                                        create_error.set("Name cannot be empty.".to_string());
                                        return;
                                    }
                                    if name.contains(' ') {
                                        create_error.set("Name cannot contain spaces.".to_string());
                                        return;
                                    }
                                    use crate::widgets;
                                    if widgets::AVAILABLE.contains(&name.as_str()) {
                                        create_error.set(format!("'{name}' collides with a built-in widget."));
                                        return;
                                    }
                                    let lang = new_lang.read().clone();
                                    let (ext, template) = if lang == "python" {
                                        (
                                            "py",
                                            format!("#!/usr/bin/env python3\nimport json, sys\ndata = json.load(sys.stdin)\nprint(\"{name}\")\n"),
                                        )
                                    } else {
                                        (
                                            "sh",
                                            format!("#!/bin/bash\n# Reads Claude Code JSON from stdin, outputs widget text\necho \"{name}\"\n"),
                                        )
                                    };
                                    if let Err(e) = plugin::create_plugin(&name, ext, &template) {
                                        create_error.set(format!("Error: {e}"));
                                        return;
                                    }
                                    plugin_metas.set(plugin::list_plugin_metas());
                                    show_create_form.set(false);
                                    create_error.set(String::new());
                                },
                                "Create"
                            }
                        }
                        // Import existing binary/script via file picker
                        div { style: "display:flex; gap:8px; align-items:center; border-top:1px solid #313244; padding-top:8px;",
                            span { style: "color:#6c7086; font-size:12px;", "or" }
                            button {
                                style: "background:#313244; color:#cdd6f4; border:1px solid #45475a; border-radius:4px; padding:4px 12px; font-size:13px; cursor:pointer;",
                                onclick: move |_| {
                                    #[cfg(feature = "desktop")]
                                    {
                                        let file = rfd::FileDialog::new()
                                            .set_title("Import script")
                                            .pick_file();
                                        if let Some(path) = file {
                                            match plugin::import_plugin(&path) {
                                                Ok(_) => {
                                                    plugin_metas.set(plugin::list_plugin_metas());
                                                    show_create_form.set(false);
                                                    create_error.set(String::new());
                                                }
                                                Err(e) => {
                                                    create_error.set(format!("Import failed: {e}"))
                                                }
                                            }
                                        }
                                    }
                                },
                                "Import existing file..."
                            }
                        }
                        if !create_error.read().is_empty() {
                            div { style: "color:#f38ba8; font-size:12px; margin-top:6px;", "{create_error.read()}" }
                        }
                    }
                }
                for p in metas.iter() {
                    WidgetAccordion {
                        config,
                        widget_name: p.name.clone(),
                        all_components: p.components.clone(),
                        has_compact: p.has_compact,
                        plugin_metas: Some(plugin_metas),
                    }
                }
            }
        }
    }
}

#[component]
fn WidgetAccordion(
    config: Signal<StatuslineConfig>,
    widget_name: String,
    all_components: Vec<String>,
    has_compact: bool,
    plugin_metas: Option<Signal<Vec<PluginMeta>>>,
) -> Element {
    let is_custom = plugin_metas.is_some();

    let source_init = if is_custom {
        plugin::read_plugin_source(&widget_name).unwrap_or_default()
    } else {
        String::new()
    };
    let mut source = use_signal(|| source_init);
    let mut preview_result: Signal<Option<plugin::PluginOutput>> = use_signal(|| None);
    let mut editing = use_signal(|| false);
    let mut renaming = use_signal(|| false);
    let mut rename_value = use_signal(|| widget_name.clone());
    let mut rename_error = use_signal(String::new);

    let (compact, current_comps) = {
        let cfg = config.read();
        let wc = cfg.widgets.get(widget_name.as_str());
        (
            wc.map(|w| w.compact).unwrap_or(false),
            wc.map(|w| w.components.clone()).unwrap_or_default(),
        )
    };
    let effective_comps: Vec<String> = if current_comps.is_empty() {
        all_components.clone()
    } else {
        current_comps.clone()
    };
    let has_custom = !current_comps.is_empty();
    let has_components = !all_components.is_empty();
    let compact_btn_style = if compact {
        "background:#89b4fa; color:#1e1e2e; border:none; border-radius:3px; padding:1px 7px; font-size:11px; cursor:pointer; font-weight:bold;"
    } else {
        "background:#313244; color:#6c7086; border:1px solid #45475a; border-radius:3px; padding:1px 7px; font-size:11px; cursor:pointer;"
    };

    let (widget_colors, widget_icons_map, widget_bar_style, has_appearance_override, palette) = {
        let cfg = config.read();
        let wc = cfg.widgets.get(widget_name.as_str());
        let has_override =
            wc.is_some_and(|w: &crate::types::WidgetConfig| w.has_appearance_overrides());
        (
            wc.and_then(|w| w.theme.clone()).unwrap_or_default(),
            wc.and_then(|w| w.icons.clone()).unwrap_or_default(),
            wc.and_then(|w| w.bar_style.clone()),
            has_override,
            cfg.palette.clone(),
        )
    };

    let wmeta = if is_custom {
        plugin::widget_meta(&widget_name)
    } else {
        None
    };
    let custom_color_slots = wmeta
        .as_ref()
        .map(|m| m.theme_slots.clone())
        .unwrap_or_default();
    let custom_icon_slots = wmeta
        .as_ref()
        .map(|m| m.icon_slots.clone())
        .unwrap_or_default();

    let wn = widget_name.clone();
    let wn2 = widget_name.clone();
    let wn_appearance = widget_name.clone();
    let wn_reset_appearance = widget_name.clone();
    let ac = all_components.clone();
    let name_save = widget_name.clone();
    let name_run = widget_name.clone();
    let name_delete = widget_name.clone();
    let name_rename = widget_name.clone();

    let btn_active = "background:#89b4fa; color:#1e1e2e; border:none; border-radius:4px; padding:4px 14px; font-size:12px; cursor:pointer; font-weight:bold;";
    let btn_inactive = "background:#313244; color:#6c7086; border:1px solid #45475a; border-radius:4px; padding:4px 14px; font-size:12px; cursor:pointer;";

    // Determine if this widget uses bar-style widgets
    let show_bar_style = matches!(widget_name.as_str(), "context_bar" | "quota");

    let wref_color_slots: &[widget_reference::ThemeSlot] = widget_ref(widget_name.as_str())
        .map(|w| w.color_slots)
        .unwrap_or(&[]);
    let wref_icon_slots: &[widget_reference::IconSlot] = widget_ref(widget_name.as_str())
        .map(|w| w.icon_slots)
        .unwrap_or(&[]);

    rsx! {
        details {
            style: "border:1px solid #313244; border-radius:6px; margin-bottom:8px; background:#181825;",
            summary {
                style: "display:flex; align-items:center; gap:10px; padding:8px 12px; cursor:pointer; list-style:none; user-select:none;",
                span { style: "font-weight:bold; color:#cba6f7; min-width:90px;", "{widget_name}" }
                span { style: "font-family:monospace; font-size:12px; flex:1;",
                    {widget_preview(&widget_name, compact, &effective_comps, &config.read())}
                }
                if has_compact {
                    button {
                        style: compact_btn_style,
                        onclick: move |evt| {
                            evt.stop_propagation();
                            {
                                let mut cfg = config.write();
                                let wc = cfg.widgets.entry(wn.clone()).or_default();
                                wc.compact = !wc.compact;
                            }
                            autosave(&config);
                        },
                        "compact"
                    }
                }
                if has_custom {
                    span { style: "background:#313244; color:#6c7086; font-size:10px; border-radius:3px; padding:1px 5px;", "custom" }
                }
            }
            div { style: "padding:8px 12px 12px; display:flex; flex-direction:column; gap:12px;",
                if has_components {
                    div {
                        div { style: "font-size:11px; color:#6c7086; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:6px;", "Components" }
                        div {
                            style: "display:flex; flex-wrap:wrap; gap:6px; align-items:center;",
                            for (idx, comp) in effective_comps.iter().cloned().enumerate() {
                                CompChip { config, widget_name: wn2.clone(), all_components: ac.clone(), comp_name: comp, comp_idx: idx }
                            }
                            div { class: "dnd-drop-zone", "data-comp-drop": "{wn2}" }
                            {
                                let hidden: Vec<String> = all_components
                                    .iter()
                                    .filter(|c| !effective_comps.contains(c))
                                    .cloned()
                                    .collect();
                                let wn3 = wn2.clone();
                                let ac2 = all_components.clone();
                                if !hidden.is_empty() {
                                    rsx! {
                                        select {
                                            style: "background:#313244; color:#6c7086; border:1px solid #45475a; border-radius:4px; padding:2px 6px; font-size:12px; cursor:pointer;",
                                            onchange: move |evt: Event<FormData>| {
                                                let comp = evt.value();
                                                if !comp.is_empty() {
                                                    {
                                                        let mut cfg = config.write();
                                                        let wc = cfg.widgets.entry(wn3.clone()).or_default();
                                                        if wc.components.is_empty() {
                                                            wc.components = ac2.clone();
                                                        }
                                                        if !wc.components.contains(&comp) {
                                                            wc.components.push(comp);
                                                        }
                                                    }
                                                    autosave(&config);
                                                }
                                            },
                                            option { value: "", disabled: true, selected: true, "＋ add" }
                                            for comp in hidden { option { value: "{comp}", "{comp}" } }
                                        }
                                    }
                                } else { rsx! {} }
                            }
                        }
                        if has_custom {
                            {
                                let wn4 = wn2.clone();
                                rsx! {
                                    button {
                                        style: "background:transparent; border:none; color:#6c7086; font-size:12px; cursor:pointer; margin-top:6px; text-decoration:underline;",
                                        onclick: move |_| {
                                            if let Some(wc) = config.write().widgets.get_mut(&wn4) {
                                                wc.components.clear();
                                            }
                                            autosave(&config);
                                        },
                                        "reset order"
                                    }
                                }
                            }
                        }
                    }
                }

                // Appearance section
                details {
                    style: "border:1px solid #313244; border-radius:4px; background:#11111b;",
                    summary {
                        style: "padding:6px 10px; cursor:pointer; font-size:11px; color:#6c7086; text-transform:uppercase; letter-spacing:0.05em; list-style:none; user-select:none;",
                        "Appearance"
                        if has_appearance_override {
                            span { style: "margin-left:6px; background:#313244; color:#89b4fa; font-size:10px; border-radius:3px; padding:1px 5px;", "overrides" }
                        }
                    }
                    div { style: "padding:10px;",
                        // Semantic color slots for built-in widgets
                        if !wref_color_slots.is_empty() {
                            div { style: "margin-bottom:10px;",
                                div { style: "font-size:11px; color:#6c7086; margin-bottom:6px;", "Colors" }
                                for slot in wref_color_slots.iter() {
                                    {
                                        let wna = wn_appearance.clone();
                                        let slot_key = slot.key.to_string();
                                        let slot_key_for_closure = slot_key.clone();
                                        let label = slot.label;
                                        let default_role = Some(slot.palette_role);
                                        let current_val: Option<ThemeValue> =
                                            widget_colors.get(slot.key).cloned();
                                        rsx! {
                                            WidgetColorPicker {
                                                key: "{slot_key}",
                                                label,
                                                value: current_val,
                                                palette: palette.clone(),
                                                default_role,
                                                on_change: move |v: Option<ThemeValue>| {
                                                    let mut binding = config.write();
                                                    let wc = binding.widgets.entry(wna.clone()).or_default();
                                                    let theme_map = wc.theme.get_or_insert_with(Default::default);
                                                    match v {
                                                        Some(cv) => { theme_map.insert(slot_key_for_closure.clone(), cv); }
                                                        None => { theme_map.remove(&slot_key_for_closure); }
                                                    }
                                                    if wc.theme.as_ref().is_some_and(|m: &std::collections::HashMap<String, ThemeValue>| m.is_empty()) {
                                                        wc.theme = None;
                                                    }
                                                    drop(binding);
                                                    autosave(&config);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Semantic icon slots for built-in widgets
                        if !wref_icon_slots.is_empty() {
                            div { style: "margin-bottom:10px;",
                                div { style: "font-size:11px; color:#6c7086; margin-bottom:6px;", "Icons" }
                                div { style: "display:grid; grid-template-columns:1fr 1fr; gap:6px 20px;",
                                    for slot in wref_icon_slots.iter() {
                                        if slot.is_char {
                                            {
                                                let wna = wn_appearance.clone();
                                                let slot_key = slot.key.to_string();
                                                let label = slot.label;
                                                let placeholder: char = slot.default_value.chars().next().unwrap_or(' ');
                                                let current_val = widget_icons_map.get(slot.key).and_then(|s| s.chars().next());
                                                rsx! { CharInputRow { label, value: current_val, placeholder,
                                                    on_change: move |v: Option<char>| {
                                                        let mut binding = config.write();
                                                        let wc = binding.widgets.entry(wna.clone()).or_default();
                                                        let icons = wc.icons.get_or_insert_with(Default::default);
                                                        match v {
                                                            Some(c) => { icons.insert(slot_key.clone(), c.to_string()); }
                                                            None => { icons.remove(&slot_key); }
                                                        }
                                                        if wc.icons.as_ref().is_some_and(|m| m.is_empty()) {
                                                            wc.icons = None;
                                                        }
                                                        drop(binding);
                                                        autosave(&config);
                                                    }
                                                }}
                                            }
                                        } else {
                                            {
                                                let wna = wn_appearance.clone();
                                                let slot_key = slot.key.to_string();
                                                let label = slot.label;
                                                let placeholder = slot.default_value;
                                                let current_val = widget_icons_map.get(slot.key).cloned();
                                                rsx! { IconInputRow { label, value: current_val, placeholder,
                                                    on_change: move |v: Option<String>| {
                                                        let mut binding = config.write();
                                                        let wc = binding.widgets.entry(wna.clone()).or_default();
                                                        let icons = wc.icons.get_or_insert_with(Default::default);
                                                        match v {
                                                            Some(s) => { icons.insert(slot_key.clone(), s); }
                                                            None => { icons.remove(&slot_key); }
                                                        }
                                                        if wc.icons.as_ref().is_some_and(|m| m.is_empty()) {
                                                            wc.icons = None;
                                                        }
                                                        drop(binding);
                                                        autosave(&config);
                                                    }
                                                }}
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Custom widget color slots (from plugin TOML)
                        if is_custom && !custom_color_slots.is_empty() {
                            div { style: "margin-bottom:10px;",
                                div { style: "font-size:11px; color:#6c7086; margin-bottom:6px;", "Colors" }
                                for slot in custom_color_slots.iter() {
                                    {
                                        let wna = wn_appearance.clone();
                                        let slot_key = slot.key.clone();
                                        let slot_key2 = slot.key.clone();
                                        let label = slot.key.clone();
                                        let current_val: Option<ThemeValue> = widget_colors.get(&slot.key).cloned();
                                        rsx! {
                                            WidgetColorPicker {
                                                key: "{slot_key}",
                                                label,
                                                value: current_val,
                                                palette: palette.clone(),
                                                default_role: None,
                                                on_change: move |v: Option<ThemeValue>| {
                                                    let mut binding = config.write();
                                                    let wc = binding.widgets.entry(wna.clone()).or_default();
                                                    let theme_map = wc.theme.get_or_insert_with(Default::default);
                                                    match v {
                                                        Some(cv) => { theme_map.insert(slot_key2.clone(), cv); }
                                                        None => { theme_map.remove(&slot_key2); }
                                                    }
                                                    if wc.theme.as_ref().is_some_and(|m: &std::collections::HashMap<String, ThemeValue>| m.is_empty()) {
                                                        wc.theme = None;
                                                    }
                                                    drop(binding);
                                                    autosave(&config);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Custom widget icon slots (from plugin TOML)
                        if is_custom && !custom_icon_slots.is_empty() {
                            div { style: "margin-bottom:10px;",
                                div { style: "font-size:11px; color:#6c7086; margin-bottom:6px;", "Icons" }
                                div { style: "display:grid; grid-template-columns:1fr 1fr; gap:6px 20px;",
                                    for slot in custom_icon_slots.iter() {
                                        {
                                            let wna = wn_appearance.clone();
                                            let slot_key = slot.key.clone();
                                            let slot_key2 = slot.key.clone();
                                            let label = slot.key.clone();
                                            let placeholder = slot.default_value.clone();
                                            let current_val = widget_icons_map.get(&slot.key).cloned();
                                            rsx! {
                                                IconInputRow {
                                                    key: "{slot_key}",
                                                    label,
                                                    value: current_val,
                                                    placeholder,
                                                    on_change: move |v: Option<String>| {
                                                        let mut binding = config.write();
                                                        let wc = binding.widgets.entry(wna.clone()).or_default();
                                                        let icons = wc.icons.get_or_insert_with(Default::default);
                                                        match v {
                                                            Some(s) => { icons.insert(slot_key2.clone(), s); }
                                                            None => { icons.remove(&slot_key2); }
                                                        }
                                                        if wc.icons.as_ref().is_some_and(|m| m.is_empty()) {
                                                            wc.icons = None;
                                                        }
                                                        drop(binding);
                                                        autosave(&config);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Bar style override (context_bar and quota only)
                        if show_bar_style {
                            div { style: "margin-bottom:10px;",
                                div { style: "font-size:11px; color:#6c7086; margin-bottom:6px;", "Bar Style" }
                                div { style: "display:flex; gap:6px;",
                                    {
                                        let wna = wn_appearance.clone();
                                        rsx! {
                                            button {
                                                style: if widget_bar_style.is_none() { btn_active } else { btn_inactive },
                                                onclick: move |_| {
                                                    if let Some(wc) = config.write().widgets.get_mut(&wna) {
                                                        wc.bar_style = None;
                                                    }
                                                    autosave(&config);
                                                },
                                                "Inherit"
                                            }
                                        }
                                    }
                                    {
                                        let wna = wn_appearance.clone();
                                        rsx! {
                                            button {
                                                style: if widget_bar_style == Some(crate::theme::BarStyle::Block) { btn_active } else { btn_inactive },
                                                onclick: move |_| {
                                                    config.write().widgets.entry(wna.clone()).or_default().bar_style = Some(crate::theme::BarStyle::Block);
                                                    autosave(&config);
                                                },
                                                "Block"
                                            }
                                        }
                                    }
                                    {
                                        let wna = wn_appearance.clone();
                                        rsx! {
                                            button {
                                                style: if widget_bar_style == Some(crate::theme::BarStyle::Dot) { btn_active } else { btn_inactive },
                                                onclick: move |_| {
                                                    config.write().widgets.entry(wna.clone()).or_default().bar_style = Some(crate::theme::BarStyle::Dot);
                                                    autosave(&config);
                                                },
                                                "Dot"
                                            }
                                        }
                                    }
                                    {
                                        let wna = wn_appearance.clone();
                                        rsx! {
                                            button {
                                                style: if widget_bar_style == Some(crate::theme::BarStyle::Ascii) { btn_active } else { btn_inactive },
                                                onclick: move |_| {
                                                    config.write().widgets.entry(wna.clone()).or_default().bar_style = Some(crate::theme::BarStyle::Ascii);
                                                    autosave(&config);
                                                },
                                                "Ascii"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Reset appearance button
                        if has_appearance_override {
                            button {
                                style: "background:transparent; border:none; color:#f38ba8; font-size:12px; cursor:pointer; text-decoration:underline;",
                                onclick: move |_| {
                                    if let Some(wc) = config.write().widgets.get_mut(&wn_reset_appearance) {
                                        wc.theme = None;
                                        wc.icons = None;
                                        wc.bar_style = None;
                                    }
                                    autosave(&config);
                                },
                                "Reset appearance"
                            }
                        }
                    }
                }

                // Custom widget editing section
                if is_custom {
                    div { style: "border-top:1px solid #313244; padding-top:8px; display:flex; align-items:center; gap:8px;",
                        button {
                            style: if *editing.read() {
                                "background:#89b4fa; color:#1e1e2e; border:none; border-radius:4px; padding:3px 8px; font-size:12px; cursor:pointer;"
                            } else {
                                "background:#313244; color:#6c7086; border:1px solid #45475a; border-radius:4px; padding:3px 8px; font-size:12px; cursor:pointer;"
                            },
                            onclick: move |_| {
                                let cur = *editing.read();
                                editing.set(!cur);
                            },
                            "✏ Edit source"
                        }
                    }
                    if *editing.read() {
                        // Source editor
                        div {
                            div { style: "font-size:11px; color:#6c7086; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:6px;", "Source" }
                            textarea {
                                style: "background:#11111b; color:#cdd6f4; border:1px solid #313244; border-radius:4px; font-family:monospace; font-size:12px; width:100%; min-height:120px; padding:8px; resize:vertical; box-sizing:border-box;",
                                value: "{source.read()}",
                                oninput: move |evt| source.set(evt.value()),
                            }
                            button {
                                style: "background:#89b4fa; color:#1e1e2e; border:none; border-radius:4px; padding:4px 12px; font-size:12px; cursor:pointer; font-weight:bold; margin-top:6px;",
                                onclick: move |_| {
                                    if let Err(e) = plugin::write_plugin_source(&name_save, &source.read()) {
                                        eprintln!("write_plugin_source failed: {e}");
                                    } else if let Some(mut pm) = plugin_metas {
                                        pm.set(plugin::list_plugin_metas());
                                    }
                                },
                                "Save"
                            }
                        }
                        // Live preview
                        div {
                            div { style: "font-size:11px; color:#6c7086; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:6px;", "Live Preview" }
                            button {
                                style: "background:#89b4fa; color:#1e1e2e; border:none; border-radius:4px; padding:4px 12px; font-size:12px; cursor:pointer; font-weight:bold; margin-bottom:6px;",
                                onclick: move |_| {
                                    let result = plugin::run_plugin_full(&name_run, &plugin::mock_stdin_json());
                                    preview_result.set(result);
                                },
                                "Run"
                            }
                            {
                                let pr = preview_result.read();
                                let (text, comps) = match pr.as_ref() {
                                    Some(out) => {
                                        let cfg = config.read();
                                        let wc_comps = cfg.widgets.get(widget_name.as_str())
                                            .map(|w| &w.components)
                                            .filter(|c| !c.is_empty());
                                        let display = if let Some(ordered) = wc_comps {
                                            out.compose(ordered, compact)
                                        } else {
                                            out.compose(&[], compact)
                                        };
                                        (display, &out.components)
                                    }
                                    None => (String::new(), &vec![] as &Vec<String>),
                                };
                                let html = crate::theme::ansi_to_html(&text);
                                rsx! {
                                    div {
                                        style: "background:#11111b; border:1px solid #313244; border-radius:4px; padding:8px; font-family:monospace; font-size:12px;",
                                        if html.is_empty() {
                                            span { style: "color:#6c7086; font-style:italic;", "click Run to preview" }
                                        } else {
                                            span { dangerous_inner_html: "{html}" }
                                        }
                                    }
                                    if !comps.is_empty() {
                                        div { style: "margin-top:4px;",
                                            span { style: "color:#6c7086; font-size:11px;", "Detected components: " }
                                            for c in comps.iter() {
                                                span { style: "background:#313244; color:#a6e3a1; font-size:11px; border-radius:3px; padding:1px 5px; margin-right:4px;", "{c}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Rename + Delete
                        div { style: "display:flex; gap:8px; align-items:center; flex-wrap:wrap;",
                            if *renaming.read() {
                                input {
                                    r#type: "text",
                                    value: "{rename_value.read()}",
                                    style: "background:#11111b; color:#cdd6f4; border:1px solid #313244; border-radius:4px; padding:4px 8px; font-size:13px; font-family:monospace;",
                                    oninput: move |evt| rename_value.set(evt.value()),
                                }
                                button {
                                    style: "background:#89b4fa; color:#1e1e2e; border:none; border-radius:4px; padding:4px 10px; font-size:12px; cursor:pointer; font-weight:bold;",
                                    onclick: move |_| {
                                        let new = rename_value.read().trim().to_string();
                                        if new == name_rename || new.is_empty() {
                                            renaming.set(false);
                                            return;
                                        }
                                        match plugin::rename_plugin(&name_rename, &new) {
                                            Ok(()) => {
                                                if let Some(mut pm) = plugin_metas {
                                                    pm.set(plugin::list_plugin_metas());
                                                }
                                                renaming.set(false);
                                            }
                                            Err(e) => rename_error.set(format!("{e}")),
                                        }
                                    },
                                    "Rename"
                                }
                                button {
                                    style: "background:#313244; color:#cdd6f4; border:1px solid #45475a; border-radius:4px; padding:4px 10px; font-size:12px; cursor:pointer;",
                                    onclick: move |_| renaming.set(false),
                                    "Cancel"
                                }
                                if !rename_error.read().is_empty() {
                                    span { style: "color:#f38ba8; font-size:12px;", "{rename_error.read()}" }
                                }
                            } else {
                                button {
                                    style: "background:#313244; color:#cdd6f4; border:1px solid #45475a; border-radius:4px; padding:4px 10px; font-size:12px; cursor:pointer;",
                                    onclick: move |_| {
                                        rename_value.set(widget_name.clone());
                                        renaming.set(true);
                                    },
                                    "Rename"
                                }
                                button {
                                    style: "background:#f38ba8; color:#1e1e2e; border:none; border-radius:4px; padding:4px 10px; font-size:12px; cursor:pointer; font-weight:bold;",
                                    onclick: move |_| {
                                        if let Err(e) = plugin::delete_plugin(&name_delete) {
                                            eprintln!("delete_plugin failed: {e}");
                                        } else if let Some(mut pm) = plugin_metas {
                                            pm.set(plugin::list_plugin_metas());
                                        }
                                    },
                                    "Delete"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CompChip(
    config: Signal<StatuslineConfig>,
    widget_name: String,
    all_components: Vec<String>,
    comp_name: String,
    comp_idx: usize,
) -> Element {
    let dnd_id = format!("{widget_name}:{comp_idx}");
    let desc = component_desc(&widget_name, &comp_name);
    let comp_for_remove = comp_name.clone();
    let wn = widget_name.clone();
    let ac = all_components.clone();

    rsx! {
        span {
            "data-comp-dnd": "{dnd_id}",
            style: "display:inline-flex; align-items:center; gap:4px; background:#313244; color:#cdd6f4; border-radius:4px; padding:3px 8px; cursor:grab; user-select:none; font-size:13px; font-weight:bold; border-left:3px solid transparent; transition: opacity 0.1s, border-left-color 0.1s;",
            title: "{desc}",
            "{comp_name}"
            span {
                "data-no-drag": "1",
                style: "margin-left:2px; opacity:0.6; cursor:pointer; font-size:11px; font-weight:normal;",
                onclick: move |_| {
                    {
                        let mut cfg = config.write();
                        let wc = cfg.widgets.entry(wn.clone()).or_default();
                        if wc.components.is_empty() {
                            wc.components = ac.clone();
                        }
                        wc.components.retain(|c| c != &comp_for_remove);
                    }
                    autosave(&config);
                },
                "×"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_within_same_line() {
        let mut config = StatuslineConfig::default();
        config.line1 = vec!["a".into(), "b".into(), "c".into()];
        perform_move(
            &mut config,
            DragState {
                src_line: 0,
                src_idx: 0,
            },
            DropTarget::Before(0, 2),
        );
        assert_eq!(config.line1, vec!["b", "a", "c"]);
    }

    #[test]
    fn move_across_lines() {
        let mut config = StatuslineConfig::default();
        config.line1 = vec!["a".into(), "b".into()];
        config.line2 = vec!["x".into()];
        perform_move(
            &mut config,
            DragState {
                src_line: 0,
                src_idx: 1,
            },
            DropTarget::Append(1),
        );
        assert_eq!(config.line1, vec!["a"]);
        assert_eq!(config.line2, vec!["x", "b"]);
    }

    #[test]
    fn move_before_in_different_line() {
        let mut config = StatuslineConfig::default();
        config.line1 = vec!["a".into(), "b".into()];
        config.line2 = vec!["x".into(), "y".into()];
        perform_move(
            &mut config,
            DragState {
                src_line: 0,
                src_idx: 0,
            },
            DropTarget::Before(1, 1),
        );
        assert_eq!(config.line1, vec!["b"]);
        assert_eq!(config.line2, vec!["x", "a", "y"]);
    }

    #[test]
    fn move_last_to_first() {
        let mut config = StatuslineConfig::default();
        config.line1 = vec!["a".into(), "b".into(), "c".into()];
        perform_move(
            &mut config,
            DragState {
                src_line: 0,
                src_idx: 2,
            },
            DropTarget::Before(0, 0),
        );
        assert_eq!(config.line1, vec!["c", "a", "b"]);
    }

    #[test]
    fn parse_line_drop_before() {
        let (drag, target) = parse_line_drop("0,1>1,0").unwrap();
        assert_eq!(drag.src_line, 0);
        assert_eq!(drag.src_idx, 1);
        assert_eq!(target, DropTarget::Before(1, 0));
    }

    #[test]
    fn parse_line_drop_append() {
        let (drag, target) = parse_line_drop("2,0>append:1").unwrap();
        assert_eq!(drag.src_line, 2);
        assert_eq!(drag.src_idx, 0);
        assert_eq!(target, DropTarget::Append(1));
    }

    #[test]
    fn parse_comp_drop_before() {
        match parse_comp_drop("cost:0>cost:2").unwrap() {
            CompDrop::Before(name, src, dst) => {
                assert_eq!(name, "cost");
                assert_eq!(src, 0);
                assert_eq!(dst, 2);
            }
            _ => panic!("expected Before"),
        }
    }

    #[test]
    fn parse_comp_drop_append() {
        match parse_comp_drop("cost:1>cost:append").unwrap() {
            CompDrop::Append(name, src) => {
                assert_eq!(name, "cost");
                assert_eq!(src, 1);
            }
            _ => panic!("expected Append"),
        }
    }

    #[test]
    fn parse_comp_drop_different_widgets_rejected() {
        assert!(parse_comp_drop("cost:0>git:1").is_none());
    }
}
