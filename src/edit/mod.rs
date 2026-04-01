mod widget_reference;

use anyhow::Result;
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

use crate::config::StatuslineConfig;
use crate::plugin::{self, PluginMeta};
use widget_reference::{component_desc, widget_ref, WIDGETS};

// ---- entry point -----------------------------------------------------------

const CUSTOM_HEAD: &str = r#"
<style>
html, body { margin:0; padding:0; overflow:hidden; background:#1e1e2e; }
body.dnd-active, body.dnd-active * { cursor: grabbing !important; user-select: none !important; -webkit-user-select: none !important; }
body.dnd-active select, body.dnd-active input, body.dnd-active textarea, body.dnd-active button { pointer-events: none !important; }
.dnd-src { opacity: 0.3 !important; }
.dnd-target { border-left: 3px solid #89b4fa !important; }
.dnd-row-target { border-color: #89b4fa !important; background: #1e1e3e !important; }
.dnd-drop-zone { width:64px; height:28px; border-radius:4px; flex-shrink:0; }
body.dnd-active .dnd-drop-zone { background: rgba(137,180,250,0.1); border: 1px dashed rgba(137,180,250,0.3); }
</style>
<script>
(function() {
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
      // Over a chip: drop before it (unless it's the source chip)
      var chipId = chip.getAttribute('data-dnd');
      if (chipId !== lineSrc) target = chipId;
    } else {
      // Not over any chip: check for drop zone or row empty space = append
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
      // Drop on component container = append to end
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
})();
</script>
"#;

pub fn run() -> Result<()> {
    dioxus::LaunchBuilder::new()
        .with_cfg(
            Config::default()
                .with_custom_head(CUSTOM_HEAD.to_string())
                .with_window(
                    WindowBuilder::new()
                        .with_title("Soffit")
                        .with_decorations(false)
                        .with_resizable(false)
                        .with_inner_size(LogicalSize::new(1100.0_f64, 530.0_f64)),
                ),
        )
        .launch(App);
    Ok(())
}

// ---- types -----------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tab {
    Lines,
    Widgets,
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

fn widget_preview(
    name: &str,
    compact: bool,
    components: &[String],
    config: &StatuslineConfig,
) -> Element {
    use crate::theme::ansi_256_to_hex;
    let tc = &config.theme;
    let dim_s = ansi_256_to_hex(tc.dim.unwrap_or(242));
    let blue_s = ansi_256_to_hex(tc.cyan.unwrap_or(111));
    let green_s = ansi_256_to_hex(tc.green.unwrap_or(114));
    let orange_s = ansi_256_to_hex(tc.orange.unwrap_or(215));
    let red_s = ansi_256_to_hex(tc.red.unwrap_or(203));
    let purple_s = ansi_256_to_hex(tc.purple.unwrap_or(183));
    let yellow_s = ansi_256_to_hex(tc.yellow.unwrap_or(228));
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
                    if !compact { "💸 " }
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
                    "update" => Some((orange, String::new())),
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
            let all: &[(&str, &str, &str, bool)] = &[
                ("bar", green, "■■■■□□□□", false),
                ("pct", green, "🯴🯲٪", false),
                ("tokens", dim, "42k/100k", true),
            ];
            let parts: Vec<(&str, &str)> = components
                .iter()
                .filter_map(|c| {
                    all.iter()
                        .find(|(k, _, _, _)| *k == c.as_str())
                        .filter(|(_, _, _, compact_hide)| !compact || !compact_hide)
                        .map(|(_, col, txt, _)| (*col, *txt))
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
            let all: &[(&str, &str, &str, bool)] = &[
                ("branch", blue, " main", false),
                ("staged", green, "•2", true),
                ("modified", orange, "~1", true),
                ("repo", dim, "jarvis", true),
            ];
            let parts: Vec<(&str, &str)> = components
                .iter()
                .filter_map(|c| {
                    all.iter()
                        .find(|(k, _, _, _)| *k == c.as_str())
                        .filter(|(_, _, _, compact_hide)| !compact || !compact_hide)
                        .map(|(_, col, txt, _)| (*col, *txt))
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
            let all: &[(&str, &str, &str)] = &[
                ("five_hour", blue, "5h:▓▓░░ 60%"),
                ("seven_day", green, "7d:▓▓▓░ 75%"),
            ];
            let parts: Vec<(&str, &str)> = components
                .iter()
                .filter_map(|c| {
                    all.iter()
                        .find(|(k, _, _)| *k == c.as_str())
                        .map(|(_, col, txt)| (*col, *txt))
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
        "duration" => rsx! { span { if !compact { "⏱ " } "1h23m" } },
        "vim" => rsx! { span { style: "color:{purple}", if !compact { " " } "NORMAL" } },
        "agent" => rsx! { span { style: "color:{orange}", if !compact { "❯ " } "worker-1" } },
        "session" => rsx! { span { style: "color:{dim}", "a3f9" } },
        _ => {
            // Run plugin to get live preview
            let input = serde_json::json!({
                "data": {},
                "config": { "compact": compact, "components": components }
            });
            match crate::plugin::run_plugin(name, &input.to_string()) {
                Some(text) => {
                    let html = format!("⚙ {}", crate::theme::ansi_to_html(&text));
                    rsx! { span { style: "color:{orange}", dangerous_inner_html: "{html}" } }
                }
                None => rsx! { span { style: "color:{orange}", "⚙ {name}" } },
            }
        }
    }
}

fn preview_line(widgets: &[String], config: &StatuslineConfig) -> Element {
    let dim_s = crate::theme::ansi_256_to_hex(config.theme.dim.unwrap_or(242));
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
fn App() -> Element {
    let config_init = StatuslineConfig::load().unwrap_or_default();
    let mut config = use_signal(|| config_init);
    let mut active_tab = use_signal(|| Tab::Lines);
    let plugin_metas = use_signal(plugin::list_plugin_metas);

    let window = dioxus::desktop::use_window();
    let window_close = window.clone();

    let tab = *active_tab.read();
    let cfg_snap = config.read().clone();

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

            // Title bar
            div {
                style: "height:32px; flex-shrink:0; cursor:grab; background:#181825; border-bottom:1px solid #313244; display:flex; align-items:center; justify-content:flex-end; padding:0 10px; user-select:none; position:relative;",
                onmousedown: move |_| window.drag(),
                span { style: "font-size:12px; color:#cdd6f4; letter-spacing:0.03em; position:absolute; left:50%; transform:translateX(-50%);", "Soffit" }
                button {
                    style: "background:none; border:none; color:#6c7086; font-size:16px; cursor:pointer; padding:2px 6px; line-height:1;",
                    onmousedown: move |evt: MouseEvent| evt.stop_propagation(),
                    onclick: move |_| window_close.close(),
                    "×"
                }
            }

            // Preview
            div {
                style: "padding:12px 16px; flex-shrink:0; border-bottom:1px solid #313244;",
                div { style: "font-size:10px; color:#6c7086; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:6px;", "Preview" }
                div {
                    style: "background:#11111b; border:1px solid #313244; border-radius:6px; padding:10px 14px; font-family:'JetBrains Mono',Menlo,Consolas,monospace; font-size:13px; line-height:1.6;",
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
            }

            // Tab content (only scrolling area)
            div {
                style: "flex:1; min-height:0; overflow-y:auto; padding:16px;",
                match tab {
                    Tab::Lines => rsx! { LinesTab { config, plugin_metas } },
                    Tab::Widgets => rsx! { WidgetsTab { config, plugin_metas } },
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
                    option { value: "{p.name}", "⚙ {p.name} — {p.description}" }
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
            for w in WIDGETS.iter().filter(|w| !w.default_components.is_empty() || w.has_compact) {
                WidgetAccordion {
                    config,
                    widget_name: w.name.to_string(),
                    all_components: w.default_components.iter().map(|s| s.to_string()).collect(),
                    has_compact: w.has_compact,
                }
            }
            div { style: "margin-top:16px;",
                div { style: "display:flex; align-items:center; gap:8px; margin-bottom:8px;",
                    div { style: "font-size:11px; color:#6c7086; text-transform:uppercase; letter-spacing:0.05em;", "Plugins" }
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
                                            format!("#!/usr/bin/env python3\nimport json, sys\ndata = json.load(sys.stdin)\nprint(\"⚙ {name}\")\n"),
                                        )
                                    } else {
                                        (
                                            "sh",
                                            format!("#!/bin/bash\n# Reads Claude Code JSON from stdin, outputs widget text\necho \"⚙ {name}\"\n"),
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
                                    let file = rfd::FileDialog::new()
                                        .set_title("Import plugin binary or script")
                                        .pick_file();
                                    if let Some(path) = file {
                                        match plugin::import_plugin(&path) {
                                            Ok(_) => {
                                                plugin_metas.set(plugin::list_plugin_metas());
                                                show_create_form.set(false);
                                                create_error.set(String::new());
                                            }
                                            Err(e) => create_error.set(format!("Import failed: {e}")),
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
                    PluginAccordion { config, meta: p.clone(), plugin_metas }
                }
            }
        }
    }
}

#[component]
fn PluginAccordion(
    config: Signal<StatuslineConfig>,
    meta: PluginMeta,
    plugin_metas: Signal<Vec<PluginMeta>>,
) -> Element {
    let source_init = plugin::read_plugin_source(&meta.name).unwrap_or_default();
    let mut source = use_signal(|| source_init);
    let mut preview_result: Signal<Option<plugin::PluginOutput>> = use_signal(|| None);
    let mut editing = use_signal(|| false);
    let mut renaming = use_signal(|| false);
    let mut rename_value = use_signal(|| meta.name.clone());
    let mut rename_error = use_signal(String::new);

    let widget_name = meta.name.clone();
    let all_components = meta.components.clone();
    let has_compact = meta.has_compact;
    let has_components = !all_components.is_empty();

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

    let compact_btn_style = if compact {
        "background:#89b4fa; color:#1e1e2e; border:none; border-radius:3px; padding:1px 7px; font-size:11px; cursor:pointer; font-weight:bold;"
    } else {
        "background:#313244; color:#6c7086; border:1px solid #45475a; border-radius:3px; padding:1px 7px; font-size:11px; cursor:pointer;"
    };

    let wn = widget_name.clone();
    let wn2 = widget_name.clone();
    let ac = all_components.clone();
    let name_save = widget_name.clone();
    let name_run = widget_name.clone();
    let name_delete = widget_name.clone();
    let name_rename = widget_name.clone();
    let meta_for_rename = meta.clone();

    rsx! {
        details {
            style: "border:1px solid #313244; border-radius:6px; margin-bottom:8px; background:#181825;",
            summary {
                style: "display:flex; align-items:center; gap:10px; padding:8px 12px; cursor:pointer; list-style:none; user-select:none;",
                span { style: "color:#fab387;", "⚙" }
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
            div { style: "padding:8px 12px 12px; display:flex; flex-direction:column; gap:10px;",
                // Component chips (same as WidgetAccordion)
                if has_components {
                    div {
                        div { style: "font-size:11px; color:#6c7086; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:6px;", "Components" }
                        div {
                            style: "display:flex; flex-wrap:wrap; gap:6px; align-items:center;",
                            for (idx, comp) in effective_comps.iter().cloned().enumerate() {
                                CompChip { config, widget_name: wn2.clone(), all_components: ac.clone(), comp_name: comp, comp_idx: idx }
                            }
                            // Drop zone at end of component list
                            div { class: "dnd-drop-zone", "data-comp-drop": "{wn2}" }
                            {
                                let hidden: Vec<String> = all_components
                                    .iter()
                                    .filter(|c| !effective_comps.contains(c))
                                    .cloned()
                                    .collect();
                                let wn_add = wn2.clone();
                                let ac_add = all_components.clone();
                                if !hidden.is_empty() {
                                    rsx! {
                                        select {
                                            style: "background:#313244; color:#6c7086; border:1px solid #45475a; border-radius:4px; padding:2px 6px; font-size:12px; cursor:pointer;",
                                            onchange: move |evt: Event<FormData>| {
                                                let comp = evt.value();
                                                if !comp.is_empty() {
                                                    {
                                                        let mut cfg = config.write();
                                                        let wc = cfg.widgets.entry(wn_add.clone()).or_default();
                                                        if wc.components.is_empty() {
                                                            wc.components = ac_add.clone();
                                                        }
                                                        if !wc.components.contains(&comp) {
                                                            wc.components.push(comp);
                                                        }
                                                    }
                                                    autosave(&config);
                                                }
                                            },
                                            option { value: "", disabled: true, selected: true, "+" }
                                            for comp in hidden { option { value: "{comp}", "{comp}" } }
                                        }
                                    }
                                } else { rsx! {} }
                            }
                        }
                        if has_custom {
                            {
                                let wn_reset = wn2.clone();
                                rsx! {
                                    button {
                                        style: "background:transparent; border:none; color:#6c7086; font-size:12px; cursor:pointer; margin-top:4px; text-decoration:underline;",
                                        onclick: move |_| {
                                            if let Some(wc) = config.write().widgets.get_mut(&wn_reset) {
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
                // Pencil toggle for deeper editing
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
                        "✏ Edit plugin"
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
                                } else {
                                    plugin_metas.set(plugin::list_plugin_metas());
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
                                            plugin_metas.set(plugin::list_plugin_metas());
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
                                    rename_value.set(meta_for_rename.name.clone());
                                    renaming.set(true);
                                },
                                "Rename"
                            }
                            button {
                                style: "background:#f38ba8; color:#1e1e2e; border:none; border-radius:4px; padding:4px 10px; font-size:12px; cursor:pointer; font-weight:bold;",
                                onclick: move |_| {
                                    if let Err(e) = plugin::delete_plugin(&name_delete) {
                                        eprintln!("delete_plugin failed: {e}");
                                    } else {
                                        plugin_metas.set(plugin::list_plugin_metas());
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

#[component]
fn WidgetAccordion(
    config: Signal<StatuslineConfig>,
    widget_name: String,
    all_components: Vec<String>,
    has_compact: bool,
) -> Element {
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

    let wn = widget_name.clone();
    let wn2 = widget_name.clone();
    let ac = all_components.clone();

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
            div { style: "padding:8px 12px 12px;",
                if has_components {
                    div {
                        div { style: "font-size:11px; color:#6c7086; text-transform:uppercase; letter-spacing:0.05em; margin-bottom:6px;", "Components" }
                        div {
                            style: "display:flex; flex-wrap:wrap; gap:6px; align-items:center;",
                            for (idx, comp) in effective_comps.iter().cloned().enumerate() {
                                CompChip { config, widget_name: wn2.clone(), all_components: ac.clone(), comp_name: comp, comp_idx: idx }
                            }
                            // Drop zone at end of component list
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
