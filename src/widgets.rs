use std::collections::HashMap;
use std::process::Command;
use std::time::UNIX_EPOCH;

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

use crate::cache;
use crate::colors::*;
use crate::config::StatuslineConfig;
use crate::fmt::*;
use crate::paths;
use crate::types::{InsightCounts, SessionSnapshot, StdinData, WidgetConfig};

pub const AVAILABLE: &[&str] = &[
    "context_bar",
    "cost",
    "version",
    "git",
    "duration",
    "vim",
    "agent",
    "quota",
    "session",
];

pub struct WidgetContext {
    pub data: StdinData,
    pub pct: u32,
    pub input_tokens: u64,
    pub compact_size: Option<u64>,
    pub terminal_width: u16,
}

pub fn build_context(data: StdinData) -> WidgetContext {
    let config_pct = StatuslineConfig::load()
        .map(|c| c.autocompact_pct)
        .unwrap_or(100);
    let autocompact_pct: u32 = std::env::var("CLAUDE_AUTOCOMPACT_PCT_OVERRIDE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(config_pct);

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
    }
}

// --- Component defaults ---

const COMPONENTS_VERSION: &[&str] = &["update", "version", "model"];
const COMPONENTS_CONTEXT_BAR: &[&str] = &["bar", "pct", "tokens"];
const COMPONENTS_COST: &[&str] = &["session", "today", "week"];
const COMPONENTS_GIT: &[&str] = &["branch", "staged", "modified", "repo", "worktree"];
const COMPONENTS_INSIGHTS: &[&str] = &["strategies", "priorities", "insights", "notes", "pending"];
const COMPONENTS_QUOTA: &[&str] = &["five_hour", "seven_day"];

fn active_components<'a>(requested: &'a [String], defaults: &'a [&'a str]) -> Vec<&'a str> {
    if requested.is_empty() {
        defaults.to_vec()
    } else {
        requested.iter().map(|s| s.as_str()).collect()
    }
}

// --- Version widget ---

const UPDATE_ICON: char = '\u{2191}';

use std::sync::LazyLock;
static MODEL_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\s*\((\d+\w*)\s+context\)").unwrap());

fn clean_model_name(raw: &str) -> String {
    MODEL_RE.replace_all(raw, " $1").to_lowercase()
}

fn latest_version() -> Option<String> {
    let path = paths::version_cache();
    let stale = cache::read_stale(path);
    if cache::needs_refresh(path, 3600.0) {
        spawn_version_fetch();
    }
    stale.and_then(|s| {
        let s = s.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    })
}

fn spawn_version_fetch() {
    spawn_bg_fetch(paths::version_lock(), "fetch-version");
}

fn latest_soffit_version() -> Option<String> {
    let path = paths::self_version_cache();
    let stale = cache::read_stale(path);
    if cache::needs_refresh(path, 3600.0) {
        spawn_bg_fetch(paths::self_version_lock(), "fetch-self-version");
    }
    stale.and_then(|s| {
        let s = s.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    })
}

fn spawn_bg_fetch(lock: &str, arg: &str) {
    let lock_path = std::path::Path::new(lock);
    if lock_path.exists() {
        let stale = lock_path
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .map(|age| age.as_secs() > 30)
            .unwrap_or(true);
        if !stale {
            return;
        }
        let _ = std::fs::remove_file(lock_path);
    }
    let _ = std::fs::write(lock_path, "");
    let exe = std::env::current_exe().unwrap_or_default();
    let mut cmd = Command::new(exe);
    cmd.arg(arg);
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    let _ = cmd.spawn();
}

