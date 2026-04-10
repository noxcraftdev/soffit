//! Customizable statusline manager for Claude Code with widget system and desktop editor.
mod cache;
mod config;
mod edit;
mod fmt;
mod http;
mod install;
mod marketplace;
mod paths;
mod presets;
mod render;
mod setup;
mod theme;
mod types;
mod update;
mod widget;
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
    /// Apply or list widget presets
    Preset {
        #[command(subcommand)]
        cmd: presets::PresetCmd,
    },
    /// Configure Claude Code to use soffit as the statusline
    Setup,
    /// Fetch latest soffit version from GitHub (hidden, used internally)
    #[command(hide = true)]
    FetchSelfVersion,
    /// Install default widgets from registry (hidden, used internally)
    #[command(hide = true)]
    InstallDefaults,
}

fn main() -> anyhow::Result<()> {
    use clap::Parser;
    let cli = Cli::parse();
    match cli {
        Cli::Render => render::run(),
        #[cfg(feature = "desktop")]
        Cli::Edit => edit::run(),
        Cli::Widgets => {
            for p in widget::list_custom_widgets() {
                println!("{p}");
            }
            Ok(())
        }
        Cli::Widget { name } => widgets::render(&name),
        Cli::FetchVersion => fetch_version(),
        Cli::Install { source, force } => install::run(&source, force),
        Cli::Uninstall { name } => widget::delete_widget(&name),
        Cli::Marketplace { cmd } => marketplace::run(cmd),
        Cli::Preset { cmd } => presets::run(cmd),
        Cli::Update => update::run(),
        Cli::FetchSelfVersion => fetch_self_version(),
        Cli::InstallDefaults => marketplace::install_defaults(),
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
