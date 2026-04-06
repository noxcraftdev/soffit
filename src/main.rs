//! Customizable statusline manager for Claude Code with widget system and desktop editor.
mod cache;
mod config;
mod edit;
mod fmt;
mod http;
mod install;
mod marketplace;
mod paths;
mod plugin;
mod render;
mod setup;
mod theme;
mod types;
mod update;
mod widgets;

#[derive(clap::Parser)]
#[command(
    name = "soffit",
    version,
    about = "Customizable statusline manager for Claude Code"
)]
enum Cli {
    /// Render the statusline (reads JSON from stdin)
    Render,
    /// Open the config editor (native desktop GUI)
    #[cfg(feature = "desktop")]
    Edit,
    /// List available widgets
    Widgets,
    /// Render a single widget for testing
    Widget {
        /// Widget name to render
        name: String,
    },
    /// Fetch latest Claude Code version from npm (hidden, used internally)
    #[command(hide = true)]
    FetchVersion,
    /// Refresh cost cache from JSONL files (hidden, used internally)
    #[command(hide = true)]
    RefreshCost {
        /// Session ID
        sid: String,
    },
    /// Install a community widget from GitHub (owner/repo or owner/repo/name)
    Install {
        /// GitHub source: owner/repo or owner/repo/widget-name
        source: String,
        /// Overwrite if already installed
        #[arg(long)]
        force: bool,
    },
    /// Uninstall a widget by name
    Uninstall {
        /// Widget name to remove
        name: String,
    },
    /// Manage widget marketplace sources
    Marketplace {
        #[command(subcommand)]
        cmd: marketplace::MarketplaceCmd,
    },
    /// Update soffit to the latest version
    Update,
    /// Configure Claude Code to use soffit as the statusline
    Setup,
    /// Fetch latest soffit version from GitHub (hidden, used internally)
    #[command(hide = true)]
    FetchSelfVersion,
}

fn main() -> anyhow::Result<()> {
    use clap::Parser;
    let cli = Cli::parse();
    match cli {
        Cli::Render => render::run(),
        #[cfg(feature = "desktop")]
        Cli::Edit => edit::run(),
        Cli::Widgets => {
            for w in widgets::AVAILABLE {
                println!("{w}");
            }
            for p in plugin::list_custom_widgets() {
                println!("{p}");
            }
            Ok(())
        }
        Cli::Widget { name } => widgets::render(&name),
        Cli::FetchVersion => fetch_version(),
        Cli::RefreshCost { sid } => refresh_cost(&sid),
        Cli::Install { source, force } => install::run(&source, force),
        Cli::Uninstall { name } => plugin::delete_widget(&name),
        Cli::Marketplace { cmd } => marketplace::run(cmd),
        Cli::Update => update::run(),
        Cli::FetchSelfVersion => fetch_self_version(),
        Cli::Setup => setup::run(),
    }
}

fn fetch_self_version() -> anyhow::Result<()> {
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "5",
            "https://api.github.com/repos/noxcraftdev/soffit/releases/latest",
        ])
        .output()?;
    if output.status.success() {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if let Some(tag) = v.get("tag_name").and_then(|v| v.as_str()) {
                let ver = tag.strip_prefix('v').unwrap_or(tag);
                cache::write_cache(paths::self_version_cache(), ver);
            }
        }
    }
    let _ = std::fs::remove_file(paths::self_version_lock());
    Ok(())
}

fn fetch_version() -> anyhow::Result<()> {
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "3",
            "https://registry.npmjs.org/@anthropic-ai/claude-code/latest",
        ])
        .output()?;
    if output.status.success() {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if let Some(ver) = v.get("version").and_then(|v| v.as_str()) {
                cache::write_cache(paths::version_cache(), ver);
            }
        }
    }
    let _ = std::fs::remove_file(paths::version_lock());
    Ok(())
}

fn refresh_cost(sid: &str) -> anyhow::Result<()> {
    use std::collections::HashMap;

    let claude_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("home directory not found"))?
        .join(".claude/projects");
    let now = std::time::SystemTime::now();
    let week_ago = now - std::time::Duration::from_secs(7 * 24 * 3600);
    let today_prefix = chrono_today_prefix();

    // msg_id -> (cost, is_today, is_session)
    let mut seen: HashMap<String, (f64, bool, bool)> = HashMap::new();

    for entry in walkdir::WalkDir::new(&claude_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        if let Ok(meta) = path.metadata() {
            if let Ok(mtime) = meta.modified() {
                if mtime < week_ago {
                    continue;
                }
            }
        }

        let is_session_file = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s == sid)
            .unwrap_or(false);

        let contents = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for line in contents.lines() {
            let v: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
                continue;
            }
            let msg = match v.get("message") {
                Some(m) => m,
                None => continue,
            };
            let msg_id = match msg.get("id").and_then(|i| i.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };
            let usage = match msg.get("usage") {
                Some(u) => u,
                None => continue,
            };
            let model = msg.get("model").and_then(|m| m.as_str()).unwrap_or("");

            let input = usage
                .get("input_tokens")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let output = usage
                .get("output_tokens")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let cache_write = usage
                .get("cache_creation_input_tokens")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let cache_read = usage
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let (in_price, out_price, cw_price, cr_price) = if model.contains("opus") {
                (5.0, 25.0, 6.25, 0.50)
            } else if model.contains("haiku") {
                (1.0, 5.0, 1.25, 0.10)
            } else {
                (3.0, 15.0, 3.75, 0.30)
            };

            let cost = (input * in_price
                + output * out_price
                + cache_write * cw_price
                + cache_read * cr_price)
                / 1_000_000.0;

            let ts = v.get("timestamp").and_then(|t| t.as_str()).unwrap_or("");
            let is_today = ts.starts_with(&today_prefix);

            let entry = seen.entry(msg_id).or_insert((0.0, false, false));
            entry.0 = cost;
            entry.1 |= is_today;
            entry.2 |= is_session_file;
        }
    }

    let mut week_cost = 0.0f64;
    let mut today_cost = 0.0f64;
    let mut session_cost = 0.0f64;

    for (cost, is_today, is_session) in seen.values() {
        week_cost += cost;
        if *is_today {
            today_cost += cost;
        }
        if *is_session {
            session_cost += cost;
        }
    }

    let target = crate::config::StatuslineConfig::load()
        .ok()
        .and_then(|c| c.weekly_budget)
        .unwrap_or(300.0);

    cache::write_cache(
        paths::cost_daily(),
        &format!("{week_cost},{target},{today_cost}"),
    );
    if !sid.is_empty() {
        cache::write_cache(&paths::cost_session(sid), &format!("{session_cost}"));
    }
    let _ = std::fs::remove_file(paths::cost_lock());
    Ok(())
}

fn chrono_today_prefix() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let (y, m, d) = days_to_ymd(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