pub fn render_version(ctx: &WidgetContext, compact: bool, components: &[String]) -> Option<String> {
    let version = ctx.data.version.as_deref().filter(|v| !v.is_empty())?;
    let model_raw = ctx
        .data
        .model
        .as_ref()
        .map(|m| m.display_name.as_str())
        .unwrap_or("");
    let model = clean_model_name(model_raw);

    let has_update =
        matches!(latest_version(), Some(latest) if !latest.is_empty() && latest != version);
    let has_self_update = matches!(
        latest_soffit_version(),
        Some(latest) if !latest.is_empty() && latest != env!("CARGO_PKG_VERSION")
    );

    let mut parts: Vec<String> = Vec::new();
    for comp in active_components(components, COMPONENTS_VERSION) {
        match comp {
            "update" if has_update || has_self_update => {
                parts.push(format!("{ORANGE}{UPDATE_ICON} {RESET}"));
            }
            "version" => {
                if compact {
                    parts.push(format!("{DIM}{version}{RESET}"));
                } else {
                    parts.push(format!("{DIM}{}{RESET}", superscript(version)));
                }
            }
            "model" if !model.is_empty() => {
                if compact {
                    parts.push(format!("{PURPLE}{model}{RESET}"));
                } else {
                    parts.push(format!("{PURPLE}{}{RESET}", subscript(&model)));
                }
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(""))
    }
}

// --- Context bar widget ---

pub fn render_context_bar(
    ctx: &WidgetContext,
    compact: bool,
    components: &[String],
) -> Option<String> {
    let ctx_size = ctx.compact_size.or_else(|| {
        ctx.data
            .context_window
            .as_ref()
            .and_then(|c| c.context_window_size)
    });
    let denom = ctx_size.map(fmt_tokens).unwrap_or_else(|| "?".to_string());

    // Responsive bar width: try 12 down to 4
    let bar_width = responsive_bar_width(ctx.terminal_width, 12, 4);
    let (bar, col) = context_bar(ctx.pct, bar_width);

    let mut parts: Vec<String> = Vec::new();
    for comp in active_components(components, COMPONENTS_CONTEXT_BAR) {
        match comp {
            "bar" => parts.push(bar.clone()),
            "pct" => parts.push(seg_pct(ctx.pct, col)),
            "tokens" if !compact => {
                parts.push(format!(
                    "{DIM}{}/{denom}{RESET}",
                    fmt_tokens(ctx.input_tokens)
                ));
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn responsive_bar_width(terminal_width: u16, max_width: usize, min_width: usize) -> usize {
    // Estimate overhead: each bar char is 1 visible col, plus pct (~4 chars) + tokens (~10 chars) + separators
    // We just pick the largest width that fits — approximate: bar + " 🯵🯵٪ 50k/200k " ~= width + 20
    let available = terminal_width as usize;
    let mut w = max_width;
    while w > min_width {
        let estimated = w + 20; // rough overhead for pct + tokens
        if estimated <= available {
            return w;
        }
        w -= 1;
    }
    min_width
}

// --- Duration widget ---

pub fn render_duration(
    ctx: &WidgetContext,
    compact: bool,
    _components: &[String],
) -> Option<String> {
    let ms = ctx.data.cost.as_ref()?.total_duration_ms?;
    if compact {
        Some(format!("{LGRAY}{}{RESET}", fmt_duration(ms)))
    } else {
        Some(format!("{LGRAY}\u{23f1} {}{RESET}", fmt_duration(ms)))
    }
}

// --- Cost widget ---

fn parse_daily_cache(s: &str) -> Option<(f64, f64, f64)> {
    let parts: Vec<f64> = s.trim().split(',').filter_map(|p| p.parse().ok()).collect();
    if parts.len() >= 3 {
        Some((parts[2], parts[0], parts[1]))
    } else {
        None
    }
}

fn color_for_budget(ratio: f64) -> &'static str {
    if ratio >= 1.0 {
        RED
    } else if ratio >= 0.7 {
        ORANGE
    } else {
        GREEN
    }
}

fn spawn_cost_refresh(sid: &str) {
    let lock = paths::cost_lock();
    let lock_path = std::path::Path::new(lock);
    if lock_path.exists() {
        let stale = lock_path
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .map(|age| age.as_secs() > 30)
            .unwrap_or(true);
        if !stale {
            return;
        }
        let _ = std::fs::remove_file(lock_path);
    }
    let _ = std::fs::write(lock_path, "");
    let exe = std::env::current_exe().unwrap_or_default();
    let mut cmd = Command::new(exe);
    cmd.arg("refresh-cost").arg(sid);
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    let _ = cmd.spawn();
}

pub fn render_cost(ctx: &WidgetContext, compact: bool, components: &[String]) -> Option<String> {
    let sid = ctx.data.session_id.as_deref().unwrap_or("");
    let daily_path = paths::cost_daily();
    let session_path = paths::cost_session(sid);

    let daily_parsed = cache::read_stale(daily_path).and_then(|s| parse_daily_cache(&s));
    if daily_parsed.is_none() || cache::needs_refresh(daily_path, 60.0) {
        spawn_cost_refresh(sid);
    }
    let Some((today_usd, week_usd, target)) = daily_parsed else {
        return Some(format!("\u{1f4b8} {DIM}--{RESET}"));
    };

    // Session cost: prefer direct stdin value, fall back to cache
    let session_cost = ctx
        .data
        .cost
        .as_ref()
        .and_then(|c| c.total_cost_usd)
        .filter(|&c| c > 0.0)
        .or_else(|| {
            if sid.is_empty() {
                None
            } else {
                cache::read_stale(&session_path).and_then(|s| s.trim().parse::<f64>().ok())
            }
        });

    let daily_pace = if target > 0.0 {
        target / 7.0
    } else {
        300.0 / 7.0
    };
    let today_col = color_for_budget(today_usd / daily_pace);
    let week_col = color_for_budget(week_usd / target.max(1.0));

    let active = active_components(components, COMPONENTS_COST);
    let mut parts: Vec<String> = Vec::new();
    for comp in &active {
        match *comp {
            "session" => {
                if let Some(c) = session_cost {
                    parts.push(format!("{DIM}{}{RESET}", fmt_cost(c)));
                }
            }
            "today" => parts.push(format!("{today_col}{}{RESET}", fmt_cost(today_usd))),
            "week" => parts.push(format!("{week_col}{}{RESET}", fmt_cost(week_usd))),
            _ => {}
        }
    }

    if parts.is_empty() {
        return None;
    }

    let sep = if compact { " " } else { " | " };
    let body = parts.join(sep);
    if compact {
        Some(body)
    } else {
        Some(format!("\u{1f4b8} {body}"))
    }
}

// --- Git widget ---

fn git_run(args: &[&str], cwd: &str) -> Option<String> {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn simple_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn truncate_worktree_name(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= 6 {
        name.to_string()
    } else {
        format!(
            "{}..{}",
            chars[..2].iter().collect::<String>(),
            chars[chars.len() - 2..].iter().collect::<String>()
        )
    }
}

fn widget_git(cwd: Option<&str>, compact: bool, components: &[String]) -> Option<String> {
    let cwd = match cwd {
        Some(d) if !d.is_empty() => d.to_string(),
        _ => std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    };

    if cwd.is_empty() {
        return None;
    }

    // Cache git info for 5s — key includes compact flag and component list
    let mut cache_key = cwd.to_string();
    cache_key.push_str(if compact { ":c" } else { ":v" });
    for c in components {
        cache_key.push_str(c);
        cache_key.push(',');
    }
    let hash = simple_hash(&cache_key);
    let cache_path = paths::git_cache(&hash);
    if !cache::needs_refresh(&cache_path, 5.0) {
        if let Some(cached) = cache::read_stale(&cache_path) {
            return if cached.is_empty() {
                None
            } else {
                Some(cached)
            };
        }
    }

    let result = compute_git_segment(&cwd, compact, components);

    // Write to cache (empty string means "not a git dir")
    let cached_val = result.as_deref().unwrap_or("");
    cache::write_cache(&cache_path, cached_val);

    result
}

fn compute_git_segment(cwd: &str, compact: bool, components: &[String]) -> Option<String> {
    let branch = git_run(&["branch", "--show-current"], cwd)
        .or_else(|| {
            git_run(&["rev-parse", "--short", "HEAD"], cwd).map(|h| h.chars().take(7).collect())
        })
        .unwrap_or_default();

    if branch.is_empty() {
        return None;
    }

    let status_out = git_run(&["status", "--porcelain"], cwd).unwrap_or_default();
    let mut staged = 0u32;
    let mut modified = 0u32;
    for line in status_out.lines() {
        if line.len() < 2 {
            continue;
        }
        let xy = line.as_bytes();
        if matches!(xy[0], b'A' | b'M' | b'D' | b'R' | b'C') {
            staged += 1;
        }
        if matches!(xy[1], b'M' | b'D') {
            modified += 1;
        }
    }

    let repo_url_and_name = git_run(&["remote", "get-url", "origin"], cwd).map(|remote| {
        let url = if remote.starts_with("git@") {
            remote.replacen(':', "/", 1).replacen("git@", "https://", 1)
        } else {
            remote
        };
        let url = url.trim_end_matches(".git").to_string();
        let name = url.rsplit('/').next().unwrap_or(&url).to_string();
        (url, name)
    });
    let dir_name = std::path::Path::new(cwd)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    // Worktree detection
    let worktree_name: Option<String> = {
        let wt_out = git_run(&["worktree", "list"], cwd).unwrap_or_default();
        let wt_count = wt_out.lines().count();
        if wt_count > 1 {
            git_run(&["rev-parse", "--show-toplevel"], cwd)
                .map(|top| {
                    std::path::Path::new(&top)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                })
                .map(|name| truncate_worktree_name(&name))
        } else {
            None
        }
    };

    let mut parts: Vec<String> = Vec::new();
    for comp in active_components(components, COMPONENTS_GIT) {
        match comp {
            "branch" => parts.push(format!("{LGRAY}\u{2387} {branch}{RESET}")),
            "staged" if staged > 0 && !compact => {
                parts.push(format!("{GREEN}\u{2022}{staged}{RESET}"));
            }
            "modified" if modified > 0 && !compact => {
                parts.push(format!("{ORANGE}+{modified}{RESET}"));
            }
            "repo" if !compact => {
                if let Some((url, name)) = &repo_url_and_name {
                    parts.push(format!("\x1b]8;;{url}\x07{CYAN}{name}{RESET}\x1b]8;;\x07"));
                } else if !dir_name.is_empty() {
                    parts.push(format!("{CYAN}{dir_name}{RESET}"));
                }
            }
            "worktree" => {
                if let Some(wt) = &worktree_name {
                    parts.push(format!("{ITALIC}{DIM_PINK}{wt}{NO_ITALIC}{RESET}"));
                }
            }
            _ => {}
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

// --- Insights widget ---

pub fn render_insights(
    _ctx: &WidgetContext,
    compact: bool,
    components: &[String],
) -> Option<String> {
    let insights_path =
        dirs::home_dir()?.join(".local/share/jarvis/insights/pending-insights.json");
    let strategies_path =
        dirs::home_dir()?.join(".local/share/jarvis/strategies/active-strategies.json");

    let mut strategies_n = 0usize;
    let mut priorities_n = 0usize;
    let mut insights_n = 0usize;
    let mut notes_n = 0usize;
    let mut pending_n = 0usize;

    if let Ok(raw) = std::fs::read_to_string(&strategies_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            strategies_n = v.as_array().map(|a| a.len()).unwrap_or(0);
        }
    }

    if let Ok(raw) = std::fs::read_to_string(&insights_path) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(arr) = parsed.as_array() {
                let c = InsightCounts::from_json(arr);
                priorities_n = c.red as usize;
                insights_n = c.orange as usize;
                notes_n = c.green as usize;
                pending_n = c.pending_actions as usize;
            }
        }
    }

    let mut parts: Vec<String> = Vec::new();
    for comp in active_components(components, COMPONENTS_INSIGHTS) {
        match comp {
            "strategies" if strategies_n > 0 => {
                if compact {
                    parts.push(format!("{PURPLE}\u{1f52d}{strategies_n}{RESET}"));
                } else {
                    let label = if strategies_n == 1 {
                        "strategy"
                    } else {
                        "strategies"
                    };
                    parts.push(format!("{PURPLE}\u{1f52d} {strategies_n} {label}{RESET}"));
                }
            }
            "priorities" if priorities_n > 0 => {
                if compact {
                    parts.push(format!("{RED}\u{1f3af}{priorities_n}{RESET}"));
                } else {
                    let label = if priorities_n == 1 {
                        "priority"
                    } else {
                        "priorities"
                    };
                    parts.push(format!("{RED}\u{1f3af} {priorities_n} {label}{RESET}"));
                }
            }
            "insights" if insights_n > 0 => {
                if compact {
                    parts.push(format!("{ORANGE}\u{1f4a1}{insights_n}{RESET}"));
                } else {
                    let label = if insights_n == 1 {
                        "insight"
                    } else {
                        "insights"
                    };
                    parts.push(format!("{ORANGE}\u{1f4a1} {insights_n} {label}{RESET}"));
                }
            }
            "notes" if notes_n > 0 => {
                if compact {
                    parts.push(format!("{GREEN}\u{1f4cb}{notes_n}{RESET}"));
                } else {
                    let label = if notes_n == 1 { "note" } else { "notes" };
                    parts.push(format!("{GREEN}\u{1f4cb} {notes_n} {label}{RESET}"));
                }
            }
            "pending" if pending_n > 0 => {
                if compact {
                    parts.push(format!("{YELLOW}\u{23f3}{pending_n}{RESET}"));
                } else {
                    parts.push(format!("{YELLOW}\u{23f3} {pending_n} pending{RESET}"));
                }
            }
            _ => {}
        }
    }

    if parts.is_empty() {
        return None;
    }

    let sep = if compact { " " } else { " | " };
    let body = parts.join(sep);
    if compact {
        Some(body)
    } else {
        Some(format!("{body} /brief"))
    }
}

// --- Vim widget ---

pub fn render_vim(ctx: &WidgetContext, _compact: bool, _components: &[String]) -> Option<String> {
    let mode = ctx
        .data
        .vim
        .as_ref()
        .map(|v| v.mode.as_str())
        .filter(|m| !m.is_empty())?;
    Some(format!("{PURPLE}{mode}{RESET}"))
}

// --- Agent widget ---

pub fn render_agent(ctx: &WidgetContext, compact: bool, _components: &[String]) -> Option<String> {
    let name = ctx
        .data
        .agent
        .as_ref()
        .map(|a| a.name.as_str())
        .filter(|n| !n.is_empty())?;
    if compact {
        Some(format!("{ORANGE}{name}{RESET}"))
    } else {
        Some(format!("{ORANGE}\u{276f} {name}{RESET}"))
    }
}

// --- Quota widget ---

const FIVE_HOURS: f64 = 5.0 * 3600.0;
const SEVEN_DAYS: f64 = 7.0 * 24.0 * 3600.0;

pub fn render_quota(ctx: &WidgetContext, _compact: bool, components: &[String]) -> Option<String> {
    let rate_limits = ctx.data.rate_limits.as_ref()?;

    let bar_width = responsive_bar_width(ctx.terminal_width, 12, 4);
    let active = active_components(components, COMPONENTS_QUOTA);
    let mut segments: Vec<String> = Vec::new();

    for comp in &active {
        match *comp {
            "five_hour" => {
                if let Some(rl) = &rate_limits.five_hour {
                    if let Some(seg) = render_quota_window(rl, bar_width, FIVE_HOURS, "5h", false) {
                        segments.push(seg);
                    }
                }
            }
            "seven_day" => {
                if let Some(rl) = &rate_limits.seven_day {
                    if let Some(seg) = render_quota_window(rl, bar_width, SEVEN_DAYS, "7d", true) {
                        segments.push(seg);
                    }
                }
            }
            _ => {}
        }
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments.join(&format!(" {DIM}|{RESET} ")))
    }
}

fn render_quota_window(
    rl: &crate::types::RateLimit,
    bar_width: usize,
    window_secs: f64,
    label: &str,
    show_pace: bool,
) -> Option<String> {
    let used = rl.used_percentage?;
    let remaining = (100.0 - used).max(0.0) as u32;

    let (reset_str, remaining_secs) = rl
        .resets_at
        .as_ref()
        .map(fmt_reset)
        .unwrap_or_else(|| ("".to_string(), 0.0));

    let pace_pct = if remaining_secs > 0.0 {
        Some(remaining_secs / window_secs * 100.0)
    } else {
        None
    };

    let col = quota_color(used, remaining_secs, window_secs);
    let (bar, _) = usage_bar(remaining, bar_width, col, pace_pct);
    let pct_str = seg_pct(remaining, col);

    let pace_part = if show_pace {
        pace_balance_secs(used, remaining_secs, window_secs)
            .map(|bal| format!(" {}", fmt_pace(bal, window_secs as u64)))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let reset_part = if reset_str.is_empty() {
        String::new()
    } else {
        format!(" {DIM}{reset_str}{RESET}")
    };

    Some(format!(
        "{DIM}{label}:{RESET} {bar} {pct_str}{pace_part}{reset_part}"
    ))
}

// --- Session widget ---

pub fn render_session(
    ctx: &WidgetContext,
    _compact: bool,
    _components: &[String],
) -> Option<String> {
    let sid = ctx.data.session_id.as_deref().filter(|s| !s.is_empty())?;

    // Load or refresh sid list (TTL 30s)
    let cache_path = paths::sid_cache();
    let all_sids: Vec<String> = if !cache::needs_refresh(cache_path, 30.0) {
        cache::read_stale(cache_path)
            .map(|s| s.lines().map(String::from).collect())
            .unwrap_or_default()
    } else {
        let sids = collect_session_ids();
        let joined = sids.join("\n");
        cache::write_cache(cache_path, &joined);
        sids
    };

    let prefix = shortest_unique_prefix(sid, &all_sids);
    Some(format!("{DIM}{prefix}{RESET}"))
}

fn collect_session_ids() -> Vec<String> {
    let projects_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude/projects"),
        None => return vec![],
    };

    walkdir::WalkDir::new(&projects_dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().map(|x| x == "jsonl").unwrap_or(false)
        })
        .filter_map(|e| {
            e.path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .collect()
}

fn shortest_unique_prefix(sid: &str, all: &[String]) -> String {
    let others: Vec<&str> = all
        .iter()
        .filter(|s| s.as_str() != sid)
        .map(|s| s.as_str())
        .collect();
    for len in 3..=sid.len() {
        let prefix = &sid[..len];
        let conflict = others.iter().any(|o| o.starts_with(prefix));
        if !conflict {
            return prefix.to_string();
        }
    }
    sid.to_string()
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
    match name {
        "version" | "model" => render_version(ctx, compact, components),
        "context_bar" => render_context_bar(ctx, compact, components),
        "duration" => render_duration(ctx, compact, components),
        "cost" => render_cost(ctx, compact, components),
        "git" => widget_git(
            ctx.data
                .workspace
                .as_ref()
                .and_then(|w| w.current_dir.as_deref()),
            compact,
            components,
        ),
        "insights" => render_insights(ctx, compact, components),
        "vim" => render_vim(ctx, compact, components),
        "agent" => render_agent(ctx, compact, components),
        "quota" => render_quota(ctx, compact, components),
        "session" => render_session(ctx, compact, components),
        other => {
            let plugin_input = serde_json::json!({
                "data": ctx.data,
                "config": {
                    "compact": compact,
                    "components": components,
                }
            });
            crate::plugin::run_plugin(other, &plugin_input.to_string())
        }
    }
}

pub fn render_line_parts(
    widget_names: &[String],
    ctx: &WidgetContext,
    widget_configs: &HashMap<String, WidgetConfig>,
) -> Vec<String> {
    widget_names
        .iter()
        .filter_map(|name| dispatch_widget(name, ctx, widget_configs))
        .collect()
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

    let plugins = crate::plugin::list_plugins();
    if name == "list" {
        for w in AVAILABLE {
            println!("{w}");
        }
        for p in &plugins {
            println!("{p} [plugin]");
        }
        return Ok(());
    }
    if !AVAILABLE.contains(&name) && !plugins.iter().any(|p| p == name) {
        bail!(
            "unknown widget '{name}'. Available: {}",
            AVAILABLE.join(", ")
        );
    }
    let ctx = build_context(StdinData::default());
    let output = render_line(&[name.to_string()], &ctx, "", &HashMap::new());
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
        let ctx = build_context(data);
        // Override terminal_width for deterministic tests
        WidgetContext {
            terminal_width: 120,
            ..ctx
        }
    }

    #[test]
    fn available_list_contains_all() {
        for w in AVAILABLE {
            assert!(AVAILABLE.contains(w), "missing: {w}");
        }
        assert!(AVAILABLE.contains(&"quota"));
        assert!(AVAILABLE.contains(&"session"));
    }

    #[test]
    fn context_bar_uses_gradient_chars() {
        let data = StdinData {
            context_window: Some(ContextWindow {
                used_percentage: Some(50.0),
                context_window_size: Some(200_000),
                current_usage: Some(CurrentUsage {
                    input_tokens: Some(100_000),
                    ..Default::default()
                }),
            }),
            ..Default::default()
        };
        let ctx = make_ctx(data);
        let bar = render_context_bar(&ctx, false, &[]).unwrap();
        // Gradient chars: ■, ◧, □ — NOT ●○
        assert!(bar.contains('■') || bar.contains('◧') || bar.contains('□'));
        assert!(!bar.contains('●'));
        assert!(!bar.contains('○'));
    }

    #[test]
    fn render_session_unique_prefix() {
        assert_eq!(
            shortest_unique_prefix("abcdef123", &["xyz000".to_string()]),
            "abc"
        );
        assert_eq!(
            shortest_unique_prefix("abcdef123", &["abcdef456".to_string()]),
            "abcdef1"
        );
        assert_eq!(
            shortest_unique_prefix("abcdef123", &["abcdef1xy".to_string()]),
            "abcdef12"
        );
    }

    #[test]
    fn render_session_prefix_min_3() {
        // Even if unique at 1 char, min is 3
        assert_eq!(
            shortest_unique_prefix("zzzzz", &["aaaaa".to_string()]),
            "zzz"
        );
    }

    #[test]
    fn render_quota_with_mock_data() {
        let data = StdinData {
            rate_limits: Some(RateLimits {
                five_hour: Some(RateLimit {
                    used_percentage: Some(40.0),
                    resets_at: None,
                }),
                seven_day: Some(RateLimit {
                    used_percentage: Some(60.0),
                    resets_at: None,
                }),
            }),
            ..Default::default()
        };
        let ctx = make_ctx(data);
        let result = render_quota(&ctx, false, &[]);
        assert!(result.is_some());
        let s = result.unwrap();
        assert!(s.contains("5h"));
        assert!(s.contains("7d"));
    }

    #[test]
    fn render_quota_none_when_no_rate_limits() {
        let ctx = make_ctx(StdinData::default());
        assert!(render_quota(&ctx, false, &[]).is_none());
    }

    #[test]
    fn git_widget_in_non_git_dir_returns_none() {
        let result = widget_git(Some("/tmp"), false, &[]);
        assert!(result.is_none());
    }

    #[test]
    fn render_line_joins_with_separator() {
        let data = StdinData {
            context_window: Some(ContextWindow {
                used_percentage: Some(50.0),
                context_window_size: Some(200_000),
                current_usage: Some(CurrentUsage {
                    input_tokens: Some(100_000),
                    ..Default::default()
                }),
            }),
            cost: Some(CostInfo {
                total_duration_ms: Some(60_000),
                total_cost_usd: None,
            }),
            ..Default::default()
        };
        let ctx = make_ctx(data);
        let names = vec!["context_bar".into(), "duration".into()];
        let result = render_line(&names, &ctx, " | ", &HashMap::new());
        assert!(result.contains(" | "));
        assert!(result.contains("1m00s"));
    }

    #[test]
    fn render_line_filters_empty_widgets() {
        let ctx = make_ctx(StdinData::default());
        let names = vec!["vim".into(), "agent".into()];
        let result = render_line(&names, &ctx, " | ", &HashMap::new());
        assert!(result.is_empty());
    }

    #[test]
    fn render_version_none_when_no_version() {
        let ctx = make_ctx(StdinData::default());
        assert!(render_version(&ctx, false, &[]).is_none());
    }

    #[test]
    fn render_duration_none_when_no_cost() {
        let ctx = make_ctx(StdinData::default());
        assert!(render_duration(&ctx, false, &[]).is_none());
    }

    #[test]
    fn build_context_defaults_with_empty_stdin() {
        let ctx = build_context(StdinData::default());
        assert_eq!(ctx.pct, 0);
        assert_eq!(ctx.input_tokens, 0);
        assert!(ctx.compact_size.is_none());
    }

    #[test]
    fn clean_model_name_strips_context() {
        let result = clean_model_name("claude-sonnet-4-5 (200k context)");
        assert!(!result.contains("context"));
        assert!(result.contains("200k"));
    }

    #[test]
    fn truncate_worktree_name_short() {
        assert_eq!(truncate_worktree_name("abc"), "abc");
        assert_eq!(truncate_worktree_name("abcdef"), "abcdef");
    }

    #[test]
    fn truncate_worktree_name_long() {
        let result = truncate_worktree_name("abcdefghij");
        assert_eq!(result, "ab..ij");
    }
}
